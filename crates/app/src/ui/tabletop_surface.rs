use gtk4::prelude::*;
use smearor_wrot_rotation::RotationWidget;
use smearor_wrot_rotation::SmearorRotation;
use tangibles_chess_core::GameConfig;
use tangibles_chess_material::Side;

use crate::ui::chess_scene::ChessScene;
use crate::ui::player_panel::PanelActions;
use crate::ui::player_panel::PlayerPanel;
use crate::ui::score_sheet::ScoreSheet;

/// The rotatable tabletop: board centered, both player panels along the
/// north/south edges plus the two score-sheet columns.
///
/// Scene space always places White's panel at the bottom and Black's at
/// the top; the outer `RotationWidget` orients the whole surface to the
/// desk's rotation. The north-side copies are flipped 180° internally so
/// both players read their own panel.
pub struct TabletopSurface {
    /// The root widget placed inside the outer `RotationWidget`.
    pub widget: gtk4::Box,
    /// The board widget.
    pub scene: ChessScene,
    /// White's panel (south edge).
    pub white_panel: PlayerPanel,
    /// Black's panel (north edge, internally flipped).
    pub black_panel: PlayerPanel,
    /// White's score sheet (south).
    pub white_sheet: ScoreSheet,
    /// Black's score sheet (north).
    pub black_sheet: ScoreSheet,
    /// White's status line (between board and controls).
    pub white_status: gtk4::Label,
    /// Black's status line (between board and controls).
    pub black_status: gtk4::Label,
}

impl TabletopSurface {
    /// Builds the surface; panel actions are shared by both panels.
    pub fn new(actions: &PanelActions, l10n: &crate::l10n::L10n, config: &GameConfig) -> Self {
        let scene = ChessScene::new();
        scene.update(|state| {
            state.theme = config.theme;
            state.render_pieces = config.tangible_mode.renders_pieces();
            state.parking_zones = config.tangible_mode.uses_tangibles();
        });

        let white_panel = PlayerPanel::new(Side::White, actions, l10n);
        let black_panel = PlayerPanel::new(Side::Black, actions, l10n);
        let white_sheet = ScoreSheet::new(Side::White);
        let black_sheet = ScoreSheet::new(Side::Black);
        let white_status = Self::status_line();
        let black_status = Self::status_line();

        // North half: rotated 180° so black's panel, sheet and status
        // line face the far edge. Scene-space order puts the status
        // label first so the flip lands it between board and controls.
        let north_rotated = RotationWidget::new(SmearorRotation::new(180.0))
            .with_gesture_rotation_enabled(false)
            .with_animations_enabled(false);
        let north_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
        north_row.set_halign(gtk4::Align::Center);
        north_row.append(&black_panel.widget);
        north_row.append(&black_sheet.widget);
        let north = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        north.append(&black_status);
        north.append(&north_row);
        north_rotated.set_child(Some(&north));

        // South half faces the near edge directly: status line between
        // board and controls.
        let south_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
        south_row.set_halign(gtk4::Align::Center);
        south_row.append(&white_sheet.widget);
        south_row.append(&white_panel.widget);
        let south = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        south.append(&white_status);
        south.append(&south_row);

        let widget = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        widget.append(&north_rotated);
        widget.append(&scene);
        widget.append(&south);

        Self {
            widget,
            scene,
            white_panel,
            black_panel,
            white_sheet,
            black_sheet,
            white_status,
            black_status,
        }
    }

    /// A centered status-line label with a fixed height so appearing or
    /// disappearing text does not move the board.
    fn status_line() -> gtk4::Label {
        let label = gtk4::Label::new(None);
        label.add_css_class("status-line");
        label.set_xalign(0.5);
        label
    }
}
