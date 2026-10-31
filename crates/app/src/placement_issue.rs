//! Which placement problem a status line part reports - the two
//! discrepancy kinds of `PlacementStatus`.

/// A placement discrepancy: what is wrong on the board vs. the
/// position.
#[derive(Debug, Clone, Copy)]
pub enum PlacementIssue {
    /// A square the position expects a marker on has none
    /// (`PlacementStatus::missing`).
    Missing,
    /// A square holds a marker the position does not expect
    /// (`PlacementStatus::unexpected`).
    Unexpected,
}

impl PlacementIssue {
    /// The Fluent key for the status line hint.
    pub fn l10n_key(&self) -> &'static str {
        match self {
            Self::Missing => "check-missing",
            Self::Unexpected => "check-unexpected",
        }
    }
}
