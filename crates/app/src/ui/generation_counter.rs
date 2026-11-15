/// Monotonic stamp for transient scene state: a delayed expiry
/// callback only clears the generation it was scheduled against, so a
/// stale timer cannot erase a newer entry.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GenerationCounter(u64);

impl GenerationCounter {
    /// The next stamp - every timer scheduled for an older generation
    /// no longer matches.
    pub fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}
