//! A canonical board-layout index.

use crate::FieldIndex;
use shakmaty::File;
use shakmaty::Rank;
use shakmaty::Square;

/// A canonical board-layout index `0..63` (`a8` = 0 .. `h1` = 63,
/// row-major) - orientation-independent scene space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SceneIndex(usize);

impl SceneIndex {
    /// Wraps `index`; `None` outside `0..64`.
    pub fn new(index: usize) -> Option<Self> {
        (index < 64).then_some(Self(index))
    }

    /// The raw index.
    pub fn as_usize(self) -> usize {
        self.0
    }
}

impl From<Square> for SceneIndex {
    /// The scene index of `square`.
    fn from(square: Square) -> Self {
        let index = square as usize;
        let (file, rank) = (index % 8, index / 8);
        Self((7 - rank) * 8 + file)
    }
}

impl From<SceneIndex> for Square {
    /// The square of the scene position.
    fn from(index: SceneIndex) -> Self {
        let file = File::new((index.0 % 8) as u32);
        let rank = Rank::new((7 - index.0 / 8) as u32);
        Square::from_coords(file, rank)
    }
}

impl From<FieldIndex> for SceneIndex {
    /// Same grid index, canonical frame - field and scene indices
    /// share the `0..64` domain.
    fn from(index: FieldIndex) -> Self {
        Self(index.as_usize())
    }
}

#[cfg(test)]
mod tests {
    use super::SceneIndex;
    use shakmaty::Square;

    #[test]
    fn square_round_trip() {
        for square in Square::ALL {
            assert_eq!(Square::from(SceneIndex::from(square)), square);
        }
    }

    #[test]
    fn known_positions() {
        assert_eq!(SceneIndex::from(Square::A8).as_usize(), 0);
        assert_eq!(SceneIndex::from(Square::H1).as_usize(), 63);
        assert_eq!(Square::from(SceneIndex::new(0).expect("in range")), Square::A8);
        assert_eq!(Square::from(SceneIndex::new(63).expect("in range")), Square::H1);
    }
}
