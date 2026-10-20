use std::collections::BTreeSet;

use shakmaty::Chess;
use shakmaty::File;
use shakmaty::Position;
use shakmaty::Square;
use tangibles_chess_core::CommitMode;

use crate::BoardGeometry;
use crate::FieldEvent;
use crate::FieldEventKind;
use crate::InferenceOutput;
use crate::TableEdge;
use crate::ZoneAction;

/// Turns debounced occupancy events into chess move intents.
///
/// The protocol a moving player follows physically: lift own piece, lift
/// a captured opponent piece off (if any), place own piece on the target
/// square. Castling places the king first; the rook's physical move is
/// swallowed without an anomaly. `CommitMode::OnConfirmZone` buffers the
/// intent until the player's confirm zone reports a marker.
#[derive(Debug, Clone)]
pub struct MoveInference {
    /// Field↔square permutation.
    geometry: BoardGeometry,
    /// When intents are committed.
    commit_mode: CommitMode,
    /// Origin square of the piece currently in the air.
    in_air: Option<Square>,
    /// Opponent squares cleared while a capture is in flight.
    lifted_opponents: BTreeSet<Square>,
    /// Target square where a marker landed on an opponent that was not
    /// lifted yet (stacked capture - completes on the opponent lift).
    pending_capture: Option<Square>,
    /// Rook origin expected to move after a castle intent.
    pending_castle_rook: Option<(Square, Square)>,
    /// Intent waiting for a confirm-zone press (`OnConfirmZone`).
    pending_confirm: Option<(Square, Square)>,
}

impl MoveInference {
    /// An inference over the given geometry and commit mode.
    pub fn new(geometry: BoardGeometry, commit_mode: CommitMode) -> Self {
        Self {
            geometry,
            commit_mode,
            in_air: None,
            lifted_opponents: BTreeSet::new(),
            pending_capture: None,
            pending_castle_rook: None,
            pending_confirm: None,
        }
    }

    /// Resets the in-flight state (new game, anomaly recovery).
    pub fn reset(&mut self) {
        self.in_air = None;
        self.lifted_opponents.clear();
        self.pending_capture = None;
        self.pending_castle_rook = None;
        self.pending_confirm = None;
    }

    /// Handles one field event against the current position.
    pub fn on_event(&mut self, event: &FieldEvent, position: &Chess) -> Vec<InferenceOutput> {
        if event.kind == FieldEventKind::Lagged {
            return vec![InferenceOutput::Lagged];
        }
        let Some(field) = event.field.as_deref() else {
            return Vec::new();
        };
        if let Some(index) = BoardGeometry::field_index(field) {
            let Some(square) = self.geometry.field_to_square(index) else {
                return vec![Self::anomaly("field index without square")];
            };
            match event.kind {
                FieldEventKind::FieldLifted => self.on_lift(square, position),
                FieldEventKind::FieldPlaced => self.on_place(square, position),
                FieldEventKind::FieldCountChanged => self.on_count(square, event.count.unwrap_or(0)),
                FieldEventKind::Lagged => Vec::new(),
            }
        } else {
            self.on_zone(field, event)
        }
    }

    /// A marker left a board square.
    fn on_lift(&mut self, square: Square, position: &Chess) -> Vec<InferenceOutput> {
        // Completes a stacked capture: the opponent's marker was removed
        // from under the attacker's marker that already sits on the
        // square.
        if self.pending_capture == Some(square) {
            self.pending_capture = None;
            let Some(from) = self.in_air.take() else {
                return vec![Self::anomaly("capture completion without a piece in the air")];
            };
            self.lifted_opponents.clear();
            return vec![self.commit(from, square, position)];
        }
        // A rook lift following a castle intent is the physical rook move.
        if let Some((rook_from, _rook_to)) = self.pending_castle_rook
            && square == rook_from
        {
            return Vec::new();
        }
        let Some(piece) = position.board().piece_at(square) else {
            return vec![Self::anomaly("lift on an empty square")];
        };
        if self.in_air.is_some() {
            // While a piece is in the air, lifting an opponent marker is
            // the capture preparation; lifting another own marker is not
            // part of the protocol.
            if piece.color != position.turn() {
                self.lifted_opponents.insert(square);
                return Vec::new();
            }
            return vec![Self::anomaly("two pieces lifted at once")];
        }
        if piece.color != position.turn() {
            return vec![Self::anomaly("piece of the non-moving side lifted")];
        }
        self.in_air = Some(square);
        Vec::new()
    }

    /// A marker touched down on a board square.
    fn on_place(&mut self, square: Square, position: &Chess) -> Vec<InferenceOutput> {
        // Completing the rook's physical move after a castle intent.
        if let Some((_rook_from, rook_to)) = self.pending_castle_rook {
            if square == rook_to {
                self.pending_castle_rook = None;
                return Vec::new();
            }
            return vec![Self::anomaly("rook placed off its castling target")];
        }
        let Some(from) = self.in_air else {
            return vec![Self::anomaly("marker placed without a lift")];
        };
        if square == from {
            self.in_air = None;
            self.lifted_opponents.clear();
            self.pending_capture = None;
            return vec![InferenceOutput::Cancelled];
        }
        match position.board().piece_at(square) {
            None => {
                self.in_air = None;
                self.lifted_opponents.clear();
                vec![self.commit(from, square, position)]
            }
            Some(target) if target.color != position.turn() => {
                // Landing on an opponent square: legal only when the
                // opponent's marker was lifted out of the way first.
                if self.lifted_opponents.contains(&square) {
                    self.in_air = None;
                    self.lifted_opponents.clear();
                    vec![self.commit(from, square, position)]
                } else {
                    self.pending_capture = Some(square);
                    Vec::new()
                }
            }
            Some(_own) => vec![Self::anomaly("marker placed on own piece")],
        }
    }

    /// Contact count changed on a board square; stacking (>1) is an
    /// anomaly worth surfacing.
    fn on_count(&self, square: Square, count: i32) -> Vec<InferenceOutput> {
        if count > 1 {
            return vec![Self::anomaly("multiple contacts stacked on one square")];
        }
        let _ = square;
        Vec::new()
    }

    /// Emits the intent directly or buffers it for the confirm zone.
    fn commit(&mut self, from: Square, to: Square, position: &Chess) -> InferenceOutput {
        // Detect castling: the rook's physical move is expected next.
        if position
            .legal_moves()
            .into_iter()
            .any(|mv| mv.is_castle() && mv.from() == Some(from) && castle_target(&mv) == to)
        {
            self.pending_castle_rook = rook_move_of(from, to);
        }
        if self.commit_mode == CommitMode::OnConfirmZone {
            self.pending_confirm = Some((from, to));
        }
        InferenceOutput::MoveIntent { from, to }
    }

    /// Routes zone fields to player actions.
    fn on_zone(&mut self, field: &str, event: &FieldEvent) -> Vec<InferenceOutput> {
        match field {
            crate::ZONE_BTN_SOUTH | crate::ZONE_BTN_NORTH => self.on_confirm_zone(field, event),
            crate::ZONE_TRAY_SOUTH | crate::ZONE_TRAY_NORTH => self.on_tray(field, event),
            _ => Vec::new(),
        }
    }

    /// A marker was placed on a confirm zone: completes a buffered intent
    /// or surfaces a stray press.
    fn on_confirm_zone(&mut self, field: &str, event: &FieldEvent) -> Vec<InferenceOutput> {
        if event.kind != FieldEventKind::FieldPlaced {
            return Vec::new();
        }
        let edge = if field == crate::ZONE_BTN_SOUTH { TableEdge::South } else { TableEdge::North };
        let mut outputs = vec![InferenceOutput::Zone(ZoneAction::ConfirmMove { edge })];
        if let Some((from, to)) = self.pending_confirm.take() {
            outputs.push(InferenceOutput::MoveIntent { from, to });
        }
        outputs
    }

    /// Tray occupancy changes are reported as captured-material counts.
    fn on_tray(&self, field: &str, event: &FieldEvent) -> Vec<InferenceOutput> {
        let edge = if field == crate::ZONE_TRAY_SOUTH {
            TableEdge::South
        } else {
            TableEdge::North
        };
        match event.kind {
            FieldEventKind::FieldPlaced => vec![InferenceOutput::Zone(ZoneAction::TrayCount { edge, count: 1 })],
            FieldEventKind::FieldLifted => vec![InferenceOutput::Zone(ZoneAction::TrayCount { edge, count: 0 })],
            FieldEventKind::FieldCountChanged => vec![InferenceOutput::Zone(ZoneAction::TrayCount {
                edge,
                count: event.count.unwrap_or(0).max(0) as u32,
            })],
            FieldEventKind::Lagged => Vec::new(),
        }
    }

    /// Uniform anomaly output.
    fn anomaly(detail: &str) -> InferenceOutput {
        InferenceOutput::Anomaly { detail: detail.to_string() }
    }
}

/// The user-facing target of a castle move (king's destination square).
fn castle_target(mv: &shakmaty::Move) -> Square {
    match mv {
        shakmaty::Move::Castle { king, rook } => {
            let file = if rook > king { File::G } else { File::C };
            Square::from_coords(file, king.rank())
        }
        _ => mv.to(),
    }
}

/// The physical rook displacement of a castle (origin, destination).
fn rook_move_of(king_from: Square, king_to: Square) -> Option<(Square, Square)> {
    let rank = king_from.rank();
    let (rook_from, rook_to) = if king_to.file() == File::G {
        (Square::from_coords(File::H, rank), Square::from_coords(File::F, rank))
    } else {
        (Square::from_coords(File::A, rank), Square::from_coords(File::D, rank))
    };
    Some((rook_from, rook_to))
}

#[cfg(test)]
mod tests {
    use shakmaty::CastlingMode;
    use shakmaty::Chess;
    use shakmaty::Square;
    use shakmaty::fen::Fen;
    use tangibles_chess_core::CommitMode;

    use super::MoveInference;
    use crate::BoardGeometry;
    use crate::FieldEvent;
    use crate::FieldEventKind;
    use crate::InferenceOutput;
    use crate::TableEdge;
    use crate::ZoneAction;

    fn inference() -> MoveInference {
        MoveInference::new(BoardGeometry::tabletop(), CommitMode::OnPlace)
    }

    fn event(kind: FieldEventKind, field: &str) -> FieldEvent {
        FieldEvent {
            kind,
            field_set: Some("chess".to_string()),
            field: Some(field.to_string()),
            count: None,
            skipped: None,
        }
    }

    fn field_of(geometry: &BoardGeometry, square: Square) -> String {
        BoardGeometry::field_name(geometry.square_to_field(square)).expect("field")
    }

    #[test]
    fn lift_and_place_yields_move_intent() {
        let geometry = BoardGeometry::tabletop();
        let mut inference = inference();
        let position = Chess::default();
        let out = inference.on_event(&event(FieldEventKind::FieldLifted, &field_of(&geometry, Square::E2)), &position);
        assert!(out.is_empty());
        let out = inference.on_event(&event(FieldEventKind::FieldPlaced, &field_of(&geometry, Square::E4)), &position);
        assert_eq!(
            out,
            vec![InferenceOutput::MoveIntent {
                from: Square::E2,
                to: Square::E4
            }]
        );
    }

    #[test]
    fn returning_home_cancels() {
        let geometry = BoardGeometry::tabletop();
        let mut inference = inference();
        let position = Chess::default();
        inference.on_event(&event(FieldEventKind::FieldLifted, &field_of(&geometry, Square::E2)), &position);
        let out = inference.on_event(&event(FieldEventKind::FieldPlaced, &field_of(&geometry, Square::E2)), &position);
        assert_eq!(out, vec![InferenceOutput::Cancelled]);
    }

    #[test]
    fn capture_needs_the_opponent_lifted() {
        let geometry = BoardGeometry::tabletop();
        let mut inference = inference();
        // White pawn e5 can capture black pawn d5.
        let position = Fen::from_ascii(b"rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2")
            .expect("fen")
            .into_position::<Chess>(CastlingMode::Standard)
            .expect("pos");
        let d5 = field_of(&geometry, Square::D5);
        inference.on_event(&event(FieldEventKind::FieldLifted, &field_of(&geometry, Square::E5)), &position);
        // Proper order: opponent lifted first.
        let out = inference.on_event(&event(FieldEventKind::FieldLifted, &d5), &position);
        assert!(out.is_empty());
        let out = inference.on_event(&event(FieldEventKind::FieldPlaced, &d5), &position);
        assert_eq!(
            out,
            vec![InferenceOutput::MoveIntent {
                from: Square::E5,
                to: Square::D5
            }]
        );
    }

    #[test]
    fn stacked_capture_completes_on_opponent_lift() {
        let geometry = BoardGeometry::tabletop();
        let mut inference = inference();
        let position = Fen::from_ascii(b"rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2")
            .expect("fen")
            .into_position::<Chess>(CastlingMode::Standard)
            .expect("pos");
        let d5 = field_of(&geometry, Square::D5);
        inference.on_event(&event(FieldEventKind::FieldLifted, &field_of(&geometry, Square::E5)), &position);
        // Stacked order: attacker lands before the opponent leaves.
        let out = inference.on_event(&event(FieldEventKind::FieldPlaced, &d5), &position);
        assert!(out.is_empty());
        let out = inference.on_event(&event(FieldEventKind::FieldLifted, &d5), &position);
        assert_eq!(
            out,
            vec![InferenceOutput::MoveIntent {
                from: Square::E5,
                to: Square::D5
            }]
        );
    }

    #[test]
    fn castle_swallows_the_rook_move() {
        let geometry = BoardGeometry::tabletop();
        let mut inference = inference();
        let position = Fen::from_ascii(b"r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1")
            .expect("fen")
            .into_position::<Chess>(CastlingMode::Standard)
            .expect("pos");
        inference.on_event(&event(FieldEventKind::FieldLifted, &field_of(&geometry, Square::E1)), &position);
        let out = inference.on_event(&event(FieldEventKind::FieldPlaced, &field_of(&geometry, Square::G1)), &position);
        assert_eq!(
            out,
            vec![InferenceOutput::MoveIntent {
                from: Square::E1,
                to: Square::G1
            }]
        );
        // Rook h1 -> f1 physically: no anomaly.
        let out = inference.on_event(&event(FieldEventKind::FieldLifted, &field_of(&geometry, Square::H1)), &position);
        assert!(out.is_empty());
        let out = inference.on_event(&event(FieldEventKind::FieldPlaced, &field_of(&geometry, Square::F1)), &position);
        assert!(out.is_empty());
    }

    #[test]
    fn stray_lift_is_an_anomaly() {
        let geometry = BoardGeometry::tabletop();
        let mut inference = inference();
        let position = Chess::default();
        let out = inference.on_event(&event(FieldEventKind::FieldLifted, &field_of(&geometry, Square::E4)), &position);
        assert!(matches!(&out[..], [InferenceOutput::Anomaly { .. }]));
    }

    #[test]
    fn confirm_zone_release_emits_buffered_intent() {
        let geometry = BoardGeometry::tabletop();
        let mut inference = MoveInference::new(geometry, CommitMode::OnConfirmZone);
        let position = Chess::default();
        inference.on_event(&event(FieldEventKind::FieldLifted, &field_of(&geometry, Square::E2)), &position);
        let out = inference.on_event(&event(FieldEventKind::FieldPlaced, &field_of(&geometry, Square::E4)), &position);
        // Intent is emitted once (for display/commit handling) and stays
        // buffered until the zone press re-emits it.
        assert_eq!(out.len(), 1);
        let out = inference.on_event(&event(FieldEventKind::FieldPlaced, "btn-south"), &position);
        assert_eq!(
            out,
            vec![
                InferenceOutput::Zone(ZoneAction::ConfirmMove { edge: TableEdge::South }),
                InferenceOutput::MoveIntent {
                    from: Square::E2,
                    to: Square::E4
                }
            ]
        );
    }
}
