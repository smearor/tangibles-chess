//! The board rectangle in screen millimetres.

use smearor_commons_geometry::Millimeters;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Vec2;

/// The board rectangle in screen mm - the field-set geometry domain:
/// `x`/`y` = top-left corner, `side` = edge length.
///
/// Thin domain wrapper around [`Rect<Millimeters>`]: the board is a
/// square, so the stored rect is normalized to `side x side` on
/// construction and the field stays private - the invariant is
/// unforgeable.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardRectMm(Rect<Millimeters>);

impl BoardRectMm {
    /// Left edge in screen mm.
    pub fn x(&self) -> Millimeters {
        self.0.min.x_unit()
    }

    /// Top edge in screen mm.
    pub fn y(&self) -> Millimeters {
        self.0.min.y_unit()
    }

    /// Board edge length in screen mm.
    pub fn side(&self) -> Millimeters {
        self.0.size.x_unit()
    }

    /// A board square's edge length in screen mm.
    pub fn cell(&self) -> Millimeters {
        self.side() * 0.125
    }

    /// Right edge (`x + side`).
    pub fn x_max(&self) -> Millimeters {
        Millimeters::new(self.0.right())
    }

    /// Bottom edge (`y + side`).
    pub fn y_max(&self) -> Millimeters {
        Millimeters::new(self.0.bottom())
    }

    /// The board's top-left corner.
    pub fn origin(&self) -> Vec2<Millimeters> {
        self.0.min
    }

    /// The board area as a generic [`Rect`] in screen mm.
    pub fn rect(&self) -> Rect<Millimeters> {
        self.0
    }
}

impl Default for BoardRectMm {
    fn default() -> Self {
        Self::from((0.0, 0.0, 0.0))
    }
}

impl From<(f64, f64, f64)> for BoardRectMm {
    fn from((x, y, side): (f64, f64, f64)) -> Self {
        Self(Rect::<Millimeters>::from_xywh(x, y, side, side))
    }
}

impl From<BoardRectMm> for (f64, f64, f64) {
    fn from(board: BoardRectMm) -> Self {
        (board.x().as_f64(), board.y().as_f64(), board.side().as_f64())
    }
}

/// The rect's top-left corner plus its shorter edge as the board side -
/// non-square input is normalized to a square, keeping the invariant.
impl From<Rect<Millimeters>> for BoardRectMm {
    fn from(rect: Rect<Millimeters>) -> Self {
        let side = rect.size.x.min(rect.size.y);
        Self(Rect::<Millimeters>::from_xywh(rect.min.x, rect.min.y, side, side))
    }
}

impl From<BoardRectMm> for Rect<Millimeters> {
    fn from(board: BoardRectMm) -> Self {
        board.0
    }
}
