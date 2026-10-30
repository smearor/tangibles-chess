//! The board rectangle in screen millimetres.

use tangible_objects_model::Millimeters;

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
