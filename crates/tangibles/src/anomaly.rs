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
    /// A `btn-*` press on the non-mover's edge while an intent is
    /// pending - only the mover's edge completes a commit.
    ConfirmWrongEdge,
    /// A `btn-*` press with no pending intent to confirm.
    ConfirmWithoutPending,
}

impl Anomaly {
    /// Whether the violation invalidates the in-flight board
    /// interpretation: gesture anomalies leave `MoveInference` state
    /// untrustworthy and warrant a `reset()` + occupancy resync, while
    /// zone-press anomalies (`btn-*` mistaps) leave the buffered intent
    /// and the physical read intact - warn only.
    pub fn needs_resync(&self) -> bool {
        !matches!(self, Self::ConfirmWrongEdge | Self::ConfirmWithoutPending)
    }

    /// All anomaly kinds, for exhaustive iteration (tests).
    pub const ALL: [Self; 10] = [
        Self::CaptureWithoutAir,
        Self::LiftOnEmptySquare,
        Self::TwoPiecesLifted,
        Self::NonMovingSideLifted,
        Self::RookOffCastlingTarget,
        Self::PlaceWithoutLift,
        Self::PlacedOnOwnPiece,
        Self::StackedContacts,
        Self::ConfirmWrongEdge,
        Self::ConfirmWithoutPending,
    ];
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
            Self::ConfirmWrongEdge => "confirm pressed on the non-mover's edge",
            Self::ConfirmWithoutPending => "confirm pressed with nothing pending",
        };
        f.write_str(detail)
    }
}

#[cfg(test)]
mod tests {
    use super::Anomaly;

    #[test]
    fn every_variant_has_a_detail_message() {
        for anomaly in Anomaly::ALL {
            assert!(!anomaly.to_string().is_empty());
        }
    }
}
