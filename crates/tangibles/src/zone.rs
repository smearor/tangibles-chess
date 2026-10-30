//! Named non-board fields of the chess field set.

use crate::CapSide;
use crate::TableEdge;

/// A named zone of the chess field set: confirm strips, captured
/// trays and single parking cells. `Display` produces the daemon
/// field name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Zone {
    /// South-side confirm zone (`btn-south`).
    BtnSouth,
    /// North-side confirm zone (`btn-north`).
    BtnNorth,
    /// South-side captured tray (`tray-south`).
    TraySouth,
    /// North-side captured tray (`tray-north`).
    TrayNorth,
    /// One captured-piece parking cell (`cap-{west|east}-{row}`).
    Cap {
        /// Which side's strip the cell belongs to.
        side: CapSide,
        /// Row within the strip (`0` = top of the board).
        row: u8,
    },
}

impl Zone {
    /// The table edge this zone conceptually belongs to (parking cells
    /// map to their side).
    pub fn edge(self) -> TableEdge {
        match self {
            Self::BtnSouth | Self::TraySouth => TableEdge::South,
            Self::BtnNorth | Self::TrayNorth => TableEdge::North,
            Self::Cap { side: CapSide::West, .. } => TableEdge::West,
            Self::Cap { side: CapSide::East, .. } => TableEdge::East,
        }
    }
}

impl std::fmt::Display for Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BtnSouth => f.write_str("btn-south"),
            Self::BtnNorth => f.write_str("btn-north"),
            Self::TraySouth => f.write_str("tray-south"),
            Self::TrayNorth => f.write_str("tray-north"),
            Self::Cap { side, row } => write!(f, "{}-{row}", side.prefix()),
        }
    }
}

impl std::str::FromStr for Zone {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "btn-south" => return Ok(Self::BtnSouth),
            "btn-north" => return Ok(Self::BtnNorth),
            "tray-south" => return Ok(Self::TraySouth),
            "tray-north" => return Ok(Self::TrayNorth),
            _ => {}
        }
        if let Some((side, row)) = CapSide::parse_field(name) {
            return Ok(Self::Cap { side, row });
        }
        Err(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::Zone;
    use crate::CapSide;
    use std::str::FromStr;

    #[test]
    fn display_round_trips_through_parse() {
        for zone in [
            Zone::BtnSouth,
            Zone::BtnNorth,
            Zone::TraySouth,
            Zone::TrayNorth,
            Zone::Cap { side: CapSide::West, row: 0 },
            Zone::Cap { side: CapSide::East, row: 7 },
        ] {
            assert_eq!(Zone::from_str(&zone.to_string()), Ok(zone));
        }
    }

    #[test]
    fn parse_rejects_board_fields_and_bad_rows() {
        assert!(Zone::from_str("f12").is_err());
        assert!(Zone::from_str("cap-west-8").is_err());
        assert!(Zone::from_str("cap-east").is_err());
    }
}
