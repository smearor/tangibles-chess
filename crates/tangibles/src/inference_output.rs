use shakmaty::Square;

use crate::ZoneAction;

/// What the move inference derived from a field event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferenceOutput {
    /// A marker lift/place sequence resolved into a move request.
    MoveIntent {
        /// Origin square the marker was lifted from.
        from: Square,
        /// Target square the marker was placed on.
        to: Square,
    },
    /// A marker was lifted and returned to the same square - the
    /// in-air piece is released without a move.
    Cancelled,
    /// A zone (`btn-*`/`tray-*`) event became a player action.
    Zone(ZoneAction),
    /// The event does not fit the move protocol (double lift, stacking,
    /// stray marker) - surfaced for the discrepancy/anomaly display.
    Anomaly {
        /// Human-readable description for diagnostics.
        detail: String,
    },
    /// The subscription lagged; a resync via `touchPoints` is needed.
    Lagged,
}
