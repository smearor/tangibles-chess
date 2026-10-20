use shakmaty::Role;
use shakmaty::Square;

/// A requested move between two squares, as produced by touch input or
/// the tangible move inference.
///
/// `promotion` is only consulted when `from`/`to` describe a promoting
/// pawn move; `None` there yields [`crate::MoveOutcome::NeedsPromotion`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveRequest {
    /// Origin square.
    pub from: Square,
    /// Target square.
    pub to: Square,
    /// Promotion role for pawn moves to the last rank.
    pub promotion: Option<Role>,
}

impl MoveRequest {
    /// A move request without promotion.
    pub fn new(from: Square, to: Square) -> Self {
        Self { from, to, promotion: None }
    }

    /// A move request with promotion role.
    pub fn promoting(from: Square, to: Square, promotion: Role) -> Self {
        Self {
            from,
            to,
            promotion: Some(promotion),
        }
    }
}
