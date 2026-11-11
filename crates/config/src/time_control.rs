use serde::Deserialize;
use serde::Serialize;

/// Clock time control for a game (FR-ENG-3): a fixed thinking budget
/// per side plus an optional Fischer increment credited after every
/// completed move. `increment_seconds = 0` is a fixed-time
/// (sudden-death) game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimeControl {
    /// Thinking time each side starts with, in seconds.
    pub initial_seconds: u32,
    /// Increment credited to the mover after each completed move, in
    /// seconds (`0` = fixed time).
    pub increment_seconds: u32,
}

impl TimeControl {
    /// A time control from base seconds + increment seconds.
    pub fn new(initial_seconds: u32, increment_seconds: u32) -> Self {
        Self {
            initial_seconds,
            increment_seconds,
        }
    }

    /// Compact chess notation (`"10+0"`, `"15+10"`) - the label of the
    /// setup picker's clock options. A base that does not split into
    /// whole minutes renders as `m:ss`.
    pub fn notation(&self) -> String {
        let (minutes, seconds) = (self.initial_seconds / 60, self.initial_seconds % 60);
        let base = if seconds == 0 {
            format!("{minutes}")
        } else {
            format!("{minutes}:{seconds:02}")
        };
        format!("{base}+{}", self.increment_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::TimeControl;

    #[test]
    fn notation_uses_minutes_plus_increment() {
        assert_eq!(TimeControl::new(600, 0).notation(), "10+0");
        assert_eq!(TimeControl::new(900, 10).notation(), "15+10");
    }

    #[test]
    fn notation_renders_sub_minute_base_as_mm_ss() {
        assert_eq!(TimeControl::new(90, 1).notation(), "1:30+1");
    }
}
