use shakmaty::File;
use shakmaty::Rank;
use shakmaty::Square;
use tangibles_chess_core::BoardOrientation;

use crate::TableEdge;

/// Maps physical occupancy fields onto logical chess squares.
///
/// Board fields are screen-fixed (`f0` = top-left cell of the board
/// rectangle in screen coordinates, row-major to `f63`). The rendered
/// board may be rotated by the `RotationWidget`; `orientation` permutes
/// field indices onto squares so a resting marker keeps its identity.
#[derive(Debug, Clone, Copy)]
pub struct BoardGeometry {
    /// Current render orientation of the board surface.
    orientation: BoardOrientation,
    /// Physical edge where the white player sits (used to decide which
    /// `btn-*`/`tray-*` zone belongs to whom).
    white_edge: TableEdge,
}

impl BoardGeometry {
    /// A geometry with white sitting at `white_edge`.
    pub fn new(orientation: BoardOrientation, white_edge: TableEdge) -> Self {
        Self { orientation, white_edge }
    }

    /// Default tabletop layout: unrotated board, white at the south edge.
    pub fn tabletop() -> Self {
        Self::new(BoardOrientation::Deg0, TableEdge::South)
    }

    /// The field name of the field index (`f0`..`f63`).
    pub fn field_name(index: usize) -> Option<String> {
        (index < 64).then(|| format!("f{index}"))
    }

    /// Parses a board field name (`f0`..`f63`) into its index.
    /// Zone names (`btn-*`, `tray-*`) return `None`.
    pub fn field_index(name: &str) -> Option<usize> {
        name.strip_prefix('f')?.parse::<usize>().ok().filter(|i| *i < 64)
    }

    /// Rotates a grid index one quarter-turn clockwise (screen space,
    /// row-major 8×8).
    fn rot_cw(index: usize) -> usize {
        let (row, col) = (index / 8, index % 8);
        col * 8 + (7 - row)
    }

    /// The logical square occupied by screen field `index`.
    ///
    /// Scene space always lays out `a8..h1` row-major; a clockwise scene
    /// rotation by θ maps screen index `i` to scene index `rot_ccw_θ(i)`.
    pub fn field_to_square(&self, index: usize) -> Option<Square> {
        if index >= 64 {
            return None;
        }
        let scene = match self.orientation {
            BoardOrientation::Deg0 => index,
            BoardOrientation::Deg90 => (0..3).fold(index, |i, _| Self::rot_cw(i)),
            BoardOrientation::Deg180 => 63 - index,
            BoardOrientation::Deg270 => Self::rot_cw(index),
        };
        let file = File::new((scene % 8) as u32);
        let rank = Rank::new((7 - scene / 8) as u32);
        Some(Square::from_coords(file, rank))
    }

    /// The screen field index that currently shows `square`.
    pub fn square_to_field(&self, square: Square) -> usize {
        // Invert by scanning: 64 entries, trivially cheap.
        (0..64).find(|index| self.field_to_square(*index) == Some(square)).unwrap_or(usize::MAX)
    }

    /// The `btn-*` confirm zone name for the player at `edge`.
    pub fn confirm_zone(edge: TableEdge) -> &'static str {
        match edge {
            TableEdge::South => crate::ZONE_BTN_SOUTH,
            TableEdge::North => crate::ZONE_BTN_NORTH,
            TableEdge::East => crate::ZONE_BTN_SOUTH,
            TableEdge::West => crate::ZONE_BTN_NORTH,
        }
    }

    /// The edge assigned to White.
    pub fn white_edge(&self) -> TableEdge {
        self.white_edge
    }

    /// The edge assigned to Black (always opposite White in this game).
    pub fn black_edge(&self) -> TableEdge {
        self.white_edge.opposite()
    }
}

#[cfg(test)]
mod tests {
    use shakmaty::Square;
    use tangibles_chess_core::BoardOrientation;

    use super::BoardGeometry;
    use crate::TableEdge;

    #[test]
    fn deg0_maps_top_left_to_a8() {
        let geometry = BoardGeometry::tabletop();
        assert_eq!(geometry.field_to_square(0), Some(Square::A8));
        assert_eq!(geometry.field_to_square(7), Some(Square::H8));
        assert_eq!(geometry.field_to_square(56), Some(Square::A1));
        assert_eq!(geometry.field_to_square(63), Some(Square::H1));
    }

    #[test]
    fn deg180_mirrors_the_board() {
        let geometry = BoardGeometry::new(BoardOrientation::Deg180, TableEdge::South);
        assert_eq!(geometry.field_to_square(0), Some(Square::H1));
        assert_eq!(geometry.field_to_square(63), Some(Square::A8));
    }

    #[test]
    fn round_trips_all_squares_in_all_orientations() {
        for orientation in BoardOrientation::ALL {
            let geometry = BoardGeometry::new(orientation, TableEdge::South);
            for index in 0..64 {
                let square = geometry.field_to_square(index).expect("square");
                assert_eq!(geometry.square_to_field(square), index);
            }
        }
    }

    #[test]
    fn field_name_roundtrip() {
        assert_eq!(BoardGeometry::field_name(5).as_deref(), Some("f5"));
        assert_eq!(BoardGeometry::field_index("f5"), Some(5));
        assert_eq!(BoardGeometry::field_index("btn-south"), None);
        assert_eq!(BoardGeometry::field_name(64), None);
    }
}
