use shakmaty::Chess;
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
    pub fn check(&self, expected: &Chess, probe: &OccupancyProbe) -> PlacementStatus {
        let mut status = PlacementStatus::default();
        for square in Square::ALL {
            let wants_piece = expected.board().piece_at(square).is_some();
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

    use tangibles_chess_core::FieldIndex;

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
        let status = gate.check(&Chess::default(), &probe_with(&fields));
        assert!(status.is_consistent());
        assert_eq!(status.total, 32);
        assert_eq!(status.placed(), 32);
    }

    #[test]
    fn missing_and_unexpected_are_reported() {
        let gate = PlacementGate::new(BoardGeometry::tabletop());
        // All expected pieces except field 12, plus a stray on f20.
        let fields: Vec<usize> = (0..16).filter(|i| *i != 12).chain(48..64).chain([20]).collect();
        let status = gate.check(&Chess::default(), &probe_with(&fields));
        assert!(!status.is_consistent());
        assert_eq!(status.missing, vec![shakmaty::Square::E7]);
        assert_eq!(status.unexpected, vec![shakmaty::Square::E6]);
    }
}
