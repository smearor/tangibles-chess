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
use shakmaty::uci::UciMove;

use crate::EngineError;

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
        let stdin = child.stdin.take().ok_or(EngineError::Protocol("no stdin".to_string()))?;
        let stdout = child.stdout.take().ok_or(EngineError::Protocol("no stdout".to_string()))?;
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let mut engine = Self { child, stdin, lines };
        engine.send("uci")?;
        engine.expect_line("uciok", Duration::from_secs(5))?;
        engine.send("isready")?;
        engine.expect_line("readyok", Duration::from_secs(5))?;
        Ok(engine)
    }

    /// Sends one command line to the engine.
    pub fn send(&mut self, command: &str) -> Result<(), EngineError> {
        writeln!(self.stdin, "{command}")?;
        self.stdin.flush()?;
        Ok(())
    }

    /// Waits for a line whose first word equals `expected`, within
    /// `timeout`. Intermediate lines are consumed and discarded.
    fn expect_line(&mut self, expected: &str, timeout: Duration) -> Result<String, EngineError> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(EngineError::Protocol(format!("timeout waiting for `{expected}`")));
            }
            let line = self
                .lines
                .recv_timeout(remaining)
                .map_err(|_| EngineError::Protocol(format!("timeout waiting for `{expected}`")))?;
            if line.starts_with(expected) {
                return Ok(line);
            }
        }
    }

    /// Starts a fresh game (`ucinewgame`).
    pub fn new_game(&mut self) -> Result<(), EngineError> {
        self.send("ucinewgame")?;
        Ok(())
    }

    /// Plays moves on the current game and asks for a move at `depth`.
    pub fn best_move(&mut self, position: &Chess, fen: &str, depth: u32) -> Result<Option<Move>, EngineError> {
        self.send(&format!("position fen {fen}"))?;
        self.send(&format!("go depth {depth}"))?;
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                self.send("stop")?;
                return Err(EngineError::Protocol("no bestmove in time".to_string()));
            }
            let Ok(line) = self.lines.recv_timeout(remaining) else {
                self.send("stop")?;
                return Err(EngineError::Protocol("no bestmove in time".to_string()));
            };
            if let Some(best) = line.strip_prefix("bestmove ") {
                let token = best.split_whitespace().next().unwrap_or_default();
                if token == "(none)" || token.is_empty() {
                    return Ok(None);
                }
                return token
                    .parse::<UciMove>()
                    .ok()
                    .and_then(|uci| uci.to_move(position).ok())
                    .map(Some)
                    .ok_or_else(|| EngineError::Protocol(format!("illegal bestmove `{token}`")));
            }
        }
    }
}

impl Drop for EngineClient {
    fn drop(&mut self) {
        let _ = self.send("quit");
        let _ = self.child.kill();
    }
}

#[cfg(test)]
mod tests {
    // Engine tests require a stockfish binary; kept for integration runs.
}
