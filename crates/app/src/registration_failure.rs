//! The last field-set registration failure - a `thread_local` slot the
//! setup dialog shows as a banner. Written on the GTK main loop when
//! `register_when_ready` gets a rejection; read (and cleared) when the
//! setup screen is built.

use std::cell::RefCell;

thread_local! {
    /// The daemon's rejection reason of the last registration attempt.
    static LAST: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Stores the registration failure reason for the next setup dialog.
pub(crate) fn set(reason: String) {
    LAST.with(|slot| *slot.borrow_mut() = Some(reason));
}

/// Takes the pending failure (consumes it - the banner shows once).
pub(crate) fn take() -> Option<String> {
    LAST.with(|slot| slot.borrow_mut().take())
}
