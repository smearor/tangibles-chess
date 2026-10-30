//! A UCI protocol line received from the engine.

use shakmaty::uci::UciMove;

/// One response line of the UCI (Universal Chess Interface) protocol.
/// Parsed by [`EngineResponse::parse`]; `info`/`option`/`id` and any
/// unrecognized line map to [`Other`](Self::Other).
#[derive(Debug)]
pub enum EngineResponse {
    /// Engine acknowledged UCI mode (`uciok`).
    UciOk,
    /// Engine answered `isready` (`readyok`).
    ReadyOk,
    /// Search answer (`bestmove <uci>`; `bestmove (none)` → `None`).
    BestMove(Option<UciMove>),
    /// A `bestmove` whose token is not valid UCI (`bestmove xyz`).
    InvalidBestMove(String),
    /// Any other protocol line (`info`, `option`, `id`, ...).
    Other(String),
}

impl EngineResponse {
    /// The protocol keyword this response line starts with.
    pub fn keyword(&self) -> &'static str {
        match self {
            Self::UciOk => "uciok",
            Self::ReadyOk => "readyok",
            Self::BestMove(_) | Self::InvalidBestMove(_) => "bestmove",
            Self::Other(_) => "",
        }
    }

    /// Parses one engine stdout line into a protocol response.
    pub fn parse(line: &str) -> Self {
        match line {
            "uciok" => Self::UciOk,
            "readyok" => Self::ReadyOk,
            _ => match line.strip_prefix("bestmove ") {
                Some(rest) => {
                    let token = rest.split_whitespace().next().unwrap_or_default();
                    if token == "(none)" || token.is_empty() {
                        Self::BestMove(None)
                    } else {
                        match token.parse::<UciMove>() {
                            Ok(mv) => Self::BestMove(Some(mv)),
                            Err(_) => Self::InvalidBestMove(token.to_string()),
                        }
                    }
                }
                None => Self::Other(line.to_string()),
            },
        }
    }
}
