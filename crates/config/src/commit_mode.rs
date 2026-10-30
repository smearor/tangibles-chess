use serde::Deserialize;
use serde::Serialize;

/// When a tangible move is committed to the game.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommitMode {
    /// The move applies as soon as the marker is placed on its target
    /// square (casual play).
    #[default]
    OnPlace,
    /// The move only applies after the moving player places the marker on
    /// their `btn-*` confirm zone - the physical clock press
    /// (tournament-style play, allows in-air corrections).
    OnConfirmZone,
}
