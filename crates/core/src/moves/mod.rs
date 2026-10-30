//! Move domain - the request → outcome flow.

mod illegal_reason;
mod move_outcome;
mod move_request;

pub use illegal_reason::IllegalReason;
pub use move_outcome::MoveOutcome;
pub use move_request::MoveRequest;
