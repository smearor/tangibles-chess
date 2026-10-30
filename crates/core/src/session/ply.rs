//! A position on the half-move axis.

/// A position on the half-move axis: [`Ply::START`] is the game's
/// initial position and `Ply(n)` the state after the n-th half-move.
/// As a move ordinal it names the n-th half-move of the played
/// sequence (0-based).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ply(usize);

impl Ply {
    /// The start position (before any half-move).
    pub const START: Self = Self(0);

    /// The raw 0-based index.
    pub fn index(self) -> usize {
        self.0
    }

    /// The position after this ply.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }

    /// The position before this ply (`None` at [`Ply::START`]).
    pub fn prev(self) -> Option<Self> {
        self.0.checked_sub(1).map(Self)
    }

    /// The position `delta` plies away, clamped at [`Ply::START`].
    pub fn step(self, delta: i64) -> Self {
        Self(self.0.saturating_add_signed(delta as isize))
    }

    /// `true` at the start position.
    pub fn is_start(self) -> bool {
        self.0 == 0
    }
}

impl std::fmt::Display for Ply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for Ply {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

impl From<Ply> for usize {
    fn from(ply: Ply) -> Self {
        ply.0
    }
}

#[cfg(test)]
mod tests {
    use super::Ply;

    #[test]
    fn steps_along_the_axis() {
        assert_eq!(Ply::START.next(), Ply::from(1));
        assert_eq!(Ply::from(3).prev(), Some(Ply::from(2)));
        assert_eq!(Ply::START.prev(), None);
        assert_eq!(Ply::from(2).step(-5), Ply::START);
        assert_eq!(Ply::from(2).step(5), Ply::from(7));
    }
}
