//! MCP reply payloads - one `Serialize` type per tool response.

mod draw;
mod game_state;
mod legal_moves;
mod make_move;
mod placement;
mod resign;
mod screenshot;
mod undo;

pub use draw::DrawReply;
pub use game_state::GameStateReply;
pub use legal_moves::LegalMovesReply;
pub use make_move::MakeMoveReply;
pub use placement::PlacementReply;
pub use resign::ResignReply;
pub use screenshot::ScreenshotReply;
pub use undo::UndoReply;

/// Serializes a reply payload to its JSON wire value (`null` only if a
/// `Serialize` impl ever fails, which derived impls on these payloads
/// cannot).
pub fn to_value<T: serde::Serialize>(reply: &T) -> serde_json::Value {
    serde_json::to_value(reply).unwrap_or_default()
}
