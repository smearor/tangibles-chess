//! A physical screen-grid position.

use smearor_commons_geometry::Cell;
use smearor_commons_geometry::Col;
use smearor_commons_geometry::Grid;
use smearor_commons_geometry::QuarterTurn;
use smearor_commons_geometry::Row;

/// The board's fixed 8×8 partition — physical screen space, row-major.
pub const BOARD_GRID: Grid = Grid::new(8, 8);

/// A physical screen-grid position `0..63` - row-major from the board
/// rectangle's top-left corner (the daemon's `f{index}` fields).
///
/// The position is screen-fixed: `f0` is the top-left cell regardless
/// of board rotation. Contrast [`crate::SceneIndex`], which counts
/// through the canonical `a8`..`h1` layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldIndex(usize);

impl FieldIndex {
    /// Number of grid cells.
    pub const COUNT: usize = 64;
    /// Row length of the grid.
    pub const ROW: usize = 8;

    /// Wraps `index`; `None` outside `0..64`.
    pub fn new(index: usize) -> Option<Self> {
        (index < Self::COUNT).then_some(Self(index))
    }

    /// The raw index.
    pub fn as_usize(self) -> usize {
        self.0
    }

    /// The [`Cell`] of this field in [`BOARD_GRID`].
    ///
    /// `index < COUNT` is the type invariant, so the row-major split is
    /// total here — [`Grid::cell`] is only partial because it accepts
    /// unbounded indices.
    pub fn cell(self) -> Cell {
        Cell::new(Col(self.0 as u32 % BOARD_GRID.cols()), Row(self.0 as u32 / BOARD_GRID.cols()))
    }

    /// Row (`0` = top of the screen).
    pub fn row(self) -> usize {
        self.cell().row.0 as usize
    }

    /// Column (`0` = left of the screen).
    pub fn col(self) -> usize {
        self.cell().col.0 as usize
    }

    /// The field rotated by `turn` within [`BOARD_GRID`].
    ///
    /// Rotation permutes cells inside the transposed grid; on the
    /// square board the rotated cell is always in range, so mapping
    /// back to a linear index is total.
    pub fn rotated(self, turn: QuarterTurn) -> Self {
        let cell = self.cell().rotated(turn, &BOARD_GRID);
        Self(cell.row.0 as usize * Self::ROW + cell.col.0 as usize)
    }

    /// One quarter-turn clockwise (screen space).
    pub fn rot_cw(self) -> Self {
        self.rotated(QuarterTurn::Deg90)
    }

    /// One quarter-turn counter-clockwise (the inverse of `rot_cw`).
    pub fn rot_ccw(self) -> Self {
        self.rotated(QuarterTurn::Deg270)
    }

    /// A half-turn (180°).
    pub fn rot_180(self) -> Self {
        self.rotated(QuarterTurn::Deg180)
    }
}

impl std::fmt::Display for FieldIndex {
    /// The daemon field name (`f{index}`).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "f{}", self.0)
    }
}

impl std::str::FromStr for FieldIndex {
    type Err = String;

    /// Parses the daemon field name `f{index}`; rejects anything else.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        name.strip_prefix('f')
            .and_then(|rest| rest.parse::<usize>().ok())
            .and_then(Self::new)
            .ok_or_else(|| name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::FieldIndex;
    use std::str::FromStr;

    #[test]
    fn rotations_are_each_others_inverse() {
        for index in 0..FieldIndex::COUNT {
            let index = FieldIndex::new(index).expect("in range");
            assert_eq!(index.rot_cw().rot_ccw(), index);
            assert_eq!(index.rot_ccw().rot_cw(), index);
            assert_eq!(index.rot_180().rot_180(), index);
            assert_eq!(index.rot_cw().rot_cw(), index.rot_180());
        }
    }

    #[test]
    fn cell_round_trips_through_row_and_col() {
        for index in 0..FieldIndex::COUNT {
            let index = FieldIndex::new(index).expect("in range");
            let cell = index.cell();
            assert_eq!(index.row(), cell.row.0 as usize);
            assert_eq!(index.col(), cell.col.0 as usize);
            assert_eq!(index.as_usize(), index.row() * FieldIndex::ROW + index.col());
        }
    }

    #[test]
    fn display_round_trips_through_parse() {
        for index in 0..FieldIndex::COUNT {
            let index = FieldIndex::new(index).expect("in range");
            assert_eq!(FieldIndex::from_str(&index.to_string()), Ok(index));
        }
        assert!(FieldIndex::from_str("f64").is_err());
        assert!(FieldIndex::from_str("btn-south").is_err());
    }
}
