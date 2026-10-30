use tangibles_chess_core::MoveRequest;

use crate::Anomaly;
use crate::ZoneAction;

/// What the move inference derived from a field event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferenceOutput {
    /// A marker lift/place sequence resolved into a move request.
    MoveIntent(MoveRequest),
    /// A marker was lifted and returned to the same square - the
    /// in-air piece is released without a move.
    Cancelled,
    /// A zone (`btn-*`/`tray-*`) event became a player action.
    Zone(ZoneAction),
    /// The event does not fit the move protocol (double lift, stacking,
    /// stray marker) - surfaced for the discrepancy/anomaly display.
    Anomaly(Anomaly),
    /// The subscription lagged; a resync via `touchPoints` is needed.
    Lagged,
}
