//! Captured-piece parking strips beside the board.

use crate::Zone;

/// Which side of the board a captured-piece parking strip sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapSide {
    /// Left of the board (from the white player's view).
    West,
    /// Right of the board.
    East,
}

impl CapSide {
    /// Parking cells per strip (board height / one square).
    pub const ROWS: u8 = 8;

    /// The daemon field name prefix of this strip (`cap-west` or
    /// `cap-east`).
    pub fn prefix(self) -> &'static str {
        match self {
            Self::West => "cap-west",
            Self::East => "cap-east",
        }
    }

    /// The daemon field name of cell `row` (`0` = board top).
    pub fn field_name(self, row: u8) -> String {
        Zone::Cap { side: self, row }.to_string()
    }

    /// Parses a `cap-{side}-{row}` field name into its side and row;
    /// `None` for anything that is not a parking cell.
    pub fn parse_field(name: &str) -> Option<(Self, u8)> {
        for side in [Self::West, Self::East] {
            if let Some(rest) = name.strip_prefix(side.prefix()).and_then(|rest| rest.strip_prefix('-')) {
                return rest.parse::<u8>().ok().filter(|row| *row < Self::ROWS).map(|row| (side, row));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::CapSide;

    #[test]
    fn field_names_round_trip() {
        for side in [CapSide::West, CapSide::East] {
            for row in 0..CapSide::ROWS {
                assert_eq!(CapSide::parse_field(&side.field_name(row)), Some((side, row)));
            }
        }
    }

    #[test]
    fn rejects_non_cap_names_and_bad_rows() {
        assert_eq!(CapSide::parse_field("f12"), None);
        assert_eq!(CapSide::parse_field("tray-south"), None);
        assert_eq!(CapSide::parse_field("cap-west-8"), None);
        assert_eq!(CapSide::parse_field("cap-east"), None);
    }
}
