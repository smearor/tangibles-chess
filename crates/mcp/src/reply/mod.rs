//! MCP reply payloads - one `Serialize` type per tool response.

mod draw_reply;
mod error_reply;
mod game_state_reply;
mod legal_moves_reply;
mod make_move_reply;
mod placement_reply;
mod resign_reply;
mod screenshot_reply;
mod undo_reply;

pub use draw_reply::DrawReply;
pub use error_reply::ErrorReply;
pub use game_state_reply::GameStateReply;
pub use legal_moves_reply::LegalMovesReply;
pub use make_move_reply::MakeMoveReply;
pub use placement_reply::PlacementReply;
pub use resign_reply::ResignReply;
pub use screenshot_reply::ScreenshotReply;
pub use undo_reply::UndoReply;

/// Serializes a reply payload to its JSON wire value (`null` only if a
/// `Serialize` impl ever fails, which derived impls on these payloads
/// cannot).
pub fn to_value<T: serde::Serialize>(reply: &T) -> serde_json::Value {
    serde_json::to_value(reply).unwrap_or_default()
}
