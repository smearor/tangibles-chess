use serde::Deserialize;
use serde::Serialize;

/// A physical edge of the desk where a player sits.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TableEdge {
    /// Near edge.
    #[default]
    South,
    /// Right edge.
    East,
    /// Far edge.
    North,
    /// Left edge.
    West,
}

impl TableEdge {
    /// The opposite edge.
    pub fn opposite(self) -> Self {
        match self {
            Self::South => Self::North,
            Self::North => Self::South,
            Self::East => Self::West,
            Self::West => Self::East,
        }
    }
}
