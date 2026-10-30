//! Move domain - the request → outcome flow, the ply axis, and
//! request projections.

mod check_projection;
mod illegal_reason;
mod outcome;
mod play_rejection;
mod ply;
mod request;

pub use check_projection::CheckProjection;
pub use illegal_reason::IllegalReason;
pub use outcome::MoveOutcome;
pub use play_rejection::PlayRejection;
pub use ply::Ply;
pub use request::MoveRequest;
