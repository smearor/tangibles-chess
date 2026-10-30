//! The `placement_status` reply shape.

use serde::Serialize;
use tangibles_chess_notation::SquareName;

/// Placement-gate status (`placement_status` tool, tangible modes).
#[derive(Debug, Serialize)]
pub struct PlacementReply {
    /// Correctly placed markers.
    pub placed: usize,
    /// Expected piece count.
    pub total: usize,
    /// Squares expected to hold a piece but missing a marker.
    pub missing: Vec<SquareName>,
    /// Squares holding a marker the position does not expect.
    pub unexpected: Vec<SquareName>,
    /// Board matches the position.
    pub consistent: bool,
}
