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
    /// Whether tangible games require the full placement handshake
    /// (all 32 pieces on their starting squares) before the game
    /// begins. Disable to debug without a complete physical setup.
    pub require_placement: bool,
    /// Whether players may take back and replay moves (undo/redo).
    #[serde(default = "default_true")]
    pub allow_undo: bool,
}

/// Field default for `allow_undo` when loading older saved games that
/// predate the option.
fn default_true() -> bool {
    true
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
            require_placement: true,
            allow_undo: true,
        }
    }
}
