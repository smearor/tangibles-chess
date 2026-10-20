use serde::Deserialize;
use serde::Serialize;

/// A rule-based reason a game may end in a draw.
///
/// `Claimable` reasons end the game when a player claims them; `Automatic`
/// reasons apply immediately (FIDE article 9.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DrawReason {
    /// The same position occurred three times (claimable).
    ThreefoldRepetition,
    /// Fifty consecutive moves without pawn move or capture (claimable).
    FiftyMoves,
    /// The same position occurred five times (automatic).
    FivefoldRepetition,
    /// Seventy-five moves without pawn move or capture (automatic).
    SeventyFiveMoves,
    /// Neither side can possibly checkmate (automatic).
    InsufficientMaterial,
    /// Both players agreed.
    Agreement,
}
