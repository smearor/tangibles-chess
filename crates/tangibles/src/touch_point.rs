use serde::Deserialize;

/// Contact position in millimeters (raw units when uncalibrated).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ContactPos {
    /// X position.
    pub x: f64,
    /// Y position.
    pub y: f64,
}

/// One active contact on the panel, as returned by `touchPoints`.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TouchPoint {
    /// Multitouch slot the contact is assigned to.
    pub slot: u32,
    /// Kernel-assigned tracking ID of the contact lifetime.
    pub tracking_id: i64,
    /// Contact position.
    pub pos: ContactPos,
}
