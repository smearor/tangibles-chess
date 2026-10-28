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
    /// The panel (centered control buttons, captured tray overlaid at
    /// the left edge).
    pub widget: gtk4::Overlay,
    /// Captured-material glyphs/text.
    captured: gtk4::Label,
    /// Undo button (sensitivity follows config + history).
    undo: gtk4::Button,
    /// Redo button (sensitivity follows the redo stack).
    redo: gtk4::Button,
    /// Draw button (hidden in replay mode).
    draw: gtk4::Button,
    /// Resign button (hidden in replay mode).
    resign: gtk4::Button,
    /// Exit-replay button (visible only in replay mode).
    exit_replay: gtk4::Button,
}

/// Rendered icon size inside the square control buttons (px).
const CONTROL_ICON_SIZE: i32 = 32;

/// Callbacks driving a panel's buttons.
#[derive(Clone, Default)]
pub struct PanelActions {
    /// Undo request.
    pub on_undo: Option<Rc<dyn Fn()>>,
    /// Redo request.
    pub on_redo: Option<Rc<dyn Fn()>>,
    /// Draw offer/claim.
    pub on_draw: Option<Rc<dyn Fn()>>,
    /// Resign for the panel's color.
    pub on_resign: Option<Rc<dyn Fn(Color)>>,
    /// Rotate request (only used on the shared toolbar).
    pub on_rotate: Option<Rc<dyn Fn()>>,
    /// Leave replay mode back to the result dialog.
    pub on_exit_replay: Option<Rc<dyn Fn()>>,
}

impl PlayerPanel {
    /// A panel for `color` bound to `actions`/`l10n` texts. The panel's
    /// background takes the player's color instead of a name label.
    pub fn new(color: Color, actions: &PanelActions, l10n: &crate::l10n::L10n) -> Self {
        // Overlay instead of a box: the buttons sit at the panel's true
        // center and the captured tray floats at the left edge, so the
        // panel stays only as wide as the button row instead of
        // spilling past the screen edge.
        let widget = gtk4::Overlay::new();
        widget.add_css_class("player-panel");
        widget.add_css_class(match color {
            Color::White => "panel-white",
            Color::Black => "panel-black",
        });
        widget.set_halign(gtk4::Align::Center);
        widget.set_margin_start(24);
        widget.set_margin_end(24);

        let captured = gtk4::Label::new(None);
        captured.add_css_class("captured-tray");

        let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let undo = Self::control_button(icon_names::TCH_UNDO_SYMBOLIC, &l10n.msg("game-undo"));
        if let Some(on_undo) = &actions.on_undo {
            let on_undo = on_undo.clone();
            undo.connect_clicked(move |_| on_undo());
        }
        let redo = Self::control_button(icon_names::TCH_REDO_SYMBOLIC, &l10n.msg("game-redo"));
        if let Some(on_redo) = &actions.on_redo {
            let on_redo = on_redo.clone();
            redo.connect_clicked(move |_| on_redo());
        }
        let draw = Self::control_button(icon_names::TCH_HANDSHAKE_SYMBOLIC, &l10n.msg("game-offer-draw"));
        if let Some(on_draw) = &actions.on_draw {
            let on_draw = on_draw.clone();
            draw.connect_clicked(move |_| on_draw());
        }
        let resign = Self::control_button(icon_names::TCH_FLAG_SYMBOLIC, &l10n.msg("game-resign"));
        if let Some(on_resign) = &actions.on_resign {
            let on_resign = on_resign.clone();
            resign.connect_clicked(move |_| on_resign(color));
        }
        let rotate = Self::control_button(icon_names::TCH_ROTATE_CW_SYMBOLIC, &l10n.msg("game-rotate"));
        if let Some(on_rotate) = &actions.on_rotate {
            let on_rotate = on_rotate.clone();
            rotate.connect_clicked(move |_| on_rotate());
        }
        // Back to the setup dialog; the running game stays alive in the
        // window's stack.
        let menu = Self::control_button(icon_names::TCH_SETTINGS_SYMBOLIC, &l10n.msg("game-menu"));
        menu.set_action_name(Some("win.new-game"));
        // Exits replay mode; hidden until a replay starts.
        let exit_replay = Self::control_button(icon_names::TCH_LOG_OUT_SYMBOLIC, &l10n.msg("game-exit-replay"));
        exit_replay.set_visible(false);
        if let Some(on_exit_replay) = &actions.on_exit_replay {
            let on_exit_replay = on_exit_replay.clone();
            exit_replay.connect_clicked(move |_| on_exit_replay());
        }
        buttons.append(&undo);
        buttons.append(&redo);
        buttons.append(&draw);
        buttons.append(&resign);
        buttons.append(&rotate);
        buttons.append(&menu);
        buttons.append(&exit_replay);

        buttons.set_halign(gtk4::Align::Center);
        widget.set_child(Some(&buttons));
        captured.set_halign(gtk4::Align::Start);
        captured.set_valign(gtk4::Align::Center);
        // The tray floats over the left edge of the button row - it
        // must not swallow clicks destined for the buttons beneath.
        captured.set_can_target(false);
        widget.add_overlay(&captured);

        Self {
            widget,
            captured,
            undo,
            redo,
            draw,
            resign,
            exit_replay,
        }
    }

    /// Builds one square icon button of the panel's control group.
    ///
    /// The button's edge length comes from the `control-button` CSS class;
    /// `valign: Center` keeps it square when the edge row is taller than
    /// the button itself.
    fn control_button(icon_name: &str, tooltip: &str) -> gtk4::Button {
        let icon = gtk4::Image::from_icon_name(icon_name);
        icon.set_pixel_size(CONTROL_ICON_SIZE);
        let button = gtk4::Button::builder().child(&icon).build();
        button.add_css_class("control-button");
        button.set_tooltip_text(Some(tooltip));
        button.set_valign(gtk4::Align::Center);
        button
    }

    /// Enables/disables the undo button (config gate + history depth).
    pub fn set_undo_enabled(&self, enabled: bool) {
        self.undo.set_sensitive(enabled);
    }

    /// Enables/disables the redo button (redo stack non-empty).
    pub fn set_redo_enabled(&self, enabled: bool) {
        self.redo.set_sensitive(enabled);
    }

    /// Enables/disables the game-ending actions (draw, resign) - off in
    /// replay mode and once the game ended.
    pub fn set_play_enabled(&self, enabled: bool) {
        self.draw.set_sensitive(enabled);
        self.resign.set_sensitive(enabled);
    }

    /// Toggles replay-mode controls: hides draw/resign, reveals the
    /// exit-replay button.
    pub fn set_replay_mode(&self, in_replay: bool) {
        self.draw.set_visible(!in_replay);
        self.resign.set_visible(!in_replay);
        self.exit_replay.set_visible(in_replay);
    }

    /// Updates the captured-material display (e.g. `♟♟♞` or counts).
    pub fn set_captured(&self, text: &str) {
        self.captured.set_text(text);
    }

    /// Marks this panel as the side to move.
    pub fn set_active(&self, active: bool) {
        if active {
            self.widget.add_css_class("active");
        } else {
            self.widget.remove_css_class("active");
        }
    }
}
