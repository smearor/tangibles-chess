//! The `screenshot` reply shape.

use serde::Serialize;

/// Window screenshot (`screenshot` tool).
#[derive(Debug, Serialize)]
pub struct ScreenshotReply {
    /// PNG image, base64-encoded.
    pub png_base64: String,
    /// Pixel width.
    pub width: i32,
    /// Pixel height.
    pub height: i32,
}
