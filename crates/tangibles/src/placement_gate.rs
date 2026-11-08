use shakmaty::Chess;
use shakmaty::Color;
use shakmaty::Position;
use shakmaty::Square;

use crate::BoardGeometry;
use crate::OccupancyProbe;
use crate::PlacementStatus;

/// The start-handshake gate: a tangible game may only begin once the
/// physical occupancy matches the expected position.
///
/// `check` diffs the probe against the position and yields the status
/// the `BoardCheckScreen` renders (`n/32 pieces`, missing and foreign
/// squares); during the game the same check detects anomalies.
#[derive(Debug, Clone)]
pub struct PlacementGate {
    /// Field↔square permutation for the current orientation.
    geometry: BoardGeometry,
}

impl PlacementGate {
    /// A gate for the given geometry.
    pub fn new(geometry: BoardGeometry) -> Self {
        Self { geometry }
    }

    /// Compares the probe's board occupancy against `expected`.
    ///
    /// Marker identity is 1-contact - the check verifies *occupancy*, not
    /// which piece stands where; piece-identity mismatches surface later
    /// via move inference anomalies.
    ///
    /// `physical_side` scopes the check to one army: when the opponent
    /// is virtual (engine/agent), their squares expect no marker - but
    /// a stray marker there still counts as `unexpected` since it can
    /// only confuse move inference.
    pub fn check(&self, expected: &Chess, probe: &OccupancyProbe, physical_side: Option<Color>) -> PlacementStatus {
        let mut status = PlacementStatus::default();
        for square in Square::ALL {
            let wants_piece = expected
                .board()
                .piece_at(square)
                .is_some_and(|piece| physical_side.is_none_or(|side| piece.color == side));
            let has_marker = self.geometry.square_to_field(square).is_some_and(|f| probe.occupied().contains(&f));
            match (wants_piece, has_marker) {
                (true, true) => status.correct.push(square),
                (true, false) => status.missing.push(square),
                (false, true) => status.unexpected.push(square),
                (false, false) => {}
            }
            if wants_piece {
                status.total += 1;
            }
        }
        status
    }
}

#[cfg(test)]
mod tests {
    use shakmaty::Chess;

    use tangibles_chess_geometry::FieldIndex;

    use super::PlacementGate;
    use crate::BoardGeometry;
    use crate::FieldEvent;
    use crate::FieldEventKind;
    use crate::FieldName;
    use crate::OccupancyProbe;

    fn probe_with(fields: &[usize]) -> OccupancyProbe {
        let mut probe = OccupancyProbe::default();
        for index in fields {
            probe.on_event(&FieldEvent {
                kind: FieldEventKind::FieldPlaced,
                field_set: Some("chess".to_string()),
                field: FieldIndex::new(*index).map(FieldName::Board),
                count: None,
                skipped: None,
            });
        }
        probe
    }

    #[test]
    fn full_initial_position_is_consistent() {
        let gate = PlacementGate::new(BoardGeometry::tabletop());
        // Deg0: ranks 8/7 = fields 0..16, ranks 1/2 = fields 48..64.
        let fields: Vec<usize> = (0..16).chain(48..64).collect();
        let status = gate.check(&Chess::default(), &probe_with(&fields), None);
        assert!(status.is_consistent());
        assert_eq!(status.total, 32);
        assert_eq!(status.placed(), 32);
    }

    #[test]
    fn missing_and_unexpected_are_reported() {
        let gate = PlacementGate::new(BoardGeometry::tabletop());
        // All expected pieces except field 12, plus a stray on f20.
        let fields: Vec<usize> = (0..16).filter(|i| *i != 12).chain(48..64).chain([20]).collect();
        let status = gate.check(&Chess::default(), &probe_with(&fields), None);
        assert!(!status.is_consistent());
        assert_eq!(status.missing, vec![shakmaty::Square::E7]);
        assert_eq!(status.unexpected, vec![shakmaty::Square::E6]);
    }

    #[test]
    fn virtual_opponent_needs_only_the_physical_army() {
        let gate = PlacementGate::new(BoardGeometry::tabletop());
        // Black is a virtual army (engine/agent): only the white home
        // ranks (fields 48..64) need markers.
        let fields: Vec<usize> = (48..64).collect();
        let status = gate.check(&Chess::default(), &probe_with(&fields), Some(shakmaty::Color::White));
        assert!(status.is_consistent());
        assert_eq!(status.total, 16);
        assert_eq!(status.placed(), 16);
        // A stray marker on a virtual-black square is still unexpected.
        let fields: Vec<usize> = (48..64).chain([10]).collect();
        let status = gate.check(&Chess::default(), &probe_with(&fields), Some(shakmaty::Color::White));
        assert!(!status.is_consistent());
        assert_eq!(status.unexpected, vec![shakmaty::Square::C7]);
    }
}
