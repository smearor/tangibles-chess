use crate::FieldIndex;
use crate::SceneIndex;
use serde::Deserialize;
use serde::Serialize;
use smearor_commons_geometry::QuarterTurn;

/// Orientation of the rendered board on the tabletop screen.
///
/// Snapped quarter-turns applied by the `RotationWidget`; the value says
/// where the white home edge sits relative to the physical south edge of
/// the screen. The rotation algebra delegates to [`QuarterTurn`]; this
/// enum carries the grid semantics (`scene_index`) and the wire shape.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BoardOrientation {
    /// White home rank faces the physical south edge.
    #[default]
    Deg0,
    /// White home rank faces the physical west edge.
    Deg90,
    /// White home rank faces the physical north edge.
    Deg180,
    /// White home rank faces the physical east edge.
    Deg270,
}

impl BoardOrientation {
    /// All orientations in clockwise order.
    pub const ALL: [Self; 4] = [Self::Deg0, Self::Deg90, Self::Deg180, Self::Deg270];

    /// The orientation rotated by one quarter-turn clockwise.
    pub fn clockwise(self) -> Self {
        QuarterTurn::from(self).clockwise().into()
    }

    /// The orientation rotated by one quarter-turn counter-clockwise.
    pub fn counter_clockwise(self) -> Self {
        QuarterTurn::from(self).counter_clockwise().into()
    }

    /// Rotation angle in degrees.
    pub fn degrees(self) -> u16 {
        QuarterTurn::from(self).degrees() as u16
    }

    /// Maps a physical screen field to the canonical scene index it
    /// currently holds under this orientation: the orientation's
    /// inverse rotation applied to the field index.
    pub fn scene_index(self, index: FieldIndex) -> SceneIndex {
        let rotated = match self {
            Self::Deg0 => index,
            Self::Deg90 => index.rot_ccw(),
            Self::Deg180 => index.rot_180(),
            Self::Deg270 => index.rot_cw(),
        };
        rotated.into()
    }
}

/// The board orientation as a technical [`QuarterTurn`].
impl From<BoardOrientation> for QuarterTurn {
    fn from(orientation: BoardOrientation) -> Self {
        match orientation {
            BoardOrientation::Deg0 => Self::Deg0,
            BoardOrientation::Deg90 => Self::Deg90,
            BoardOrientation::Deg180 => Self::Deg180,
            BoardOrientation::Deg270 => Self::Deg270,
        }
    }
}

/// A technical [`QuarterTurn`] as a board orientation (identical
/// rotation, different domain semantics).
impl From<QuarterTurn> for BoardOrientation {
    fn from(turn: QuarterTurn) -> Self {
        match turn {
            QuarterTurn::Deg0 => Self::Deg0,
            QuarterTurn::Deg90 => Self::Deg90,
            QuarterTurn::Deg180 => Self::Deg180,
            QuarterTurn::Deg270 => Self::Deg270,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BoardOrientation;

    #[test]
    fn clockwise_cycles_through_all_orientations() {
        let mut orientation = BoardOrientation::Deg0;
        let mut seen = Vec::new();
        for _ in 0..4 {
            seen.push(orientation);
            orientation = orientation.clockwise();
        }
        assert_eq!(orientation, BoardOrientation::Deg0);
        assert_eq!(seen, BoardOrientation::ALL.to_vec());
    }

    #[test]
    fn counter_clockwise_inverts_clockwise() {
        for orientation in BoardOrientation::ALL {
            assert_eq!(orientation.clockwise().counter_clockwise(), orientation);
            assert_eq!(orientation.counter_clockwise().clockwise(), orientation);
        }
    }

    #[test]
    fn scene_index_rotates_the_grid() {
        use crate::FieldIndex;
        let index = |i| FieldIndex::new(i).expect("in range");
        // Top-left corner under each orientation.
        assert_eq!(BoardOrientation::Deg0.scene_index(index(0)).as_usize(), 0);
        assert_eq!(BoardOrientation::Deg90.scene_index(index(0)).as_usize(), 56);
        assert_eq!(BoardOrientation::Deg180.scene_index(index(0)).as_usize(), 63);
        assert_eq!(BoardOrientation::Deg270.scene_index(index(0)).as_usize(), 7);
        // A center field keeps a symmetric partner under 180°.
        assert_eq!(BoardOrientation::Deg180.scene_index(index(27)).as_usize(), 36);
    }

    #[test]
    fn scene_index_is_a_bijection() {
        use crate::FieldIndex;
        for orientation in BoardOrientation::ALL {
            let mut seen = [false; 64];
            for i in 0..64 {
                let index = FieldIndex::new(i).expect("in range");
                let mapped = orientation.scene_index(index).as_usize();
                assert!(!seen[mapped], "collision at {mapped}");
                seen[mapped] = true;
            }
        }
    }
}
