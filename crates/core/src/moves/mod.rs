//! Move domain - the request → outcome flow.

mod illegal_reason;
mod outcome;
mod request;

pub use illegal_reason::IllegalReason;
pub use outcome::MoveOutcome;
pub use request::MoveRequest;
