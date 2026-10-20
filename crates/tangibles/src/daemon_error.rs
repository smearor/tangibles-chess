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
}
