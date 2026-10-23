//! MCP server for tangibles-chess: exposes the live game state and a
//! command surface (`make_move`, `undo`, `resign`, `agree_draw`) over
//! Streamable HTTP for debugging and agent-driven play.
//!
//! The server runs on a dedicated tokio thread; every tool call crosses
//! into the GTK main loop through an `async_channel` of [`GameRequest`]s
//! and awaits the reply on a tokio oneshot.

mod command;
mod server;

pub use command::CommandKind;
pub use command::GameRequest;
pub use server::spawn_http;
