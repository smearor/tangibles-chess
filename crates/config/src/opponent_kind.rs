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
    /// An external agent driving its moves over the MCP endpoint -
    /// the controller never computes or requests a move for this
    /// side (like `Human`, but the moves arrive via `make_move`).
    Agent,
}
