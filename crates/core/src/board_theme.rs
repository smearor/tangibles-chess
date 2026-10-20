use serde::Deserialize;
use serde::Serialize;

/// Visual theme of the board material.
///
/// Themes carry the artistic direction of the app (noble, rustic) - the
/// board is rendered as material (wood inlay, slate, felt) rather than
/// plain UI chrome.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BoardTheme {
    /// Classic walnut/maple wood inlay.
    #[default]
    WalnutMaple,
    /// Dark modern slate.
    Slate,
    /// Tournament felt green.
    TournamentGreen,
}
