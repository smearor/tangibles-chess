//! Derived build geometry for one field-set build pass.

use crate::SurfaceMm;
use smearor_commons_geometry::Millimeters;
use smearor_commons_geometry::Vec2;

/// Derived build geometry: the board clamped into the surface, the
/// cell size and the confirm-zone depth scaled to it (half a square
/// fits a resting figure - ~40mm on an 80mm cell - without eating the
/// whole tray strip). All values in screen millimeters.
pub struct Layout {
    /// Board top-left corner.
    pub board: Vec2<Millimeters>,
    /// Board edge length, clamped to fit the surface.
    pub side: Millimeters,
    /// One board square's edge length (`side / 8`).
    pub cell: Millimeters,
    /// Gap between board edge and confirm zone.
    pub gap: Millimeters,
    /// Confirm-zone depth (`cell / 2`).
    pub zone: Millimeters,
    /// Surface the board is laid out on.
    pub surface: SurfaceMm,
}
