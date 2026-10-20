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
}
