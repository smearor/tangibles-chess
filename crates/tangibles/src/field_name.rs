//! A registered field-set field name.

use serde::Deserialize;
use std::str::FromStr;
use tangibles_chess_core::FieldIndex;

use crate::Zone;

/// The name of one field in the registered `chess` field set: a board
/// square field (`f0`..`f63`) or a zone (`btn-*`, `tray-*`, `cap-*`).
///
/// Names the chess field set does not define keep their raw string in
/// [`FieldName::Unknown`] so decoding never fails on foreign field
/// sets sharing the subscription.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldName {
    /// A board field (`f{index}`).
    Board(FieldIndex),
    /// A zone field (`btn-*`, `tray-*`, `cap-{side}-{row}`).
    Zone(Zone),
    /// A name outside the `f*`/`btn-*`/`tray-*`/`cap-*` vocabulary.
    Unknown(String),
}

impl std::fmt::Display for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Board(index) => index.fmt(f),
            Self::Zone(zone) => zone.fmt(f),
            Self::Unknown(name) => f.write_str(name),
        }
    }
}

impl From<&str> for FieldName {
    fn from(name: &str) -> Self {
        name.parse::<FieldIndex>()
            .map(Self::Board)
            .or_else(|_| Zone::from_str(name).map(Self::Zone))
            .unwrap_or_else(|_| Self::Unknown(name.to_string()))
    }
}

impl From<String> for FieldName {
    fn from(name: String) -> Self {
        Self::from(name.as_str())
    }
}

impl<'de> Deserialize<'de> for FieldName {
    /// Never fails: unrecognised names decode into `Unknown`.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::from(String::deserialize(deserializer)?))
    }
}

#[cfg(test)]
mod tests {
    use super::FieldName;
    use tangibles_chess_core::FieldIndex;

    use crate::CapSide;
    use crate::Zone;

    #[test]
    fn parses_board_and_zone_and_keeps_unknowns() {
        assert_eq!(FieldName::from("f7"), FieldName::Board(FieldIndex::new(7).expect("in range")));
        assert_eq!(FieldName::from("btn-south"), FieldName::Zone(Zone::BtnSouth));
        assert_eq!(FieldName::from("cap-west-3"), FieldName::Zone(Zone::Cap { side: CapSide::West, row: 3 }));
        assert_eq!(FieldName::from("x9"), FieldName::Unknown("x9".to_string()));
    }

    #[test]
    fn display_round_trips_known_names() {
        for name in ["f7", "btn-north", "tray-south", "cap-east-7"] {
            assert_eq!(FieldName::from(name).to_string(), name);
        }
    }
}
