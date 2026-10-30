use crate::l10n::L10n;
use gtk4::prelude::*;
use gtk4::{Align, Orientation};
use smearor_wrot_rotation::RotationWidget;
use smearor_wrot_rotation::SmearorRotation;
use std::rc::Rc;
use tangibles_chess_core::Piece;
use tangibles_chess_core::Role;
use tangibles_chess_core::Side;

/// Promotion role chooser shown when a pawn reaches the last rank.
///
/// Rendered as a card facing the moving player's edge - wrapped in a
/// `RotationWidget` for the north player.
pub struct PromotionPicker {
    /// Root overlay widget (initially hidden).
    pub widget: gtk4::Widget,
}

impl PromotionPicker {
    /// A picker presenting the four promotion pieces of `side`, oriented
    /// toward `facing_north`'s edge.
    pub fn new(side: Side, facing_north: bool, on_pick: impl Fn(Role) + 'static, l10n: &L10n) -> Self {
        let card = gtk4::Box::new(Orientation::Vertical, 12);
        card.add_css_class("promotion-card");
        card.append(&gtk4::Label::new(Some(&l10n.msg("promotion-heading"))));
        let row = gtk4::Box::new(Orientation::Horizontal, 12);
        row.set_halign(Align::Center);
        let callback: Rc<dyn Fn(Role)> = Rc::new(on_pick);
        for role in Role::PROMOTABLE {
            let icon = gtk4::Image::from_icon_name(Piece { side, role }.icon_name());
            icon.set_pixel_size(96);
            let button = gtk4::Button::builder().child(&icon).build();
            button.add_css_class("promotion-choice");
            let callback = callback.clone();
            button.connect_clicked(move |_| callback(role));
            row.append(&button);
        }
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
