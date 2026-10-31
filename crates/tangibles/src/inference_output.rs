use tangibles_chess_moves::MoveRequest;

use crate::Anomaly;
use crate::ZoneAction;

/// What the move inference derived from a field event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferenceOutput {
    /// A marker lift/place sequence resolved into a move request. In
    /// `OnConfirmZone` mode this arrives only after the mover's zone
    /// press - the confirmed commit.
    MoveIntent(MoveRequest),
    /// In `OnConfirmZone` mode: the intent is buffered and displayed
    /// as pending - it commits only when a `btn-*` zone press turns it
    /// into a `MoveIntent`. The caller must not apply it.
    PendingIntent(MoveRequest),
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

impl From<Anomaly> for InferenceOutput {
    /// Wraps an anomaly in its output envelope (`Anomaly::X.into()`).
    fn from(anomaly: Anomaly) -> Self {
        Self::Anomaly(anomaly)
    }
}

impl From<ZoneAction> for InferenceOutput {
    /// Wraps a zone action in its output envelope.
    fn from(action: ZoneAction) -> Self {
        Self::Zone(action)
    }
}
