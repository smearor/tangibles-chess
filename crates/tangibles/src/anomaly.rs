//! Why a field event does not fit the move protocol.

/// A protocol anomaly detected by [`crate::MoveInference`]: the event
/// does not fit the lift/place move protocol (double lift, stacking,
/// stray marker) - surfaced for the discrepancy/anomaly display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anomaly {
    /// The opponent marker under a landing attacker was removed, but
    /// no attacking piece is in the air.
    CaptureWithoutAir,
    /// A marker was lifted from a square the position reports empty.
    LiftOnEmptySquare,
    /// A second own marker was lifted while a piece is in the air.
    TwoPiecesLifted,
    /// A marker of the non-moving side was lifted outside capture
    /// preparation.
    NonMovingSideLifted,
    /// The rook's physical move landed off its castling target.
    RookOffCastlingTarget,
    /// A marker touched down without a preceding lift.
    PlaceWithoutLift,
    /// A marker was placed on a square occupied by an own piece.
    PlacedOnOwnPiece,
    /// More than one contact rests on a single square.
    StackedContacts,
}

impl std::fmt::Display for Anomaly {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let detail = match self {
            Self::CaptureWithoutAir => "capture completion without a piece in the air",
            Self::LiftOnEmptySquare => "lift on an empty square",
            Self::TwoPiecesLifted => "two pieces lifted at once",
            Self::NonMovingSideLifted => "piece of the non-moving side lifted",
            Self::RookOffCastlingTarget => "rook placed off its castling target",
            Self::PlaceWithoutLift => "marker placed without a lift",
            Self::PlacedOnOwnPiece => "marker placed on own piece",
            Self::StackedContacts => "multiple contacts stacked on one square",
        };
        f.write_str(detail)
    }
}

#[cfg(test)]
mod tests {
    use super::Anomaly;

    #[test]
    fn every_variant_has_a_detail_message() {
        for anomaly in [
            Anomaly::CaptureWithoutAir,
            Anomaly::LiftOnEmptySquare,
            Anomaly::TwoPiecesLifted,
            Anomaly::NonMovingSideLifted,
            Anomaly::RookOffCastlingTarget,
            Anomaly::PlaceWithoutLift,
            Anomaly::PlacedOnOwnPiece,
            Anomaly::StackedContacts,
        ] {
            assert!(!anomaly.to_string().is_empty());
        }
    }
}
