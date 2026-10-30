//! A single captured-piece parking cell beside the board.

use shakmaty::Piece;

/// One cell of a captured-piece parking strip: whether a physical
/// marker rests on it and which captured piece renders there.
#[derive(Debug, Clone, Copy, Default)]
pub struct CapCell {
    /// A physical marker currently rests on the cell.
    pub occupied: bool,
    /// The captured piece rendered on the cell - `None` while empty or
    /// in modes that draw no piece glyphs.
    pub piece: Option<Piece>,
}
