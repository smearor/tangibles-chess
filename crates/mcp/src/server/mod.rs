//! rmcp server surface: tools dispatch [`GameRequest`]s onto the GTK
//! main loop and await the JSON reply. Split by concern - the service
//! type, one args struct per tool, the `#[tool_router]` surface, the
//! `ServerHandler` trait impl, and the HTTP spawn.

mod chess_mcp;
mod legal_moves_args;
mod make_move_args;
mod resign_args;
mod server_handler;
mod spawn;
mod tools;

pub use spawn::spawn_http;
