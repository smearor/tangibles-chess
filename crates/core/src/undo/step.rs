use shakmaty::Square;
use tangibles_chess_material::Piece;

/// One physical step of an [`crate::UndoPlan`].
///
/// Steps describe marker movements needed to physically restore the
/// previous position. `MoveBack` steps come first (they free the target
/// squares), `Restore` steps afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoStep {
    /// Move the marker standing on `to` back onto `from`.
    MoveBack {
        /// Square the piece originally moved away from.
        from: Square,
        /// Square the piece currently stands on.
        to: Square,
    },
    /// Place a captured piece's marker back on its square.
    Restore {
        /// Square the captured piece stood on.
        square: Square,
        /// The captured piece (side and role).
        piece: Piece,
    },
}

impl std::fmt::Display for UndoStep {
    /// Human-readable instruction (`"move e4 back to e2"`).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MoveBack { from, to } => write!(f, "move {to} back to {from}"),
            Self::Restore { square, piece } => write!(f, "restore {piece} on {square}"),
        }
    }
}
