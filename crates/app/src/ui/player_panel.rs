use std::rc::Rc;

use gtk4::prelude::*;
use shakmaty::Color;
use tangibles_chess_icons::icon_names;

/// Per-player edge panel: name, captured-material tray display and the
/// action buttons (undo, draw offer, resign).
///
/// North panels are shown upside down via a nested `RotationWidget` in
/// `TabletopSurface`; callbacks drive the shared `GameController`, so
/// both panel instances act identically.
pub struct PlayerPanel {
    /// The panel row.
    pub widget: gtk4::Box,
    /// Player label.
    name: gtk4::Label,
    /// Captured-material glyphs/text.
    captured: gtk4::Label,
    /// Small status line (turn, check).
    status: gtk4::Label,
}

/// Callbacks driving a panel's buttons.
#[derive(Clone, Default)]
pub struct PanelActions {
    /// Undo request.
    pub on_undo: Option<Rc<dyn Fn()>>,
    /// Draw offer/claim.
    pub on_draw: Option<Rc<dyn Fn()>>,
    /// Resign for the panel's color.
    pub on_resign: Option<Rc<dyn Fn(Color)>>,
    /// Rotate request (only used on the shared toolbar).
    pub on_rotate: Option<Rc<dyn Fn()>>,
}

impl PlayerPanel {
    /// A panel for `color` bound to `actions`/`l10n` texts.
    pub fn new(color: Color, name: &str, actions: &PanelActions, l10n: &crate::l10n::L10n) -> Self {
        let widget = gtk4::Box::new(gtk4::Orientation::Horizontal, 18);
        widget.add_css_class("player-panel");
        widget.set_halign(gtk4::Align::Center);
        widget.set_margin_start(24);
        widget.set_margin_end(24);
        widget.set_margin_top(8);
        widget.set_margin_bottom(8);

        let name_label = gtk4::Label::new(Some(name));
        name_label.add_css_class("player-name");
        let captured = gtk4::Label::new(None);
        captured.add_css_class("captured-tray");
        let status = gtk4::Label::new(None);
        status.add_css_class("player-status");

        let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let undo = gtk4::Button::builder().icon_name(icon_names::TCH_UNDO_SYMBOLIC).build();
        undo.set_tooltip_text(Some(&l10n.msg("game-undo")));
        if let Some(on_undo) = &actions.on_undo {
            let on_undo = on_undo.clone();
            undo.connect_clicked(move |_| on_undo());
        }
        let draw = gtk4::Button::builder().icon_name(icon_names::TCH_HANDSHAKE_SYMBOLIC).build();
        draw.set_tooltip_text(Some(&l10n.msg("game-offer-draw")));
        if let Some(on_draw) = &actions.on_draw {
            let on_draw = on_draw.clone();
            draw.connect_clicked(move |_| on_draw());
        }
        let resign = gtk4::Button::builder().icon_name(icon_names::TCH_FLAG_SYMBOLIC).build();
        resign.set_tooltip_text(Some(&l10n.msg("game-resign")));
        if let Some(on_resign) = &actions.on_resign {
            let on_resign = on_resign.clone();
            resign.connect_clicked(move |_| on_resign(color));
        }
        let rotate = gtk4::Button::builder().icon_name(icon_names::TCH_ROTATE_CW_SYMBOLIC).build();
        rotate.set_tooltip_text(Some(&l10n.msg("game-rotate")));
        if let Some(on_rotate) = &actions.on_rotate {
            let on_rotate = on_rotate.clone();
            rotate.connect_clicked(move |_| on_rotate());
        }
        buttons.append(&undo);
        buttons.append(&draw);
        buttons.append(&resign);
        buttons.append(&rotate);

        widget.append(&name_label);
        widget.append(&captured);
        widget.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
        widget.append(&status);
        widget.append(&buttons);

        Self {
            widget,
            name: name_label,
            captured,
            status,
        }
    }

    /// Updates the status line.
    pub fn set_status(&self, text: &str) {
        self.status.set_text(text);
    }

    /// Updates the captured-material display (e.g. `♟♟♞` or counts).
    pub fn set_captured(&self, text: &str) {
        self.captured.set_text(text);
    }

    /// Marks this panel as the side to move.
    pub fn set_active(&self, active: bool) {
        if active {
            self.name.add_css_class("active");
        } else {
            self.name.remove_css_class("active");
        }
    }
}
