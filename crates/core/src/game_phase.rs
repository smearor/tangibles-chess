use serde::Deserialize;
use serde::Serialize;

/// Lifecycle phase of a game session.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GamePhase {
    /// The game is being configured; no position exists on screen yet.
    #[default]
    Setup,
    /// Tangible modes only: the app waits until the physical occupancy
    /// matches the expected position (start handshake).
    AwaitingPlacement,
    /// The game is live; moves are validated and applied.
    InProgress,
    /// The game ended (decisive result, draw or resignation).
    Finished,
}
