use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use tangibles_chess_core::BoardTheme;
use tangibles_chess_core::CommitMode;
use tangibles_chess_core::GameConfig;
use tangibles_chess_core::OpponentKind;
use tangibles_chess_core::SavedGame;
use tangibles_chess_core::TangibleMode;

use crate::l10n::L10n;
use crate::ui::dual_facing_row::DualFacingRow;

/// The pre-game configuration screen.
///
/// Dual-facing: the settings card is rendered twice (once per table
/// edge), both bound to the same `GameConfig`; either player can set up
/// and start the game. Controls are intentionally large for the lying
/// 65" touchscreen.
pub struct SetupScreen {
    /// Root widget.
    pub widget: gtk4::Box,
    /// Both facing copies of the tangible-mode selector (for disabling
    /// when the daemon is unreachable).
    mode_rows: Rc<RefCell<Vec<adw::ComboRow>>>,
}

impl SetupScreen {
    /// Builds the screen; `on_start` fires with the chosen config,
    /// `on_resume` with a previously autosaved game.
    pub fn new(
        initial: GameConfig,
        saved: Option<SavedGame>,
        l10n: Rc<L10n>,
        on_start: impl Fn(&GameConfig) + 'static,
        on_resume: impl Fn(&SavedGame) + 'static,
    ) -> Self {
        let config = Rc::new(RefCell::new(initial));
        let on_start = Rc::new(on_start);
        let on_resume = Rc::new(on_resume);
        let mode_rows = Rc::new(RefCell::new(Vec::new()));
        let saved = Rc::new(saved);

        let row = DualFacingRow::new({
            let config = config.clone();
            let on_start = on_start.clone();
            let on_resume = on_resume.clone();
            let mode_rows = mode_rows.clone();
            let saved = saved.clone();
            let l10n = l10n.clone();
            move || Self::build_card(config.clone(), l10n.clone(), on_start.clone(), on_resume.clone(), mode_rows.clone(), saved.clone())
        });
        Self { widget: row.widget, mode_rows }
    }

    /// Enables/disables the tangible-mode selectors on both card copies.
    pub fn set_daemon_available(&self, available: bool, offline_text: &str) {
        for row in self.mode_rows.borrow().iter() {
            row.set_sensitive(available);
            if !available {
                row.set_subtitle(offline_text);
            }
        }
    }

    /// One facing half of the setup card.
    fn build_card(
        config: Rc<RefCell<GameConfig>>,
        l10n: Rc<L10n>,
        on_start: Rc<dyn Fn(&GameConfig)>,
        on_resume: Rc<dyn Fn(&SavedGame)>,
        mode_rows: Rc<RefCell<Vec<adw::ComboRow>>>,
        saved: Rc<Option<SavedGame>>,
    ) -> gtk4::Widget {
        let card = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        card.set_halign(gtk4::Align::Center);
        card.set_valign(gtk4::Align::Center);
        card.set_vexpand(true);
        card.add_css_class("setup-card");

        let heading = gtk4::Label::new(Some(&l10n.msg("setup-heading")));
        heading.add_css_class("title-1");
        card.append(&heading);

        let group = adw::PreferencesGroup::new();
        group.set_width_request(720);

        // Tangible mode.
        let mode = adw::ComboRow::new();
        mode.set_title(&l10n.msg("setup-mode-label"));
        let mode_model = gtk4::StringList::new(&[
            &l10n.msg("setup-mode-touch"),
            &l10n.msg("setup-mode-tangibles-rendered"),
            &l10n.msg("setup-mode-tangibles"),
        ]);
        mode.set_model(Some(&mode_model));
        let mode_index = match config.borrow().tangible_mode {
            TangibleMode::NoTangibles => 0u32,
            TangibleMode::TangiblesRendered => 1,
            TangibleMode::Tangibles => 2,
        };
        mode.set_selected(mode_index);
        mode.connect_selected_notify({
            let config = config.clone();
            move |row| {
                config.borrow_mut().tangible_mode = match row.selected() {
                    1 => TangibleMode::TangiblesRendered,
                    2 => TangibleMode::Tangibles,
                    _ => TangibleMode::NoTangibles,
                };
            }
        });
        mode_rows.borrow_mut().push(mode.clone());
        group.add(&mode);

        // Board theme.
        let theme = adw::ComboRow::new();
        theme.set_title(&l10n.msg("setup-theme-label"));
        theme.set_model(Some(&gtk4::StringList::new(&[
            &l10n.msg("setup-theme-walnut"),
            &l10n.msg("setup-theme-slate"),
            &l10n.msg("setup-theme-felt"),
        ])));
        theme.set_selected(match config.borrow().theme {
            BoardTheme::WalnutMaple => 0,
            BoardTheme::Slate => 1,
            BoardTheme::TournamentGreen => 2,
        });
        theme.connect_selected_notify({
            let config = config.clone();
            move |row| {
                config.borrow_mut().theme = match row.selected() {
                    1 => BoardTheme::Slate,
                    2 => BoardTheme::TournamentGreen,
                    _ => BoardTheme::WalnutMaple,
                };
            }
        });
        group.add(&theme);

        // White's edge (board orientation).
        let edge = adw::ComboRow::new();
        edge.set_title(&l10n.msg("setup-orientation-label"));
        edge.set_model(Some(&gtk4::StringList::new(&[&l10n.msg("setup-edge-south"), &l10n.msg("setup-edge-north")])));
        edge.set_selected(match config.borrow().orientation {
            tangibles_chess_core::BoardOrientation::Deg0 => 0,
            tangibles_chess_core::BoardOrientation::Deg180 => 1,
            _ => 0,
        });
        edge.connect_selected_notify({
            let config = config.clone();
            move |row| {
                config.borrow_mut().orientation = if row.selected() == 1 {
                    tangibles_chess_core::BoardOrientation::Deg180
                } else {
                    tangibles_chess_core::BoardOrientation::Deg0
                };
            }
        });
        group.add(&edge);

        // Commit mode (only meaningful with tangibles).
        let commit = adw::ComboRow::new();
        commit.set_title(&l10n.msg("setup-commit-label"));
        commit.set_model(Some(&gtk4::StringList::new(&[&l10n.msg("setup-commit-place"), &l10n.msg("setup-commit-zone")])));
        commit.set_selected(match config.borrow().commit_mode {
            CommitMode::OnPlace => 0,
            CommitMode::OnConfirmZone => 1,
        });
        commit.connect_selected_notify({
            let config = config.clone();
            move |row| {
                config.borrow_mut().commit_mode = if row.selected() == 1 {
                    CommitMode::OnConfirmZone
                } else {
                    CommitMode::OnPlace
                };
            }
        });
        group.add(&commit);

        // Placement handshake (only meaningful with tangibles).
        let handshake = adw::SwitchRow::new();
        handshake.set_title(&l10n.msg("setup-handshake-label"));
        handshake.set_subtitle(&l10n.msg("setup-handshake-subtitle"));
        handshake.set_active(config.borrow().require_placement);
        handshake.connect_active_notify({
            let config = config.clone();
            move |row| {
                config.borrow_mut().require_placement = row.is_active();
            }
        });
        group.add(&handshake);

        // Opponent (engine reserved for M3 - visible but disabled).
        let opponent = adw::ComboRow::new();
        opponent.set_title(&l10n.msg("setup-opponent-label"));
        opponent.set_model(Some(&gtk4::StringList::new(&[&l10n.msg("setup-human"), &l10n.msg("setup-engine")])));
        opponent.set_selected(match config.borrow().black {
            OpponentKind::Human => 0,
            OpponentKind::Engine => 1,
        });
        opponent.set_sensitive(false);
        group.add(&opponent);

        // Sound toggle.
        let sound = adw::SwitchRow::new();
        sound.set_title(&l10n.msg("setup-sound-label"));
        sound.set_active(config.borrow().sound_enabled);
        sound.connect_active_notify({
            let config = config.clone();
            move |row| {
                config.borrow_mut().sound_enabled = row.is_active();
            }
        });
        group.add(&sound);

        card.append(&group);

        let start = gtk4::Button::builder()
            .label(l10n.msg("setup-start"))
            .halign(gtk4::Align::Center)
            .margin_top(24)
            .build();
        start.add_css_class("pill");
        start.add_css_class("suggested-action");
        start.set_size_request(360, 84);
        start.connect_clicked(move |_| {
            let config = config.borrow().clone();
            on_start(&config);
        });
        card.append(&start);

        if let Some(saved_game) = saved.as_ref() {
            let resume = gtk4::Button::builder()
                .label(format!("{} – {} {}", l10n.msg("setup-resume"), saved_game.moves.len(), l10n.msg("setup-resume-moves")))
                .halign(gtk4::Align::Center)
                .margin_top(12)
                .build();
            resume.add_css_class("pill");
            resume.set_size_request(360, 64);
            let saved_game = saved_game.clone();
            let on_resume = on_resume.clone();
            resume.connect_clicked(move |_| on_resume(&saved_game));
            card.append(&resume);
        }

        card.upcast()
    }
}
