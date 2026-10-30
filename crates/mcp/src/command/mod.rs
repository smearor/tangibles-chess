//! The command protocol between the MCP thread and the GTK main loop:
//! kinds in, typed replies and application errors out.

mod error;
mod kind;
mod reply;

pub use error::CommandError;
pub use kind::CommandKind;
pub use reply::CommandReply;
