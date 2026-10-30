//! A square's notation name (`"e2"`).

use serde::Deserialize;
use serde::Serialize;
use shakmaty::Square;

/// The notation name of a board square (`"e2"`, `"h8"`) - the string
/// form used on reply wires, not a coordinate type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SquareName(String);

impl SquareName {
    /// The raw name string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SquareName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<Square> for SquareName {
    /// The name of `square` (`"e2"`).
    fn from(square: Square) -> Self {
        Self(square.to_string())
    }
}
