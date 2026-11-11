/// How a theme's texture maps onto the board.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TextureLayout {
    /// One continuous sheet across the whole board (stone, felt).
    #[default]
    BoardWide,
    /// Each square shows an individual block of material with its own
    /// grain - like real wood inlay, where light and dark squares are
    /// separate pieces of maple and walnut.
    PerSquare,
}
