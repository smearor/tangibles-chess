use std::time::Instant;

use tangibles_chess_config::TimeControl;
use tangibles_chess_material::Side;

/// A running chess clock (FR-ENG-3): one countdown per side; only the
/// side to move is debited.
///
/// `GameSession` owns the switching - the countdown starts when a turn
/// begins (game start, placement confirmation, undo/redo), [`Self::press`]
/// fires inside `request_move` once a move lands and the clock stops on
/// every terminal transition. The UI only needs to call [`Self::tick`]
/// and read [`Self::remaining_ms`].
#[derive(Debug, Clone)]
pub struct ChessClock {
    /// The configured time control.
    control: TimeControl,
    /// Banked remaining milliseconds per side (everything up to
    /// `last_tick` already debited).
    remaining: [u64; 2],
    /// The side currently being debited; `None` while paused.
    running: Option<Side>,
    /// When the current debit window began.
    last_tick: Instant,
    /// The side whose flag fell first, if any.
    flagged: Option<Side>,
}

/// Index of `side` inside `remaining`.
fn index_of(side: Side) -> usize {
    match side {
        Side::White => 0,
        Side::Black => 1,
    }
}

impl ChessClock {
    /// A fresh clock: both sides hold the full budget, paused until the
    /// game starts.
    pub fn new(control: TimeControl) -> Self {
        Self {
            control,
            remaining: [u64::from(control.initial_seconds) * 1000; 2],
            running: None,
            last_tick: Instant::now(),
            flagged: None,
        }
    }

    /// A clock restored from the autosave's banked snapshot, still
    /// paused.
    pub fn resume(control: TimeControl, white_ms: u64, black_ms: u64) -> Self {
        Self {
            control,
            remaining: [white_ms, black_ms],
            running: None,
            last_tick: Instant::now(),
            flagged: None,
        }
    }

    /// Starts debiting `side`, banking the previously running side
    /// first. Re-pointing the countdown keeps spent time spent - undo
    /// does not refund think time.
    pub fn start(&mut self, side: Side) {
        self.bank(Instant::now());
        self.running = Some(side);
    }

    /// Stops the clock, banking the running side's elapsed time.
    pub fn stop(&mut self) {
        self.bank(Instant::now());
        self.running = None;
    }

    /// The mover completed a move: their countdown stops, the increment
    /// is credited and the clock passes to the opponent.
    pub fn press(&mut self, mover: Side) {
        self.bank(Instant::now());
        let mover_index = index_of(mover);
        self.remaining[mover_index] = self.remaining[mover_index].saturating_add(u64::from(self.control.increment_seconds) * 1000);
        self.running = Some(mover.other());
    }

    /// Debits elapsed wall time; returns the side whose flag just fell,
    /// only when this tick crossed zero for the first time.
    pub fn tick(&mut self) -> Option<Side> {
        if self.flagged.is_some() {
            return None;
        }
        self.bank(Instant::now());
        self.flagged
    }

    /// Remaining milliseconds for `side`, live-debited while running.
    pub fn remaining_ms(&self, side: Side) -> u64 {
        let banked = self.remaining[index_of(side)];
        if self.running == Some(side) {
            banked.saturating_sub(self.last_tick.elapsed().as_millis() as u64)
        } else {
            banked
        }
    }

    /// Whether `side`'s countdown is currently running.
    pub fn is_running(&self, side: Side) -> bool {
        self.running == Some(side)
    }

    /// The side whose flag fell, if any.
    pub fn flagged(&self) -> Option<Side> {
        self.flagged
    }

    /// Debits the running side's elapsed time into `remaining`,
    /// recording a flag fall once the budget reaches zero.
    fn bank(&mut self, now: Instant) {
        if let Some(side) = self.running {
            let elapsed = now.saturating_duration_since(self.last_tick).as_millis() as u64;
            let index = index_of(side);
            self.remaining[index] = self.remaining[index].saturating_sub(elapsed);
            if self.remaining[index] == 0 && self.flagged.is_none() {
                self.flagged = Some(side);
            }
        }
        self.last_tick = now;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::time::Instant;

    use super::ChessClock;
    use tangibles_chess_config::TimeControl;
    use tangibles_chess_material::Side;

    fn clock() -> ChessClock {
        ChessClock::new(TimeControl::new(600, 5))
    }

    /// Rewinds the debit window so `elapsed` appears to have passed.
    fn drain(clock: &mut ChessClock, elapsed: Duration) {
        clock.last_tick = Instant::now() - elapsed;
    }

    #[test]
    fn fresh_clock_is_paused_at_full_budget() {
        let clock = clock();
        assert_eq!(clock.remaining_ms(Side::White), 600_000);
        assert_eq!(clock.remaining_ms(Side::Black), 600_000);
        assert!(!clock.is_running(Side::White));
        assert!(clock.flagged().is_none());
    }

    #[test]
    fn only_the_running_side_is_debited() {
        let mut clock = clock();
        clock.start(Side::White);
        drain(&mut clock, Duration::from_secs(12));
        assert!(clock.remaining_ms(Side::White) <= 588_500);
        assert_eq!(clock.remaining_ms(Side::Black), 600_000);
    }

    #[test]
    fn press_banks_elapsed_and_credits_the_increment() {
        let mut clock = clock();
        clock.start(Side::White);
        drain(&mut clock, Duration::from_secs(10));
        clock.press(Side::White);
        assert!(clock.remaining_ms(Side::White) >= 594_000);
        assert!(clock.is_running(Side::Black));
    }

    #[test]
    fn flag_falls_once_and_stays() {
        let mut clock = clock();
        clock.start(Side::White);
        drain(&mut clock, Duration::from_secs(700));
        assert_eq!(clock.tick(), Some(Side::White));
        assert_eq!(clock.tick(), None);
        assert_eq!(clock.flagged(), Some(Side::White));
        assert_eq!(clock.remaining_ms(Side::White), 0);
    }

    #[test]
    fn resume_restores_the_banked_snapshot() {
        let clock = ChessClock::resume(TimeControl::new(600, 0), 123_000, 456_000);
        assert_eq!(clock.remaining_ms(Side::White), 123_000);
        assert_eq!(clock.remaining_ms(Side::Black), 456_000);
    }
}
