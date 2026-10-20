use gtk4::prelude::*;
use libadwaita as adw;
use smearor_wrot_rotation::RotationWidget;
use smearor_wrot_rotation::SmearorRotation;
use tangibles_chess_core::BoardOrientation;
use tangibles_chess_core::GameConfig;

use crate::l10n::L10n;
use crate::ui::board_check::BoardCheckOverlay;
use crate::ui::player_panel::PanelActions;
use crate::ui::tabletop_surface::TabletopSurface;

/// The live game screen: rotatable tabletop surface plus overlays
/// (promotion picker, board check, daemon banner, toasts).
pub struct GameScreen {
    /// Root widget.
    pub widget: adw::ToastOverlay,
    /// The rotatable surface (scene + panels + score sheets).
    pub surface: TabletopSurface,
    /// The outer rotation container.
    pub rotation: RotationWidget,
    /// Start-handshake overlay.
    pub board_check: BoardCheckOverlay,
    /// Daemon status banner.
    pub daemon_banner: adw::Banner,
    /// Overlay layer for pickers/dialogs.
    overlay: gtk4::Overlay,
    /// Current snapped orientation.
    orientation: std::cell::Cell<BoardOrientation>,
}

impl GameScreen {
    /// Builds the game screen for a config.
    pub fn new(actions: &PanelActions, l10n: &L10n, config: &GameConfig) -> Self {
        let surface = TabletopSurface::new(actions, l10n, config);
        let rotation = RotationWidget::new(SmearorRotation::new(config.orientation.degrees() as f32));
        rotation.set_child(Some(&surface.widget));

        let overlay = gtk4::Overlay::new();
        overlay.set_child(Some(&rotation));

        let board_check = BoardCheckOverlay::new(l10n);
        overlay.add_overlay(&board_check.widget);

        let daemon_banner = adw::Banner::new(&l10n.msg("daemon-offline"));
        daemon_banner.set_revealed(false);

        let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        column.append(&daemon_banner);
        column.append(&overlay);
        overlay.set_vexpand(true);

        let widget = adw::ToastOverlay::new();
        widget.set_child(Some(&column));

        Self {
            widget,
            surface,
            rotation,
            board_check,
            daemon_banner,
            overlay,
            orientation: std::cell::Cell::new(config.orientation),
        }
    }

    /// Current snapped orientation.
    pub fn orientation(&self) -> BoardOrientation {
        self.orientation.get()
    }

    /// Rotates the surface by one quarter turn (`clockwise` true → +90°).
    pub fn rotate_step(&self, clockwise: bool) {
        let next = if clockwise {
            self.orientation.get().clockwise()
        } else {
            self.orientation.get().counter_clockwise()
        };
        self.set_orientation(next);
    }

    /// Sets the snapped orientation with the widget's zoom animation.
    pub fn set_orientation(&self, orientation: BoardOrientation) {
        self.orientation.set(orientation);
        self.rotation.set_rotation_with_animation(orientation.degrees() as f64);
    }

    /// Locks/unlocks the rotation gesture (tangible games lock rotation
    /// while a game is in progress).
    pub fn set_rotation_locked(&self, locked: bool) {
        self.rotation.set_gesture_rotation_enabled(!locked);
    }

    /// Shows a transient toast.
    pub fn toast(&self, message: &str) {
        self.widget.add_toast(adw::Toast::new(message));
    }

    /// Adds a transient overlay (promotion picker etc.).
    pub fn push_overlay(&self, widget: &gtk4::Widget) {
        self.overlay.add_overlay(widget);
    }

    /// Removes an overlay.
    pub fn pop_overlay(&self, widget: &gtk4::Widget) {
        self.overlay.remove_overlay(widget);
    }
}
