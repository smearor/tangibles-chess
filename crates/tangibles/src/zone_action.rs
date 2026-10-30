use crate::CapSide;
use crate::TableEdge;

/// A player action triggered via a physical marker on a zone field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneAction {
    /// The player confirmed their pending move (clock press).
    ConfirmMove {
        /// Which physical edge's zone was pressed.
        edge: TableEdge,
    },
    /// A piece was added to or removed from a captured tray; carries the
    /// current marker count of that tray.
    TrayCount {
        /// Which physical edge's tray changed.
        edge: TableEdge,
        /// Markers currently resting on the tray.
        count: u32,
    },
    /// A marker entered or left a captured-piece parking cell beside
    /// the board.
    CapCell {
        /// Which side's strip the cell belongs to.
        side: CapSide,
        /// Row within the strip (`0` = top of the board).
        row: u8,
        /// Whether the cell now holds a marker.
        occupied: bool,
    },
}
