use gtk4::prelude::*;
use smearor_wrot_rotation::RotationWidget;
use smearor_wrot_rotation::SmearorRotation;

/// Which table edge a facing copy of the dual-facing content is oriented
/// toward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    /// The far (north) edge; the copy is rendered rotated 180° so the
    /// north player reads it upright.
    North,
    /// The near (south) edge; the copy is rendered upright.
    South,
}

/// Renders the same control group twice - once upright for the south
/// player, once upside down inside a nested `RotationWidget` for the
/// north player. Both instances are built by the same factory and should
/// bind to the same actions/state.
pub struct DualFacingRow {
    /// The outer vertical container.
    pub widget: gtk4::Box,
}

impl DualFacingRow {
    /// Builds the row; `factory` is invoked twice (north copy first, then
    /// south) to create the two facing instances. The [`Facing`] argument
    /// lets the copy render viewer-relative content - for the north card
    /// "the near edge" is the physical north edge.
    pub fn new(factory: impl Fn(Facing) -> gtk4::Widget) -> Self {
        let widget = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let north_slot = RotationWidget::new(SmearorRotation::new(180.0))
            .with_gesture_rotation_enabled(false)
            .with_animations_enabled(false);
        north_slot.set_child(Some(&factory(Facing::North)));
        north_slot.set_vexpand(true);
        let south = factory(Facing::South);
        south.set_vexpand(true);
        widget.append(&north_slot);
        widget.append(&south);
        Self { widget }
    }
}
