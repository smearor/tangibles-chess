//! The chess-clock timer (FR-ENG-3): a GTK source debits the
//! session-owned `ChessClock` and mirrors the remaining budgets onto
//! the per-edge panel labels. The session switches sides inside
//! `request_move`; this module only ticks, paints and reports flag
//! falls.

use std::time::Duration;

use glib::ControlFlow;
use tangibles_chess_material::Side;
use tangibles_chess_moves::GamePhase;

use super::game_controller::GameController;
use super::registry::with_controller;
use crate::sound_effect::SoundEffect;

/// Tick cadence of the clock timer (ms). Sub-second resolution keeps
/// flag detection and the tenths display snappy.
const CLOCK_TICK_MS: u64 = 100;

impl GameController {
    /// Installs the clock timer when the session runs a clock. The
    /// timer waits out `AwaitingPlacement` and self-terminates once the
    /// game is `Finished`.
    pub(super) fn start_clock_timer(&mut self) {
        if self.session.clock().is_none() {
            return;
        }
        glib::timeout_add_local(Duration::from_millis(CLOCK_TICK_MS), move || {
            let mut ticking = false;
            with_controller(|controller| {
                ticking = controller.borrow_mut().clock_tick();
            });
            if ticking { ControlFlow::Continue } else { ControlFlow::Break }
        });
    }

    /// One clock timer pass: debits elapsed time, mirrors the labels
    /// and finishes the game on a flag fall. Returns whether the timer
    /// keeps ticking - it must survive `AwaitingPlacement` (the
    /// countdown only starts once the handshake completes) and only
    /// exits on `Finished`.
    fn clock_tick(&mut self) -> bool {
        if self.session.phase() == GamePhase::InProgress && self.replay_index.is_none() {
            if self.session.tick_clock() {
                self.sound.play(SoundEffect::End);
                self.sync_scene();
                self.autosave();
                return false;
            }
            self.update_clock_labels();
            return true;
        }
        self.update_clock_labels();
        self.session.phase() != GamePhase::Finished
    }

    /// Mirrors the session clock onto both edge panels; untimed games
    /// hide the display.
    pub(super) fn update_clock_labels(&self) {
        match self.session.clock() {
            Some(clock) => {
                for side in Side::ALL {
                    let panel = match side {
                        Side::White => &self.screen.surface.white_panel,
                        Side::Black => &self.screen.surface.black_panel,
                    };
                    panel.set_clock(&format_remaining(clock.remaining_ms(side)), clock.is_running(side), clock.flagged() == Some(side));
                }
            }
            None => {
                self.screen.surface.white_panel.hide_clock();
                self.screen.surface.black_panel.hide_clock();
            }
        }
    }
}

/// Formats a remaining budget for the edge display: `h:mm:ss` for
/// long games, `m:ss` normally and `s.d` tenths under ten seconds -
/// the flag tension needs the finer grain.
fn format_remaining(ms: u64) -> String {
    if ms >= 3_600_000 {
        format!("{}:{:02}:{:02}", ms / 3_600_000, (ms % 3_600_000) / 60_000, (ms % 60_000) / 1000)
    } else if ms >= 10_000 {
        format!("{}:{:02}", ms / 60_000, (ms % 60_000) / 1000)
    } else {
        format!("{}.{}", ms / 1000, (ms % 1000) / 100)
    }
}

#[cfg(test)]
mod tests {
    use super::format_remaining;

    #[test]
    fn long_budgets_render_as_mm_ss() {
        assert_eq!(format_remaining(600_000), "10:00");
        assert_eq!(format_remaining(65_400), "1:05");
    }

    #[test]
    fn hour_budgets_render_as_h_mm_ss() {
        assert_eq!(format_remaining(3_725_000), "1:02:05");
    }

    #[test]
    fn flag_tension_renders_tenths() {
        assert_eq!(format_remaining(9_340), "9.3");
        assert_eq!(format_remaining(0), "0.0");
    }
}
