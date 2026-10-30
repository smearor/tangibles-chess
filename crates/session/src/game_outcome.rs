//! The game's terminal result.

use serde::Deserialize;
use serde::Serialize;
use shakmaty::KnownOutcome;
use tangibles_chess_material::Side;

/// How a finished game ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameOutcome {
    /// `winner` won the game.
    Decisive {
        /// The winning side.
        winner: Side,
    },
    /// The game was drawn.
    Draw,
}

impl From<KnownOutcome> for GameOutcome {
    /// The domain outcome of the engine's `outcome`.
    fn from(outcome: KnownOutcome) -> Self {
        match outcome {
            KnownOutcome::Decisive { winner } => Self::Decisive { winner: Side::from(winner) },
            KnownOutcome::Draw => Self::Draw,
        }
    }
}
