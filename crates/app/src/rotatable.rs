//! Board rotation in quarter turns.

/// Rotatable board display: `rotate` applies the step, the direction
/// helpers are provided as defaults.
pub trait Rotatable {
    /// Rotates the board one quarter turn clockwise.
    fn rotate_cw(&mut self) {
        self.rotate(true);
    }

    /// Rotates the board one quarter turn counter-clockwise.
    fn rotate_ccw(&mut self) {
        self.rotate(false);
    }

    /// Rotates the board one quarter turn in `clockwise`'s direction.
    fn rotate(&mut self, clockwise: bool);
}
