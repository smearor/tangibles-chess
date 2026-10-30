//! Playing a game: board input, move application, and the game-ending
//! actions.

use shakmaty::Square;
use tangibles_chess_core::MoveOutcome;
use tangibles_chess_core::MoveRequest;
use tangibles_chess_material::Side;

/// The playing surface's verb surface: scene taps, validated move
/// application, and the two ways a game ends by consent.
pub trait Playable {
    /// Touch tap on a scene square (touch-only input mode).
    fn on_square(&mut self, square: Square);

    /// Validates and applies a move `request`, dispatching the outcome
    /// to the UI. Returns the session's outcome for remote callers.
    fn apply_request(&mut self, request: MoveRequest) -> MoveOutcome;

    /// `side` resigns.
    fn resign(&mut self, side: Side);

    /// Agrees a draw.
    fn draw(&mut self);
}
