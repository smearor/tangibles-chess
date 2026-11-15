//! The touch surface's size in millimetres.

use smearor_commons_geometry::Millimeters;
use smearor_commons_geometry::Vec2;
use std::ops::Deref;

/// Overall window (screen) size in screen mm - the bounds every field
/// set is built against and clipped into.
///
/// Wraps [`Vec2<Millimeters>`] with the domain name (`x` is the width,
/// `y` the height, following the [`smearor_commons_geometry::Size`]
/// convention); `Deref` exposes the vector's methods directly.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SurfaceMm(pub Vec2<Millimeters>);

impl SurfaceMm {
    /// Assumed panel size when the daemon reports no calibration
    /// (1000x600mm - a plausible desk surface extent).
    pub const FALLBACK: Self = Self(Vec2::new(1000.0, 600.0));

    /// Surface width in screen mm.
    pub fn width(&self) -> Millimeters {
        self.0.x_unit()
    }

    /// Surface height in screen mm.
    pub fn height(&self) -> Millimeters {
        self.0.y_unit()
    }

    /// Whether `x` lies within the surface's horizontal bounds.
    pub fn contains_x(&self, x: Millimeters) -> bool {
        (Millimeters::default()..=self.width()).contains(&x)
    }
}

impl Deref for SurfaceMm {
    type Target = Vec2<Millimeters>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<(f64, f64)> for SurfaceMm {
    fn from((width, height): (f64, f64)) -> Self {
        Self(Vec2::new(width, height))
    }
}

impl From<(Millimeters, Millimeters)> for SurfaceMm {
    fn from((width, height): (Millimeters, Millimeters)) -> Self {
        Self(Vec2::from_mm(width, height))
    }
}

impl From<Vec2<Millimeters>> for SurfaceMm {
    fn from(size: Vec2<Millimeters>) -> Self {
        Self(size)
    }
}

impl From<SurfaceMm> for Vec2<Millimeters> {
    fn from(surface: SurfaceMm) -> Self {
        surface.0
    }
}

impl From<SurfaceMm> for (f64, f64) {
    fn from(surface: SurfaceMm) -> Self {
        (surface.0.x, surface.0.y)
    }
}
