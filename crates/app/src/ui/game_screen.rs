use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita as adw;
use shakmaty::Color;
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
    /// White's status line (between board and controls).
    white_line: StatusLine,
    /// Black's status line (between board and controls).
    black_line: StatusLine,
}

/// How long a transient status message stays up before the persistent
/// instruction returns (ms).
const STATUS_MESSAGE_MS: u64 = 3000;

/// One side's status line: a persistent instruction plus transient
/// messages that expire back to it.
struct StatusLine {
    /// The label between board and controls.
    label: gtk4::Label,
    /// Last persistent instruction, restored when a message expires.
    instruction: Rc<RefCell<String>>,
    /// Generation counter guarding the expiry timer.
    seq: Rc<Cell<u64>>,
}

impl StatusLine {
    /// A line backed by `label`.
    fn new(label: gtk4::Label) -> Self {
        Self {
            label,
            instruction: Rc::new(RefCell::new(String::new())),
            seq: Rc::new(Cell::new(0)),
        }
    }

    /// Persistent instruction (turn hints); empty text clears the line.
    fn set(&self, text: &str) {
        *self.instruction.borrow_mut() = text.to_string();
        self.label.set_text(text);
    }

    /// Transient message; the line falls back to the instruction after
    /// `ms` unless a newer message superseded it.
    fn flash(&self, text: &str, ms: u64) {
        let seq = self.seq.get() + 1;
        self.seq.set(seq);
        self.label.set_text(text);
        let label = self.label.clone();
        let instruction = self.instruction.clone();
        let cell = self.seq.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(ms), move || {
            if cell.get() == seq {
                label.set_text(&instruction.borrow());
            }
        });
    }
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

        let white_line = StatusLine::new(surface.white_status.clone());
        let black_line = StatusLine::new(surface.black_status.clone());
        Self {
            widget,
            surface,
            rotation,
            board_check,
            daemon_banner,
            overlay,
            orientation: std::cell::Cell::new(config.orientation),
            white_line,
            black_line,
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

    /// The status line of `color`'s side.
    fn line(&self, color: Color) -> &StatusLine {
        match color {
            Color::White => &self.white_line,
            Color::Black => &self.black_line,
        }
    }

    /// Persistent instruction on `color`'s status line (e.g. "White to
    /// move"); empty text clears the line.
    pub fn set_instruction(&self, color: Color, text: &str) {
        self.line(color).set(text);
    }

    /// Transient message on `color`'s status line - for errors caused
    /// by that side, like a rejected move or a protocol anomaly.
    pub fn show_message_for(&self, color: Color, text: &str) {
        self.line(color).flash(text, STATUS_MESSAGE_MS);
    }

    /// Transient message on both status lines.
    pub fn show_message(&self, text: &str) {
        self.white_line.flash(text, STATUS_MESSAGE_MS);
        self.black_line.flash(text, STATUS_MESSAGE_MS);
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
