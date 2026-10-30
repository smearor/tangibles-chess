//! A position in Forsyth-Edwards Notation.

use serde::Deserialize;
use serde::Serialize;
use shakmaty::Chess;
use shakmaty::EnPassantMode;

/// A position in Forsyth-Edwards Notation - display text, not a
/// parsed value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Fen(String);

impl Fen {
    /// The FEN of `position` (legal en-passant mode).
    pub fn from_position(position: &Chess) -> Self {
        Self(shakmaty::fen::Fen::from_position(position, EnPassantMode::Legal).to_string())
    }

    /// The raw notation string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Fen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Fen {
    fn from(notation: &str) -> Self {
        Self(notation.to_string())
    }
}

impl From<String> for Fen {
    fn from(notation: String) -> Self {
        Self(notation)
    }
}

impl TryFrom<&Fen> for shakmaty::fen::Fen {
    type Error = shakmaty::fen::ParseFenError;

    /// Parses the FEN notation (fails on malformed strings).
    fn try_from(fen: &Fen) -> Result<Self, Self::Error> {
        fen.0.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::Fen;
    use shakmaty::Chess;

    #[test]
    fn from_position_produces_the_startpos_fen() {
        assert_eq!(Fen::from_position(&Chess::default()).as_str(), "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    }
}
