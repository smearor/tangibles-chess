//! Named game sound effects.

/// A named game sound effect - the `Display` name is the gresource
/// asset stem (`"check"` → `sounds/check.ogg`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundEffect {
    /// A piece was moved.
    Move,
    /// A piece was captured.
    Capture,
    /// The opponent's king is in check.
    Check,
    /// The game ended.
    End,
}

impl std::fmt::Display for SoundEffect {
    /// The gresource asset name (`"move"`, `"capture"`, ...).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Move => "move",
            Self::Capture => "capture",
            Self::Check => "check",
            Self::End => "end",
        })
    }
}
