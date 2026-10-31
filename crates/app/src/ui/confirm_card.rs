//! A two-button prompt card facing one table edge.

use gtk4::Align;
use gtk4::Orientation;
use gtk4::prelude::*;
use smearor_wrot_rotation::RotationWidget;
use smearor_wrot_rotation::SmearorRotation;

/// A prompt with confirm/decline buttons, rendered as an overlay card
/// facing one player's edge - the accept side of a draw offer or a
/// resign confirmation.
pub struct ConfirmCard {
    /// Root overlay widget.
    pub widget: gtk4::Widget,
}

impl ConfirmCard {
    /// A card titled `heading` with `confirm_label`/`decline_label`
    /// buttons, oriented toward `facing_north`'s edge.
    pub fn new(
        heading: &str,
        confirm_label: &str,
        decline_label: &str,
        facing_north: bool,
        on_confirm: impl Fn() + 'static,
        on_decline: impl Fn() + 'static,
    ) -> Self {
        let card = gtk4::Box::new(Orientation::Vertical, 12);
        card.add_css_class("confirm-card");
        card.append(&gtk4::Label::new(Some(heading)));
        let row = gtk4::Box::new(Orientation::Horizontal, 12);
        row.set_halign(Align::Center);
        let confirm = gtk4::Button::with_label(confirm_label);
        confirm.add_css_class("suggested-action");
        confirm.add_css_class("confirm-choice");
        confirm.connect_clicked(move |_| on_confirm());
        let decline = gtk4::Button::with_label(decline_label);
        decline.add_css_class("confirm-choice");
        decline.connect_clicked(move |_| on_decline());
        row.append(&confirm);
        row.append(&decline);
        card.append(&row);
        card.set_halign(Align::Center);
        card.set_valign(Align::Center);

        let widget: gtk4::Widget = if facing_north {
            let rotated = RotationWidget::new(SmearorRotation::new(180.0))
                .with_gesture_rotation_enabled(false)
                .with_animations_enabled(false);
            rotated.set_child(Some(&card));
            rotated.upcast()
        } else {
            card.upcast()
        };
        widget.set_halign(Align::Center);
        widget.set_valign(Align::Center);
        Self { widget }
    }
}
