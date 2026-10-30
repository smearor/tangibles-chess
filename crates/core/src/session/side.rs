//! One of the two player colors.

use serde::Deserialize;
use serde::Serialize;
use shakmaty::Color;

/// One of the two player colors - the domain spelling used on the wire
/// (`"white"`/`"black"` in saves and MCP arguments) instead of raw
/// strings or `shakmaty::Color` leaking through boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// The white side.
    White,
    /// The black side.
    Black,
}

impl Side {
    /// The other side.
    pub fn other(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }

    /// The CSS class styling a player widget in this side's color.
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::White => "panel-white",
            Self::Black => "panel-black",
        }
    }
}

impl std::fmt::Display for Side {
    /// `"white"`/`"black"` (matching `shakmaty::Color`'s spelling).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::White => "white",
            Self::Black => "black",
        })
    }
}

impl std::str::FromStr for Side {
    type Err = String;

    /// Parses `"white"`/`"black"` (case-insensitive).
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        if name.eq_ignore_ascii_case("white") {
            Ok(Self::White)
        } else if name.eq_ignore_ascii_case("black") {
            Ok(Self::Black)
        } else {
            Err(name.to_string())
        }
    }
}

impl From<Side> for Color {
    fn from(side: Side) -> Self {
        match side {
            Side::White => Self::White,
            Side::Black => Self::Black,
        }
    }
}

impl From<Color> for Side {
    fn from(color: Color) -> Self {
        match color {
            Color::White => Self::White,
            Color::Black => Self::Black,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Side;
    use shakmaty::Color;

    #[test]
    fn converts_both_ways() {
        assert_eq!(Side::from(Color::White), Side::White);
        assert_eq!(Side::from(Color::Black), Side::Black);
        assert_eq!(Color::from(Side::White), Color::White);
        assert_eq!(Color::from(Side::Black), Color::Black);
    }

    #[test]
    fn display_round_trips_through_parse() {
        for side in [Side::White, Side::Black] {
            assert_eq!(side.to_string().parse::<Side>(), Ok(side));
        }
        assert!("green".parse::<Side>().is_err());
    }
}
