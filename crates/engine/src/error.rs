use thiserror::Error;

/// Errors of the UCI engine client.
#[derive(Debug, Error)]
pub enum EngineError {
    /// The engine process could not be spawned.
    #[error("spawn failed: {0}")]
    Spawn(#[from] std::io::Error),
    /// The spawned process did not expose a stdin pipe.
    #[error("engine has no stdin")]
    MissingStdin,
    /// The spawned process did not expose a stdout pipe.
    #[error("engine has no stdout")]
    MissingStdout,
    /// A protocol response did not arrive within the timeout
    /// (`{0}` = the awaited protocol keyword).
    #[error("timeout waiting for `{0}`")]
    Timeout(&'static str),
    /// The search produced no `bestmove` before the deadline.
    #[error("no bestmove in time")]
    NoBestMove,
    /// The `bestmove` token is not valid UCI.
    #[error("illegal bestmove `{0}`")]
    IllegalBestMove(String),
    /// The `bestmove` is valid UCI but illegal in the position.
    #[error("bestmove not legal in the position")]
    BestMoveNotLegal,
    /// The engine terminated unexpectedly.
    #[error("engine exited")]
    Exited,
}
