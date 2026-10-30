//! A captured-piece parking strip beside the board.

use tangibles_chess_material::Piece;
use tangibles_chess_tangibles::CapSide;

use crate::ui::cap_cell::CapCell;

/// One parking strip beside the board (west or east): `CELLS`
/// top-to-bottom cells the size of a board square. The controller
/// tracks marker occupancy in it; `sync_scene` assigns captured pieces
/// to occupied cells for rendering.
#[derive(Debug, Clone, Copy, Default)]
pub struct ParkingStrip {
    /// Per-cell state (occupancy + rendered piece), row `0` = top.
    cells: [CapCell; Self::CELLS],
}

impl ParkingStrip {
    /// Cells per strip - matches `CapSide::ROWS` (the daemon field
    /// naming `cap-{side}-{row}`).
    pub const CELLS: usize = CapSide::ROWS as usize;

    /// Read-only view of all cells.
    pub fn cells(&self) -> &[CapCell; Self::CELLS] {
        &self.cells
    }

    /// Sets marker occupancy of `row` (`0` = top); out-of-range rows
    /// are ignored.
    pub fn set_occupied(&mut self, row: usize, occupied: bool) {
        if let Some(cell) = self.cells.get_mut(row) {
            cell.occupied = occupied;
        }
    }

    /// Assigns captured pieces to occupied cells in row order,
    /// consuming from `captured`; cells beyond the list render empty.
    pub fn assign_pieces(&mut self, captured: &mut impl Iterator<Item = Piece>) {
        for cell in &mut self.cells {
            cell.piece = if cell.occupied { captured.next() } else { None };
        }
    }

    /// Clears occupancy and rendered pieces (fresh resync).
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
