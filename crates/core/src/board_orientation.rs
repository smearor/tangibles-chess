use serde::Deserialize;
use serde::Serialize;

/// Orientation of the rendered board on the tabletop screen.
///
/// Snapped quarter-turns applied by the `RotationWidget`; the value says
/// where the white home edge sits relative to the physical south edge of
/// the screen.
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
        match self {
            Self::Deg0 => Self::Deg90,
            Self::Deg90 => Self::Deg180,
            Self::Deg180 => Self::Deg270,
            Self::Deg270 => Self::Deg0,
        }
    }

    /// The orientation rotated by one quarter-turn counter-clockwise.
    pub fn counter_clockwise(self) -> Self {
        match self {
            Self::Deg0 => Self::Deg270,
            Self::Deg90 => Self::Deg0,
            Self::Deg180 => Self::Deg90,
            Self::Deg270 => Self::Deg180,
        }
    }

    /// Rotation angle in degrees.
    pub fn degrees(self) -> u16 {
        match self {
            Self::Deg0 => 0,
            Self::Deg90 => 90,
            Self::Deg180 => 180,
            Self::Deg270 => 270,
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
}
