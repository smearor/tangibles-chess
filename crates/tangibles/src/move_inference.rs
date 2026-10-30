use std::collections::BTreeSet;

use shakmaty::Chess;
use shakmaty::Position;
use shakmaty::Square;
use tangibles_chess_core::CommitMode;
use tangibles_chess_core::MoveRequest;

use crate::Anomaly;
use crate::BoardGeometry;
use crate::CapSide;
use crate::FieldEvent;
use crate::FieldEventKind;
use crate::FieldName;
use crate::InferenceOutput;
use crate::TableEdge;
use crate::Zone;
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
    pending_castle_rook: Option<MoveRequest>,
    /// Intent waiting for a confirm-zone press (`OnConfirmZone`).
    pending_confirm: Option<MoveRequest>,
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
        match &event.field {
            Some(FieldName::Board(index)) => {
                let square = self.geometry.field_to_square(*index);
                match event.kind {
                    FieldEventKind::FieldLifted => self.on_lift(square, position),
                    FieldEventKind::FieldPlaced => self.on_place(square, position),
                    FieldEventKind::FieldCountChanged => self.on_count(square, event.count.unwrap_or(0)),
                    FieldEventKind::Lagged | FieldEventKind::Other => Vec::new(),
                }
            }
            Some(FieldName::Zone(zone)) => self.on_zone(*zone, event),
            Some(FieldName::Unknown(_)) | None => Vec::new(),
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
                return vec![InferenceOutput::Anomaly(Anomaly::CaptureWithoutAir)];
            };
            self.lifted_opponents.clear();
            return vec![self.commit(from, square, position)];
        }
        // A rook lift following a castle intent is the physical rook move.
        if self.pending_castle_rook.is_some_and(|rook| rook.from == square) {
            return Vec::new();
        }
        let Some(piece) = position.board().piece_at(square) else {
            return vec![InferenceOutput::Anomaly(Anomaly::LiftOnEmptySquare)];
        };
        if self.in_air.is_some() {
            // While a piece is in the air, lifting an opponent marker is
            // the capture preparation; lifting another own marker is not
            // part of the protocol.
            if piece.color != position.turn() {
                self.lifted_opponents.insert(square);
                return Vec::new();
            }
            return vec![InferenceOutput::Anomaly(Anomaly::TwoPiecesLifted)];
        }
        if piece.color != position.turn() {
            return vec![InferenceOutput::Anomaly(Anomaly::NonMovingSideLifted)];
        }
        self.in_air = Some(square);
        Vec::new()
    }

    /// A marker touched down on a board square.
    fn on_place(&mut self, square: Square, position: &Chess) -> Vec<InferenceOutput> {
        // Completing the rook's physical move after a castle intent.
        if let Some(rook) = self.pending_castle_rook {
            if square == rook.to {
                self.pending_castle_rook = None;
                return Vec::new();
            }
            return vec![InferenceOutput::Anomaly(Anomaly::RookOffCastlingTarget)];
        }
        let Some(from) = self.in_air else {
            return vec![InferenceOutput::Anomaly(Anomaly::PlaceWithoutLift)];
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
            Some(_own) => vec![InferenceOutput::Anomaly(Anomaly::PlacedOnOwnPiece)],
        }
    }

    /// Contact count changed on a board square; stacking (>1) is an
    /// anomaly worth surfacing.
    fn on_count(&self, square: Square, count: i32) -> Vec<InferenceOutput> {
        if count > 1 {
            return vec![InferenceOutput::Anomaly(Anomaly::StackedContacts)];
        }
        let _ = square;
        Vec::new()
    }

    /// Emits the intent directly or buffers it for the confirm zone.
    fn commit(&mut self, from: Square, to: Square, position: &Chess) -> InferenceOutput {
        let request = MoveRequest::new(from, to);
        // Detect castling: the rook's physical move is expected next.
        self.pending_castle_rook = position
            .legal_moves()
            .into_iter()
            .find(|mv| mv.is_castle() && request.matches(mv))
            .and_then(|mv| MoveRequest::castle_rook(&mv));
        if self.commit_mode == CommitMode::OnConfirmZone {
            self.pending_confirm = Some(request);
        }
        InferenceOutput::MoveIntent(request)
    }

    /// Routes zone fields to player actions.
    fn on_zone(&mut self, zone: Zone, event: &FieldEvent) -> Vec<InferenceOutput> {
        match zone {
            zone @ (Zone::BtnSouth | Zone::BtnNorth) => self.on_confirm_zone(zone.edge(), event),
            zone @ (Zone::TraySouth | Zone::TrayNorth) => self.on_tray(zone.edge(), event),
            Zone::Cap { side, row } => Self::on_cap_cell(side, row, event),
        }
    }

    /// Parking cells only carry occupancy - no move inference.
    fn on_cap_cell(side: CapSide, row: u8, event: &FieldEvent) -> Vec<InferenceOutput> {
        let occupied = match event.kind {
            FieldEventKind::FieldPlaced => true,
            FieldEventKind::FieldLifted => false,
            FieldEventKind::FieldCountChanged => event.count.is_some_and(|count| count > 0),
            _ => return Vec::new(),
        };
        vec![InferenceOutput::Zone(ZoneAction::CapCell { side, row, occupied })]
    }

    /// A marker was placed on a confirm zone: completes a buffered intent
    /// or surfaces a stray press.
    fn on_confirm_zone(&mut self, edge: TableEdge, event: &FieldEvent) -> Vec<InferenceOutput> {
        if event.kind != FieldEventKind::FieldPlaced {
            return Vec::new();
        }
        let mut outputs = vec![InferenceOutput::Zone(ZoneAction::ConfirmMove { edge })];
        if let Some(request) = self.pending_confirm.take() {
            outputs.push(InferenceOutput::MoveIntent(request));
        }
        outputs
    }

    /// Tray occupancy changes are reported as captured-material counts.
    fn on_tray(&self, edge: TableEdge, event: &FieldEvent) -> Vec<InferenceOutput> {
        match event.kind {
            FieldEventKind::FieldPlaced => vec![InferenceOutput::Zone(ZoneAction::TrayCount { edge, count: 1 })],
            FieldEventKind::FieldLifted => vec![InferenceOutput::Zone(ZoneAction::TrayCount { edge, count: 0 })],
            FieldEventKind::FieldCountChanged => vec![InferenceOutput::Zone(ZoneAction::TrayCount {
                edge,
                count: event.count.unwrap_or(0).max(0) as u32,
            })],
            FieldEventKind::Lagged | FieldEventKind::Other => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use shakmaty::CastlingMode;
    use shakmaty::Chess;
    use shakmaty::Square;
    use shakmaty::fen::Fen;
    use tangibles_chess_core::CommitMode;
    use tangibles_chess_core::MoveRequest;

    use super::MoveInference;
    use crate::BoardGeometry;
    use crate::FieldEvent;
    use crate::FieldEventKind;
    use crate::FieldName;
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
            field: Some(FieldName::from(field)),
            count: None,
            skipped: None,
        }
    }

    fn field_of(geometry: &BoardGeometry, square: Square) -> String {
        geometry.square_to_field(square).map(|index| index.to_string()).expect("field")
    }

    #[test]
    fn lift_and_place_yields_move_intent() {
        let geometry = BoardGeometry::tabletop();
        let mut inference = inference();
        let position = Chess::default();
        let out = inference.on_event(&event(FieldEventKind::FieldLifted, &field_of(&geometry, Square::E2)), &position);
        assert!(out.is_empty());
        let out = inference.on_event(&event(FieldEventKind::FieldPlaced, &field_of(&geometry, Square::E4)), &position);
        assert_eq!(out, vec![InferenceOutput::MoveIntent(MoveRequest::new(Square::E2, Square::E4))]);
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
        assert_eq!(out, vec![InferenceOutput::MoveIntent(MoveRequest::new(Square::E5, Square::D5))]);
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
        assert_eq!(out, vec![InferenceOutput::MoveIntent(MoveRequest::new(Square::E5, Square::D5))]);
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
        assert_eq!(out, vec![InferenceOutput::MoveIntent(MoveRequest::new(Square::E1, Square::G1))]);
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
                InferenceOutput::MoveIntent(MoveRequest::new(Square::E2, Square::E4))
            ]
        );
    }
}
