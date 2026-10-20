//! UCI subprocess client driving external chess engines.
//!
//! Minimal synchronous client: spawns the engine process, performs the
//! `uci`/`isready` handshake, sends positions and parses `bestmove`.
//! Used by the app for AI opponents; blocking calls happen on dedicated
//! threads so the GTK main loop never stalls.

mod engine_client;
mod engine_error;

pub use engine_client::EngineClient;
pub use engine_error::EngineError;
