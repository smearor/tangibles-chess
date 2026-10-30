//! A move in UCI notation.

use serde::Deserialize;
use serde::Serialize;
use shakmaty::CastlingMode;
use shakmaty::Move;
use shakmaty::uci::ParseUciMoveError;
use shakmaty::uci::UciMove;

/// A move in UCI notation (`e2e4`, `e7e8q`) - the autosave/replay
/// format, not a validated value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Uci(String);

impl Uci {
    /// The UCI of `mv` (standard castling notation `e1g1`).
    pub fn from_move(mv: &Move) -> Self {
        Self(mv.to_uci(CastlingMode::Standard).to_string())
    }

    /// The raw notation string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Uci {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Uci {
    fn from(notation: &str) -> Self {
        Self(notation.to_string())
    }
}

impl From<String> for Uci {
    fn from(notation: String) -> Self {
        Self(notation)
    }
}

impl TryFrom<&Uci> for UciMove {
    type Error = ParseUciMoveError;

    /// Parses the UCI notation (fails on malformed strings).
    fn try_from(uci: &Uci) -> Result<Self, Self::Error> {
        uci.0.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::Uci;
    use shakmaty::Chess;
    use shakmaty::uci::UciMove;

    use std::str::FromStr;

    #[test]
    fn from_move_produces_notation() {
        let position = Chess::default();
        let mv = UciMove::from_str("e2e4").expect("uci").to_move(&position).expect("legal");
        assert_eq!(Uci::from_move(&mv).as_str(), "e2e4");
    }
}
