use shakmaty::Square;

/// Snapshot of the start-handshake comparison: physical occupancy vs.
/// the position the board is expected to show.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlacementStatus {
    /// Squares expected to hold a piece that currently have a marker.
    pub correct: Vec<Square>,
    /// Squares expected to hold a piece but missing a marker.
    pub missing: Vec<Square>,
    /// Squares with a marker where the position expects none.
    pub unexpected: Vec<Square>,
    /// Total number of pieces the position expects.
    pub total: usize,
}

impl PlacementStatus {
    /// Whether the physical board matches the expected position - the
    /// handshake completion criterion.
    pub fn is_consistent(&self) -> bool {
        self.missing.is_empty() && self.unexpected.is_empty()
    }

    /// How many expected squares are already occupied.
    pub fn placed(&self) -> usize {
        self.correct.len()
    }
}
