use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use tangibles_chess_config::BoardTheme;
use tangibles_chess_config::CommitMode;
use tangibles_chess_config::GameConfig;
use tangibles_chess_config::OpponentKind;
use tangibles_chess_config::TangibleMode;
use tangibles_chess_core::SavedGame;
use tangibles_chess_geometry::BoardOrientation;
use tangibles_chess_icons::icon_names;

use crate::l10n::L10n;
use crate::ui::dual_facing_row::DualFacingRow;
use crate::ui::dual_facing_row::Facing;
use crate::ui::setup_preview;

/// The pre-game configuration screen.
///
/// Dual-facing: the settings card is rendered twice (once per table
/// edge), both bound to the same `GameConfig`; either player can set up
/// and start the game. Controls are intentionally large for the lying
/// 65" touchscreen.
pub struct SetupScreen {
    /// Root widget.
    pub widget: gtk4::Box,
    /// Both facing copies of the daemon-gated input options (for greying
    /// them out when the daemon is unreachable).
    daemon_gates: Rc<RefCell<Vec<DaemonGate>>>,
    /// Both facing copies of the "back to game" button (shown only while
    /// a live game exists in the window stack).
    back_buttons: Rc<RefCell<Vec<gtk4::Button>>>,
    /// The shared config sync (for the live-apply change hook).
    sync: Rc<SideSync>,
}

/// The controls of one card copy that are only usable while the
/// tangible-objects daemon is reachable.
struct DaemonGate {
    /// The input-mode row; its subtitle explains an offline daemon.
    row: adw::ActionRow,
    /// Option buttons that require the daemon connection.
    buttons: Vec<gtk4::ToggleButton>,
}

/// A preferences row plus its per-value toggle buttons.
struct PickerRow<T> {
    /// The row added to the preferences group.
    row: adw::ActionRow,
    /// Toggle button of each option value (for per-option sensitivity,
    /// e.g. daemon-gated input modes or the unavailable engine).
    buttons: Vec<(T, gtk4::ToggleButton)>,
}

/// Shared state and callbacks threaded into both facing copies of the
/// setup card.
#[derive(Clone)]
struct SetupContext {
    /// The configuration both cards edit.
    config: Rc<RefCell<GameConfig>>,
    /// Mirrors control state between the two card copies.
    sync: Rc<SideSync>,
    /// Localization.
    l10n: Rc<L10n>,
    /// Start callback for a fresh game.
    on_start: Rc<dyn Fn(&GameConfig)>,
    /// Resume callback for a previously autosaved game.
    on_resume: Rc<dyn Fn(&SavedGame)>,
    /// Every facing copy of the daemon-gated input controls.
    daemon_gates: Rc<RefCell<Vec<DaemonGate>>>,
    /// Every facing copy of the "back to game" button.
    back_buttons: Rc<RefCell<Vec<gtk4::Button>>>,
    /// The autosaved game offered for resume, if any.
    saved: Rc<Option<SavedGame>>,
}

/// Renders the shared config onto one control copy.
type Applier = Box<dyn Fn(&GameConfig)>;
/// Shared callback fired once after every config change.
type ConfigChanged = Rc<dyn Fn(&GameConfig)>;

/// Cross-edge synchronization for the dual-facing setup card.
///
/// Every control exists twice (north + south copy) but edits one shared
/// `GameConfig`. Each copy registers an `apply` closure that renders the
/// config onto its own widget; a change on either side is then mirrored
/// to the peer. The `mirroring` flag suppresses the re-triggered change
/// handlers while appliers run.
#[derive(Default)]
struct SideSync {
    /// Reentrancy guard while a mirror pass is in progress.
    mirroring: Cell<bool>,
    /// One applier per control copy; renders config onto that widget.
    appliers: RefCell<Vec<Applier>>,
    /// Optional hook invoked once after every config change (used to
    /// apply live-safe options to a running game).
    on_changed: RefCell<Option<ConfigChanged>>,
}

impl SideSync {
    /// Registers a control copy so future `update`/`refresh` calls mirror
    /// state onto it.
    fn register(&self, apply: impl Fn(&GameConfig) + 'static) {
        self.appliers.borrow_mut().push(Box::new(apply));
    }

    /// Sets the hook fired once after every user config change.
    fn set_on_changed(&self, callback: ConfigChanged) {
        *self.on_changed.borrow_mut() = Some(callback);
    }

    /// Applies `change` to the shared config and mirrors the result onto
    /// every registered control copy. Calls arriving while a mirror pass
    /// is running (i.e. re-triggered change handlers) are ignored.
    fn update(&self, config: &RefCell<GameConfig>, change: impl FnOnce(&mut GameConfig)) {
        if self.mirroring.replace(true) {
            return;
        }
        change(&mut config.borrow_mut());
        self.mirror(config);
        if let Some(on_changed) = self.on_changed.borrow().as_ref() {
            on_changed(&config.borrow());
        }
        self.mirroring.set(false);
    }

    /// Re-renders every registered control from the current config -
    /// used once after both cards are built to apply the initial state.
    fn refresh(&self, config: &RefCell<GameConfig>) {
        if self.mirroring.replace(true) {
            return;
        }
        self.mirror(config);
        self.mirroring.set(false);
    }

    /// Runs all appliers; caller must hold the `mirroring` guard.
    fn mirror(&self, config: &RefCell<GameConfig>) {
        let config = config.borrow();
        for apply in self.appliers.borrow().iter() {
            apply(&config);
        }
    }
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
        let ctx = SetupContext {
            config: Rc::new(RefCell::new(initial)),
            sync: Rc::new(SideSync::default()),
            l10n,
            on_start: Rc::new(on_start),
            on_resume: Rc::new(on_resume),
            daemon_gates: Rc::new(RefCell::new(Vec::new())),
            back_buttons: Rc::new(RefCell::new(Vec::new())),
            saved: Rc::new(saved),
        };

        let row = DualFacingRow::new({
            let ctx = ctx.clone();
            move |facing| Self::build_card(facing, &ctx)
        });
        // Render the initial config onto both card copies.
        ctx.sync.refresh(&ctx.config);
        Self {
            widget: row.widget,
            daemon_gates: ctx.daemon_gates.clone(),
            back_buttons: ctx.back_buttons.clone(),
            sync: ctx.sync,
        }
    }

    /// Enables/disables the daemon-dependent input options on both card
    /// copies; the input row explains an outage in its subtitle.
    pub fn set_daemon_available(&self, available: bool, offline_text: &str) {
        for gate in self.daemon_gates.borrow().iter() {
            for button in &gate.buttons {
                button.set_sensitive(available);
            }
            gate.row.set_subtitle(if available { "" } else { offline_text });
        }
    }

    /// Registers a callback fired once after every config change - used
    /// to apply live-safe options (theme, sound, undo) to a running game.
    pub fn set_live_config_callback(&self, callback: ConfigChanged) {
        self.sync.set_on_changed(callback);
    }

    /// Shows/hides the "back to game" buttons on both card copies -
    /// visible only while a live game is waiting in the window stack.
    pub fn set_game_running(&self, running: bool) {
        for button in self.back_buttons.borrow().iter() {
            button.set_visible(running);
        }
    }

    /// One facing half of the setup card.
    fn build_card(facing: Facing, ctx: &SetupContext) -> gtk4::Widget {
        let l10n = &ctx.l10n;
        let card = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        card.set_halign(gtk4::Align::Center);
        card.set_valign(gtk4::Align::Center);
        card.set_vexpand(true);
        card.add_css_class("setup-card");

        let group = adw::PreferencesGroup::new();
        group.set_width_request(720);

        // Input mode: one preview button per mode. Options that need the
        // tangible daemon are gated via `daemon_gates` and greyed out
        // when the daemon is unreachable.
        let input = Self::picker_row(
            ctx,
            &l10n.msg("setup-mode-label"),
            vec![
                (
                    TangibleMode::NoTangibles,
                    l10n.msg("setup-mode-touch"),
                    setup_preview::input_preview(TangibleMode::NoTangibles),
                ),
                (
                    TangibleMode::TangiblesRendered,
                    l10n.msg("setup-mode-tangibles-rendered"),
                    setup_preview::input_preview(TangibleMode::TangiblesRendered),
                ),
                (
                    TangibleMode::Tangibles,
                    l10n.msg("setup-mode-tangibles"),
                    setup_preview::input_preview(TangibleMode::Tangibles),
                ),
            ],
            |config| config.tangible_mode,
            |config, mode| config.tangible_mode = mode,
        );
        let gated = input
            .buttons
            .iter()
            .filter(|(mode, _)| mode.uses_tangibles())
            .map(|(_, button)| button.clone())
            .collect();
        ctx.daemon_gates.borrow_mut().push(DaemonGate {
            row: input.row.clone(),
            buttons: gated,
        });
        group.add(&input.row);

        // Board theme: one preview button per material.
        group.add(
            &Self::picker_row(
                ctx,
                &l10n.msg("setup-theme-label"),
                vec![
                    (BoardTheme::WalnutMaple, l10n.msg("setup-theme-walnut"), setup_preview::theme_preview(BoardTheme::WalnutMaple)),
                    (BoardTheme::Slate, l10n.msg("setup-theme-slate"), setup_preview::theme_preview(BoardTheme::Slate)),
                    (
                        BoardTheme::TournamentGreen,
                        l10n.msg("setup-theme-felt"),
                        setup_preview::theme_preview(BoardTheme::TournamentGreen),
                    ),
                    (BoardTheme::Marble, l10n.msg("setup-theme-marble"), setup_preview::theme_preview(BoardTheme::Marble)),
                ],
                |config| config.theme,
                |config, theme| config.theme = theme,
            )
            .row,
        );

        // White's edge. The labels are viewer-relative: "near edge" means
        // the edge in front of whoever reads this card copy - physical
        // south for the upright card, north for the flipped one.
        let (near, far) = match facing {
            Facing::South => (BoardOrientation::Deg0, BoardOrientation::Deg180),
            Facing::North => (BoardOrientation::Deg180, BoardOrientation::Deg0),
        };
        group.add(
            &Self::picker_row(
                ctx,
                &l10n.msg("setup-orientation-label"),
                vec![
                    (near, l10n.msg("setup-edge-south"), setup_preview::orientation_preview(true)),
                    (far, l10n.msg("setup-edge-north"), setup_preview::orientation_preview(false)),
                ],
                |config| config.orientation,
                |config, orientation| config.orientation = orientation,
            )
            .row,
        );

        // Commit mode: one preview button per confirmation style. Only
        // meaningful with physical pieces (the confirm zone is a
        // `btn-*` marker zone) - greyed out for touch-only games.
        let commit = Self::picker_row(
            ctx,
            &l10n.msg("setup-commit-label"),
            vec![
                (CommitMode::OnPlace, l10n.msg("setup-commit-place"), setup_preview::commit_preview(CommitMode::OnPlace)),
                (
                    CommitMode::OnConfirmZone,
                    l10n.msg("setup-commit-zone"),
                    setup_preview::commit_preview(CommitMode::OnConfirmZone),
                ),
            ],
            |config| config.commit_mode,
            |config, commit_mode| config.commit_mode = commit_mode,
        );
        ctx.sync.register({
            let row = commit.row.clone();
            let buttons: Vec<gtk4::ToggleButton> = commit.buttons.iter().map(|(_, button)| button.clone()).collect();
            move |config| {
                let usable = config.tangible_mode.uses_tangibles();
                row.set_sensitive(usable);
                for button in &buttons {
                    button.set_sensitive(usable);
                }
            }
        });
        group.add(&commit.row);

        // Opponent: one preview button per kind. The engine is reserved
        // for M3 - visible but greyed out and unselectable.
        let opponent = Self::picker_row(
            ctx,
            &l10n.msg("setup-opponent-label"),
            vec![
                (OpponentKind::Human, l10n.msg("setup-human"), setup_preview::opponent_preview(OpponentKind::Human)),
                (OpponentKind::Engine, l10n.msg("setup-engine"), setup_preview::opponent_preview(OpponentKind::Engine)),
            ],
            |config| config.black,
            |config, kind| config.black = kind,
        );
        for (kind, button) in &opponent.buttons {
            if *kind == OpponentKind::Engine {
                button.set_sensitive(false);
            }
        }
        group.add(&opponent.row);

        // Placement handshake and sound in one row.
        let toggles = adw::PreferencesRow::new();
        let toggles_content = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
        toggles_content.add_css_class("setup-switch-row");
        let (handshake_cell, handshake_switch) = Self::switch_cell(&l10n.msg("setup-handshake-label"));
        handshake_cell.set_tooltip_text(Some(&l10n.msg("setup-handshake-subtitle")));
        toggles_content.append(&handshake_cell);
        let (undo_cell, undo_switch) = Self::switch_cell(&l10n.msg("setup-undo-label"));
        toggles_content.append(&undo_cell);
        let (sound_cell, sound_switch) = Self::switch_cell(&l10n.msg("setup-sound-label"));
        toggles_content.append(&sound_cell);
        toggles.set_child(Some(&toggles_content));

        handshake_switch.connect_active_notify({
            let ctx = ctx.clone();
            move |switch| ctx.sync.update(&ctx.config, |config| config.require_placement = switch.is_active())
        });
        ctx.sync.register({
            let handshake_switch = handshake_switch.clone();
            move |config| handshake_switch.set_active(config.require_placement)
        });
        // The handshake only exists for physical pieces - grey the cell
        // out for touch-only games.
        ctx.sync.register({
            let handshake_cell = handshake_cell.clone();
            move |config| handshake_cell.set_sensitive(config.tangible_mode.uses_tangibles())
        });
        // Whether undo/redo takebacks are allowed in this game.
        undo_switch.connect_active_notify({
            let ctx = ctx.clone();
            move |switch| ctx.sync.update(&ctx.config, |config| config.allow_undo = switch.is_active())
        });
        ctx.sync.register({
            let undo_switch = undo_switch.clone();
            move |config| undo_switch.set_active(config.allow_undo)
        });
        // Sound: a global app setting, so it mirrors between both copies
        // like every other control.
        sound_switch.connect_active_notify({
            let ctx = ctx.clone();
            move |switch| ctx.sync.update(&ctx.config, |config| config.sound_enabled = switch.is_active())
        });
        ctx.sync.register({
            let sound_switch = sound_switch.clone();
            move |config| sound_switch.set_active(config.sound_enabled)
        });
        group.add(&toggles);

        card.append(&group);

        // Bottom action row: back to a live game (if any), start,
        // resume (if a saved game exists), quit.
        let actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 16);
        actions.set_halign(gtk4::Align::Center);
        actions.set_margin_top(32);

        let back_icon = gtk4::Image::from_icon_name(icon_names::TCH_PLAY_SYMBOLIC);
        back_icon.set_pixel_size(32);
        let back = gtk4::Button::new();
        back.set_child(Some(&back_icon));
        back.add_css_class("pill");
        back.set_tooltip_text(Some(&l10n.msg("setup-back-to-game")));
        back.set_action_name(Some("win.back-to-game"));
        back.set_size_request(84, 84);
        back.set_visible(false);
        ctx.back_buttons.borrow_mut().push(back.clone());
        actions.append(&back);

        let start = gtk4::Button::builder().label(l10n.msg("setup-start")).build();
        start.add_css_class("pill");
        start.add_css_class("suggested-action");
        start.set_size_request(360, 84);
        start.connect_clicked({
            let ctx = ctx.clone();
            move |_| {
                let config = ctx.config.borrow().clone();
                (ctx.on_start)(&config);
            }
        });
        actions.append(&start);

        if let Some(saved_game) = ctx.saved.as_ref() {
            let resume = gtk4::Button::builder()
                .label(format!("{} – {} {}", l10n.msg("setup-resume"), saved_game.moves.len(), l10n.msg("setup-resume-moves")))
                .build();
            resume.add_css_class("pill");
            resume.set_size_request(360, 84);
            resume.connect_clicked({
                let ctx = ctx.clone();
                let saved_game = saved_game.clone();
                move |_| (ctx.on_resume)(&saved_game)
            });
            actions.append(&resume);
        }

        let quit_icon = gtk4::Image::from_icon_name(icon_names::TCH_X_SYMBOLIC);
        quit_icon.set_pixel_size(32);
        let quit = gtk4::Button::new();
        quit.set_child(Some(&quit_icon));
        quit.add_css_class("pill");
        quit.set_tooltip_text(Some(&l10n.msg("setup-quit")));
        quit.set_size_request(84, 84);
        quit.connect_clicked(|button| {
            if let Some(window) = button.root().and_then(|root| root.downcast::<gtk4::Window>().ok()) {
                window.close();
            }
        });
        actions.append(&quit);

        card.append(&actions);
        // Each facing half scrolls within its own half of the surface:
        // on small screens the card stays inside its slot instead of
        // pushing the whole dual-facing row past the screen edge.
        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_child(Some(&card));
        scroll.set_hscrollbar_policy(gtk4::PolicyType::Never);
        scroll.set_propagate_natural_height(false);
        scroll.upcast()
    }

    /// A label + toggle cell for the combined handshake/sound row.
    fn switch_cell(label: &str) -> (gtk4::Box, gtk4::Switch) {
        let cell = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        cell.set_hexpand(true);
        let text = gtk4::Label::new(Some(label));
        text.set_xalign(0.0);
        text.set_hexpand(true);
        let switch = gtk4::Switch::new();
        switch.set_valign(gtk4::Align::Center);
        cell.append(&text);
        cell.append(&switch);
        (cell, switch)
    }

    /// An `adw::ActionRow` whose suffix is a group of radio-style preview
    /// buttons - one per `options` entry. Activating a button writes its
    /// value into the shared config; the `SideSync` mirror then reflects
    /// the choice onto both card copies.
    fn picker_row<T: Copy + PartialEq + 'static>(
        ctx: &SetupContext,
        title: &str,
        options: Vec<(T, String, gtk4::DrawingArea)>,
        get: impl Fn(&GameConfig) -> T + 'static,
        set: impl Fn(&mut GameConfig, T) + 'static,
    ) -> PickerRow<T> {
        let get = Rc::new(get);
        let set = Rc::new(set);
        let row = adw::ActionRow::new();
        row.set_title(title);
        let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        buttons.set_valign(gtk4::Align::Center);
        let mut leader: Option<gtk4::ToggleButton> = None;
        let mut picker_buttons = Vec::new();
        for (value, label, preview) in options {
            let toggle = gtk4::ToggleButton::new();
            toggle.add_css_class("setup-option");
            let cell = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
            cell.append(&preview);
            let caption = gtk4::Label::new(Some(&label));
            caption.add_css_class("setup-option-caption");
            caption.set_justify(gtk4::Justification::Center);
            cell.append(&caption);
            toggle.set_child(Some(&cell));
            match &leader {
                Some(leader) => toggle.set_group(Some(leader)),
                None => leader = Some(toggle.clone()),
            }

            ctx.sync.register({
                let toggle = toggle.clone();
                let get = get.clone();
                move |config| {
                    if get(config) == value {
                        toggle.set_active(true);
                    }
                }
            });
            // `clicked` fires only for real user interaction, after the
            // toggle group has settled - `toggled` would also fire mid-
            // transition for the deactivated sibling while the config
            // still holds its old value.
            toggle.connect_clicked({
                let ctx = ctx.clone();
                let set = set.clone();
                move |button| {
                    if button.is_active() {
                        ctx.sync.update(&ctx.config, |config| set(config, value));
                    } else {
                        // Deselecting the active button leaves an empty
                        // group - re-assert the stored value.
                        ctx.sync.refresh(&ctx.config);
                    }
                }
            });
            buttons.append(&toggle);
            picker_buttons.push((value, toggle));
        }
        row.add_suffix(&buttons);
        PickerRow { row, buttons: picker_buttons }
    }
}
