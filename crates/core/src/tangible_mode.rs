use serde::Deserialize;
use serde::Serialize;

/// Input mode of a game: physical markers, markers plus rendering, or
/// pure touch play.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TangibleMode {
    /// Physical markers drive the game; the screen renders no piece
    /// figures - only the board, light guidance and panels.
    Tangibles,
    /// Physical markers drive the game and pieces are rendered as a
    /// control layer with discrepancy display.
    TangiblesRendered,
    /// No markers; moves are made by touch (tap-tap or drag) on the
    /// rendered board.
    #[default]
    NoTangibles,
}

impl TangibleMode {
    /// Whether pieces are rendered in this mode.
    pub fn renders_pieces(self) -> bool {
        !matches!(self, Self::Tangibles)
    }

    /// Whether this mode needs the tangible-objects daemon connection.
    pub fn uses_tangibles(self) -> bool {
        !matches!(self, Self::NoTangibles)
    }
}

#[cfg(test)]
mod tests {
    use super::TangibleMode;

    #[test]
    fn mode_capabilities_are_consistent() {
        assert!(!TangibleMode::Tangibles.renders_pieces());
        assert!(TangibleMode::TangiblesRendered.renders_pieces());
        assert!(TangibleMode::NoTangibles.renders_pieces());
        assert!(TangibleMode::Tangibles.uses_tangibles());
        assert!(TangibleMode::TangiblesRendered.uses_tangibles());
        assert!(!TangibleMode::NoTangibles.uses_tangibles());
    }
}
