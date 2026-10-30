//! Game configuration - the settings assembled by [`GameConfig`].

mod board_theme;
mod commit_mode;
mod game_config;
mod opponent_kind;
mod tangible_mode;

pub use board_theme::BoardTheme;
pub use commit_mode::CommitMode;
pub use game_config::GameConfig;
pub use opponent_kind::OpponentKind;
pub use tangible_mode::TangibleMode;
