use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::process::Child;
use std::process::ChildStdin;
use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

use shakmaty::Chess;
use shakmaty::Move;
use tangibles_chess_core::Fen;

use crate::EngineError;
use crate::command::EngineCommand;
use crate::response::EngineResponse;

/// A running UCI engine process.
///
/// Blocking line I/O on the engine's stdout is offloaded to a reader
/// thread so `go` responses can stream in while the caller waits with a
/// timeout.
pub struct EngineClient {
    /// The spawned engine process.
    child: Child,
    /// Stdin writer.
    stdin: ChildStdin,
    /// Lines received on stdout.
    lines: mpsc::Receiver<String>,
}

impl EngineClient {
    /// Spawns `path` and performs the `uci` + `isready` handshake.
    pub fn spawn(path: &str) -> Result<Self, EngineError> {
        let mut child = Command::new(path).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
        let stdin = child.stdin.take().ok_or(EngineError::MissingStdin)?;
        let stdout = child.stdout.take().ok_or(EngineError::MissingStdout)?;
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let mut engine = Self { child, stdin, lines };
        engine.send(&EngineCommand::Uci)?;
        engine.expect_response(&EngineResponse::UciOk, Duration::from_secs(5))?;
        engine.send(&EngineCommand::IsReady)?;
        engine.expect_response(&EngineResponse::ReadyOk, Duration::from_secs(5))?;
        Ok(engine)
    }

    /// Sends one command line to the engine.
    pub fn send(&mut self, command: &EngineCommand) -> Result<(), EngineError> {
        writeln!(self.stdin, "{command}")?;
        self.stdin.flush()?;
        Ok(())
    }

    /// Waits for a response whose keyword matches `expected`'s, within
    /// `timeout`. Intermediate lines are consumed and discarded.
    fn expect_response(&mut self, expected: &EngineResponse, timeout: Duration) -> Result<EngineResponse, EngineError> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(EngineError::Timeout(expected.keyword()));
            }
            let line = self.lines.recv_timeout(remaining).map_err(|_| EngineError::Timeout(expected.keyword()))?;
            let response = EngineResponse::parse(&line);
            if response.keyword() == expected.keyword() {
                return Ok(response);
            }
        }
    }

    /// Starts a fresh game (`ucinewgame`).
    pub fn new_game(&mut self) -> Result<(), EngineError> {
        self.send(&EngineCommand::UciNewGame)?;
        Ok(())
    }

    /// Plays moves on the current game and asks for a move at `depth`.
    pub fn best_move(&mut self, position: &Chess, fen: &Fen, depth: u32) -> Result<Option<Move>, EngineError> {
        self.send(&EngineCommand::Position { fen: fen.clone() })?;
        self.send(&EngineCommand::Go { depth })?;
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                self.send(&EngineCommand::Stop)?;
                return Err(EngineError::NoBestMove);
            }
            let Ok(line) = self.lines.recv_timeout(remaining) else {
                self.send(&EngineCommand::Stop)?;
                return Err(EngineError::NoBestMove);
            };
            match EngineResponse::parse(&line) {
                EngineResponse::BestMove(mv) => {
                    return mv.map(|uci| uci.to_move(position).map_err(|_| EngineError::BestMoveNotLegal)).transpose();
                }
                EngineResponse::InvalidBestMove(token) => {
                    return Err(EngineError::IllegalBestMove(token));
                }
                _ => {}
            }
        }
    }
}

impl Drop for EngineClient {
    fn drop(&mut self) {
        let _ = self.send(&EngineCommand::Quit);
        let _ = self.child.kill();
    }
}

#[cfg(test)]
mod tests {
    // Engine tests require a stockfish binary; kept for integration runs.
}
