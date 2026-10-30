//! The live game controller: session glue between rules, tangibles,
//! and UI. Split by concern - construction/startup, the global
//! registry, tangible lifecycle, the MCP endpoint, scene sync, and the
//! four capability trait impls.

mod game_controller;
mod mcp;
mod playable;
mod registry;
mod replayable;
mod rotatable;
mod source;
mod sync;
mod tangibles;
mod undoable;

pub use game_controller::GameController;
pub use registry::set_active_controller;
pub use registry::with_controller;
