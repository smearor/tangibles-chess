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

impl From<&tangible_objects_model::TouchPoint> for TouchPoint {
    /// Projects the upstream model type into our plain-value mirror.
    fn from(point: &tangible_objects_model::TouchPoint) -> Self {
        Self {
            slot: point.slot.as_u32(),
            tracking_id: point.tracking_id.as_i32() as i64,
            pos: ContactPos {
                x: point.pos.x,
                y: point.pos.y,
            },
        }
    }
}
