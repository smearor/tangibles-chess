//! Stepping backward and forward through the game record.

/// Reversible navigation: undoes and re-does half-moves. In replay
/// mode the same operations walk the replay cursor instead of
/// mutating the session.
pub trait Undoable {
    /// Steps one half-move back.
    fn undo(&mut self);

    /// Steps one half-move forward.
    fn redo(&mut self);
}
