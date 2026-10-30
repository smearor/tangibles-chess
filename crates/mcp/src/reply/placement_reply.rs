//! The `placement_status` reply shape.

use serde::Serialize;

/// Placement-gate status (`placement_status` tool, tangible modes).
#[derive(Debug, Serialize)]
pub struct PlacementReply {
    /// Correctly placed markers.
    pub placed: usize,
    /// Expected piece count.
    pub total: usize,
    /// Squares expected to hold a piece but missing a marker.
    pub missing: Vec<String>,
    /// Squares holding a marker the position does not expect.
    pub unexpected: Vec<String>,
    /// Board matches the position.
    pub consistent: bool,
}
