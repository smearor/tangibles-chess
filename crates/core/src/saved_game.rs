use serde::Deserialize;
use serde::Serialize;
use shakmaty::Chess;
use shakmaty::Position;
use shakmaty::uci::UciMove;

use crate::GameConfig;
use crate::GameError;

/// Persistent snapshot of a game for autosave/resume.
///
/// Stores the setup plus every move as a UCI string; the position is
/// rebuilt by replaying the moves from the standard initial position.
/// Stored as TOML below the user's data directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedGame {
    /// The configuration the game was started with.
    pub config: GameConfig,
    /// Played moves in UCI notation, in order.
    pub moves: Vec<String>,
    /// SAN history for display (recomputed on load as a check is cheap).
    pub sans: Vec<String>,
    /// Resigned side, if the game ended by resignation.
    pub resigned: Option<String>,
    /// Whether a draw was agreed.
    pub agreed_draw: bool,
}

impl SavedGame {
    /// Serializes to TOML.
    pub fn to_toml(&self) -> Result<String, GameError> {
        toml::to_string_pretty(self).map_err(|e| GameError::SavedGame(e.to_string()))
    }

    /// Parses from TOML.
    pub fn from_toml(text: &str) -> Result<Self, GameError> {
        toml::from_str(text).map_err(|e| GameError::SavedGame(e.to_string()))
    }

    /// Replays all stored moves onto a fresh position.
    ///
    /// Returns every intermediate position plus the final one, so callers
    /// get the full stack a session needs.
    pub fn replay(&self) -> Result<Vec<Chess>, GameError> {
        let mut positions = Vec::with_capacity(self.moves.len() + 1);
        let mut position = Chess::default();
        positions.push(position.clone());
        for (index, uci) in self.moves.iter().enumerate() {
            let mv = uci
                .parse::<UciMove>()
                .map_err(|e| GameError::CorruptSave {
                    index,
                    uci: uci.clone(),
                    reason: e.to_string(),
                })?
                .to_move(&position)
                .map_err(|e| GameError::CorruptSave {
                    index,
                    uci: uci.clone(),
                    reason: e.to_string(),
                })?;
            position = position.play(mv).map_err(|e| GameError::CorruptSave {
                index,
                uci: uci.clone(),
                reason: e.to_string(),
            })?;
            positions.push(position.clone());
        }
        Ok(positions)
    }
}

#[cfg(test)]
mod tests {
    use super::SavedGame;
    use crate::GameConfig;

    #[test]
    fn toml_roundtrip() {
        let saved = SavedGame {
            config: GameConfig::default(),
            moves: vec!["e2e4".to_string(), "e7e5".to_string()],
            sans: vec!["e4".to_string(), "e5".to_string()],
            resigned: None,
            agreed_draw: false,
        };
        let parsed = SavedGame::from_toml(&saved.to_toml().expect("ser")).expect("de");
        assert_eq!(parsed, saved);
    }

    #[test]
    fn replay_rebuilds_positions() {
        let saved = SavedGame {
            config: GameConfig::default(),
            moves: vec!["e2e4".to_string(), "e7e5".to_string(), "g1f3".to_string()],
            sans: vec![],
            resigned: None,
            agreed_draw: false,
        };
        let positions = saved.replay().expect("replay");
        assert_eq!(positions.len(), 4);
    }

    #[test]
    fn replay_rejects_illegal_move() {
        let saved = SavedGame {
            config: GameConfig::default(),
            moves: vec!["e2e5".to_string()],
            sans: vec![],
            resigned: None,
            agreed_draw: false,
        };
        assert!(saved.replay().is_err());
    }
}
