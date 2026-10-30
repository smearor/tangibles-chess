//! MCP server for tangibles-chess: exposes the live game state and a
//! command surface (`make_move`, `undo`, `resign`, `agree_draw`) over
//! Streamable HTTP for debugging and agent-driven play.
//!
//! The server runs on a dedicated tokio thread; every tool call crosses
//! into the GTK main loop through an `async_channel` of [`GameRequest`]s
//! and awaits the reply on a tokio oneshot.

mod command;
mod command_error;
mod command_reply;
mod game_request;
mod mcp_error;
mod reply;
mod server;

pub use command::CommandKind;
pub use command_error::CommandError;
pub use command_reply::CommandReply;
pub use game_request::GameRequest;
pub use mcp_error::McpError;
pub use reply::DrawReply;
pub use reply::GameStateReply;
pub use reply::LegalMovesReply;
pub use reply::MakeMoveReply;
pub use reply::PlacementReply;
pub use reply::ResignReply;
pub use reply::ScreenshotReply;
pub use reply::UndoReply;
pub use reply::to_value;
pub use server::spawn_http;

/// URI of the live game-state resource (`resources/read` +
/// `resources/subscribe` stream).
pub const GAME_STATE_URI: &str = "chess://game-state";
