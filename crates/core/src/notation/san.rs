//! A move in Standard Algebraic Notation.

use serde::Deserialize;
use serde::Serialize;
use shakmaty::Chess;
use shakmaty::Move;
use shakmaty::san::SanPlus;

/// A move in Standard Algebraic Notation (`e4`, `Nf3`, `O-O`,
/// `exd8=Q+`) - display text, not a parsed value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct San(String);

impl San {
    /// The SAN of `mv` in `position` (evaluated before it is played).
    pub fn from_move(position: Chess, mv: Move) -> Self {
        Self(SanPlus::from_move(position, mv).to_string())
    }

    /// The raw notation string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for San {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for San {
    fn from(notation: &str) -> Self {
        Self(notation.to_string())
    }
}

impl From<String> for San {
    fn from(notation: String) -> Self {
        Self(notation)
    }
}

#[cfg(test)]
mod tests {
    use super::San;
    use shakmaty::Chess;
    use shakmaty::uci::UciMove;

    use std::str::FromStr;

    #[test]
    fn from_move_produces_notation() {
        let position = Chess::default();
        let mv = UciMove::from_str("e2e4").expect("uci").to_move(&position).expect("legal");
        assert_eq!(San::from_move(position, mv).as_str(), "e4");
    }
}
