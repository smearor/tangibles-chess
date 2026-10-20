use serde::Serialize;

/// One occupancy cell of a field set registration.
///
/// Coordinates are millimeters, or `0.0..=1.0` normalized when the
/// registration's `window` carries width and height.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldInput {
    /// Cell name (`f0`..`f63`, `btn-*`, `tray-*`).
    pub name: String,
    /// Lower x bound.
    pub x_min: f64,
    /// Lower y bound.
    pub y_min: f64,
    /// Upper x bound.
    pub x_max: f64,
    /// Upper y bound.
    pub y_max: f64,
}
