//! Global access to the running controller.

use std::cell::RefCell;
use std::rc::Rc;

use super::game_controller::GameController;

thread_local! {
    /// The active controller, reachable from widget callbacks that do
    /// not hold it directly (promotion picker, panel actions).
    static ACTIVE_CONTROLLER: RefCell<Option<Rc<RefCell<GameController>>>> = const { RefCell::new(None) };
}

/// Registers the running controller globally (single window app).
pub fn set_active_controller(controller: Rc<RefCell<GameController>>) {
    ACTIVE_CONTROLLER.with(|slot| *slot.borrow_mut() = Some(controller));
}

/// Runs `f` with the active controller, if registered.
pub fn with_controller(f: impl FnOnce(&Rc<RefCell<GameController>>)) {
    ACTIVE_CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow().as_ref() {
            f(controller);
        }
    });
}
