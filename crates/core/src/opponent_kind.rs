use serde::Deserialize;
use serde::Serialize;

/// Who controls one side of the board.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpponentKind {
    /// A human at the table.
    #[default]
    Human,
    /// An external UCI chess engine subprocess.
    Engine,
}
