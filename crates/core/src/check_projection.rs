use shakmaty::Square;

/// What would be check after a naively applied move - the reason a
/// request like "pinned piece" or "king into an attacked square" is
/// illegal. Used to explain rejected moves visually.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckProjection {
    /// Square of the king that would be left in check.
    pub king: Square,
    /// Attacker squares giving that check.
    pub attackers: Vec<Square>,
}
