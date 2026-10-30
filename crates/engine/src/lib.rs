//! UCI subprocess client driving external chess engines.
//!
//! Minimal synchronous client: spawns the engine process, performs the
//! `uci`/`isready` handshake, sends positions and parses `bestmove`.
//! Used by the app for AI opponents; blocking calls happen on dedicated
//! threads so the GTK main loop never stalls.

mod client;
mod command;
mod error;
mod response;

pub use client::EngineClient;
pub use command::EngineCommand;
pub use error::EngineError;
pub use response::EngineResponse;
