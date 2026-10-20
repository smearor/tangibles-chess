use thiserror::Error;

/// Errors of the UCI engine client.
#[derive(Debug, Error)]
pub enum EngineError {
    /// The engine process could not be spawned.
    #[error("spawn failed: {0}")]
    Spawn(#[from] std::io::Error),
    /// The engine sent no or a malformed answer in time.
    #[error("protocol error: {0}")]
    Protocol(String),
    /// The engine terminated unexpectedly.
    #[error("engine exited")]
    Exited,
}
