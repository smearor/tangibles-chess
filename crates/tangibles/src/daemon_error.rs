use tangible_objects_model::RejectionKind;
use thiserror::Error;

/// Errors of the daemon-facing layer.
#[derive(Debug, Error)]
pub enum DaemonError {
    /// The HTTP/WebSocket transport failed.
    #[error("transport error: {0}")]
    Transport(String),
    /// The daemon returned a GraphQL error.
    #[error("graphql error: {0}")]
    GraphQl(String),
    /// A response could not be decoded.
    #[error("decode error: {0}")]
    Decode(String),
    /// The daemon is not reachable or the connection closed.
    #[error("daemon unavailable: {0}")]
    Unavailable(String),
    /// A registration was rejected with a typed reason (`kind` is the
    /// machine category, `reason` the daemon's message).
    #[error("registration rejected ({kind:?}): {reason}")]
    Rejected {
        /// Machine-readable rejection category.
        kind: RejectionKind,
        /// The daemon's human-readable rejection message.
        reason: String,
    },
}
