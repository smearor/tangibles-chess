/// What a flashed square explains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlashRole {
    /// The rejected destination.
    Target,
    /// The own king left in check.
    King,
    /// An attacker giving that check.
    Attacker,
}
