use serde::Deserialize;
use serde::Serialize;

use crate::BoardOrientation;
use crate::BoardTheme;
use crate::CommitMode;
use crate::OpponentKind;
use crate::TangibleMode;

/// Static configuration chosen on the setup screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameConfig {
    /// Tangible input mode of the game.
    pub tangible_mode: TangibleMode,
    /// When tangible moves are committed.
    pub commit_mode: CommitMode,
    /// Board material theme.
    pub theme: BoardTheme,
    /// Initial board orientation on the desk.
    pub orientation: BoardOrientation,
    /// Who plays the white side.
    pub white: OpponentKind,
    /// Who plays the black side.
    pub black: OpponentKind,
    /// Whether sound effects are enabled.
    pub sound_enabled: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            tangible_mode: TangibleMode::default(),
            commit_mode: CommitMode::default(),
            theme: BoardTheme::default(),
            orientation: BoardOrientation::default(),
            white: OpponentKind::Human,
            black: OpponentKind::Human,
            sound_enabled: true,
        }
    }
}
