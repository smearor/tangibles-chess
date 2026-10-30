use shakmaty::Square;
use tangibles_chess_core::BoardOrientation;
use tangibles_chess_core::FieldIndex;

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

    /// The logical square occupied by screen field `index`.
    ///
    /// Scene space always lays out `a8..h1` row-major; a clockwise scene
    /// rotation by θ maps screen index `i` to scene index `rot_ccw_θ(i)`.
    pub fn field_to_square(&self, index: FieldIndex) -> Square {
        Square::from(self.orientation.scene_index(index))
    }

    /// The screen field index that currently shows `square`.
    pub fn square_to_field(&self, square: Square) -> Option<FieldIndex> {
        // Invert by scanning: 64 entries, trivially cheap.
        (0..FieldIndex::COUNT)
            .filter_map(FieldIndex::new)
            .find(|index| self.field_to_square(*index) == square)
    }

    /// The edge assigned to White.
    pub fn white_edge(&self) -> TableEdge {
        self.white_edge
    }

    /// The edge assigned to Black (always opposite White in this game).
    pub fn black_edge(&self) -> TableEdge {
        self.white_edge.opposite()
    }

    /// The board edge `side` plays from.
    pub fn side_edge(&self, side: tangibles_chess_material::Side) -> TableEdge {
        match side {
            tangibles_chess_material::Side::White => self.white_edge(),
            tangibles_chess_material::Side::Black => self.black_edge(),
        }
    }
}

#[cfg(test)]
mod tests {
    use shakmaty::Square;
    use tangibles_chess_core::BoardOrientation;
    use tangibles_chess_core::FieldIndex;

    use super::BoardGeometry;
    use crate::TableEdge;

    fn index(i: usize) -> FieldIndex {
        FieldIndex::new(i).expect("in range")
    }

    #[test]
    fn deg0_maps_top_left_to_a8() {
        let geometry = BoardGeometry::tabletop();
        assert_eq!(geometry.field_to_square(index(0)), Square::A8);
        assert_eq!(geometry.field_to_square(index(7)), Square::H8);
        assert_eq!(geometry.field_to_square(index(56)), Square::A1);
        assert_eq!(geometry.field_to_square(index(63)), Square::H1);
    }

    #[test]
    fn deg180_mirrors_the_board() {
        let geometry = BoardGeometry::new(BoardOrientation::Deg180, TableEdge::South);
        assert_eq!(geometry.field_to_square(index(0)), Square::H1);
        assert_eq!(geometry.field_to_square(index(63)), Square::A8);
    }

    #[test]
    fn round_trips_all_squares_in_all_orientations() {
        for orientation in BoardOrientation::ALL {
            let geometry = BoardGeometry::new(orientation, TableEdge::South);
            for index in (0..64).filter_map(FieldIndex::new) {
                let square = geometry.field_to_square(index);
                assert_eq!(geometry.square_to_field(square), Some(index));
            }
        }
    }

    #[test]
    fn field_name_roundtrip() {
        assert_eq!(index(5).to_string(), "f5");
        assert_eq!("f5".parse::<FieldIndex>(), Ok(index(5)));
        assert!("btn-south".parse::<FieldIndex>().is_err());
        assert!("f64".parse::<FieldIndex>().is_err());
    }
}
