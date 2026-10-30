//! `Rotatable` on the controller: quarter-turn board rotation.

use tangibles_chess_moves::GamePhase;
use tangibles_chess_tangibles::BoardGeometry;
use tangibles_chess_tangibles::MoveInference;
use tangibles_chess_tangibles::PlacementGate;

use super::game_controller::GameController;
use super::registry::with_controller;
use crate::rotatable::Rotatable;

impl Rotatable for GameController {
    /// Applies a rotation step, updating the field mapping. Locked while
    /// a tangible game is in progress (resting markers would remap).
    fn rotate(&mut self, clockwise: bool) {
        let locked = self.session.config().tangible_mode.uses_tangibles() && self.session.phase() == GamePhase::InProgress;
        if locked {
            self.screen.show_message(&self.l10n.msg("game-rotation-locked"));
            return;
        }
        self.screen.rotate_step(clockwise);
        // The physical field mapping follows the rotated board - re-register
        // once the rotation animation has settled.
        glib::timeout_add_local_once(std::time::Duration::from_millis(600), || {
            with_controller(|controller| Self::register_when_ready(controller.clone()));
        });
        self.geometry = BoardGeometry::new(self.screen.orientation(), self.geometry.white_edge());
        self.inference = MoveInference::new(self.geometry, self.session.config().commit_mode);
        self.gate = PlacementGate::new(self.geometry);
    }
}
