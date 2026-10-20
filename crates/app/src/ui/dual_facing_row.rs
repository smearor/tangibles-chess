use gtk4::prelude::*;
use smearor_wrot_rotation::RotationWidget;
use smearor_wrot_rotation::SmearorRotation;

/// Renders the same control group twice - once upright for the south
/// player, once upside down inside a nested `RotationWidget` for the
/// north player. Both instances are built by the same factory and should
/// bind to the same actions/state.
pub struct DualFacingRow {
    /// The outer vertical container.
    pub widget: gtk4::Box,
}

impl DualFacingRow {
    /// Builds the row; `factory` is invoked twice to create the two
    /// facing instances.
    pub fn new(factory: impl Fn() -> gtk4::Widget) -> Self {
        let widget = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let north_slot = RotationWidget::new(SmearorRotation::new(180.0))
            .with_gesture_rotation_enabled(false)
            .with_animations_enabled(false);
        north_slot.set_child(Some(&factory()));
        north_slot.set_vexpand(true);
        let south = factory();
        south.set_vexpand(true);
        widget.append(&north_slot);
        widget.append(&south);
        Self { widget }
    }
}
