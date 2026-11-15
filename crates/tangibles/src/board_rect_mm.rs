//! The board rectangle in screen millimetres.

use smearor_commons_geometry::Millimeters;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Vec2;

/// The board rectangle in screen mm - the field-set geometry domain:
/// `x`/`y` = top-left corner, `side` = edge length.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BoardRectMm {
    /// Left edge in screen mm.
    pub x: Millimeters,
    /// Top edge in screen mm.
    pub y: Millimeters,
    /// Board edge length in screen mm.
    pub side: Millimeters,
}

impl BoardRectMm {
    /// A board square's edge length in screen mm.
    pub fn cell(&self) -> Millimeters {
        self.side * 0.125
    }

    /// Right edge (`x + side`).
    pub fn x_max(&self) -> Millimeters {
        self.x + self.side
    }

    /// Bottom edge (`y + side`).
    pub fn y_max(&self) -> Millimeters {
        self.y + self.side
    }

    /// The board's top-left corner.
    pub fn origin(&self) -> Vec2<Millimeters> {
        Vec2::from_mm(self.x, self.y)
    }

    /// The board area as a generic [`Rect`] in screen mm.
    pub fn rect(&self) -> Rect<Millimeters> {
        Rect::new(self.origin(), Vec2::splat(self.side))
    }
}

impl From<(f64, f64, f64)> for BoardRectMm {
    fn from((x, y, side): (f64, f64, f64)) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            side: side.into(),
        }
    }
}

impl From<BoardRectMm> for (f64, f64, f64) {
    fn from(rect: BoardRectMm) -> Self {
        (rect.x.as_f64(), rect.y.as_f64(), rect.side.as_f64())
    }
}

/// The rect's top-left corner plus its shorter edge as the board side.
impl From<Rect<Millimeters>> for BoardRectMm {
    fn from(rect: Rect<Millimeters>) -> Self {
        Self {
            x: rect.min.x.into(),
            y: rect.min.y.into(),
            side: rect.size.x.min(rect.size.y).into(),
        }
    }
}

impl From<BoardRectMm> for Rect<Millimeters> {
    fn from(board: BoardRectMm) -> Self {
        board.rect()
    }
}
