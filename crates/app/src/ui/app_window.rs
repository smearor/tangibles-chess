use std::cell::RefCell;
use std::net::TcpStream;
use std::rc::Rc;
use std::time::Duration;

use libadwaita as adw;
use libadwaita::prelude::*;
use tangibles_chess_core::GameConfig;
use tangibles_chess_core::GameSession;
use tangibles_chess_core::SavedGame;
use tangibles_chess_tangibles::MockSource;
use tracing::info;

use crate::controller::GameController;
use crate::controller::set_active_controller;
use crate::controller::with_controller;
use crate::l10n::L10n;
use crate::settings::AppSettings;
use crate::ui::game_screen::GameScreen;
use crate::ui::player_panel::PanelActions;
use crate::ui::setup_screen::SetupScreen;

/// The application window: a stack switching between setup and the game.
pub struct AppWindow {
    /// The GTK window object.
    pub window: adw::ApplicationWindow,
}

impl AppWindow {
    /// Builds the window and wires both screens to the controller.
    /// `mock` replaces the daemon with an injectable `MockSource`;
    /// `mcp_listen` enables the game-domain MCP endpoint.
    pub fn new(app: &adw::Application, settings: AppSettings, l10n: Rc<L10n>, mock: bool, mcp_listen: Option<std::net::SocketAddr>) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(l10n.msg("app-title"))
            .default_width(-1)
            .default_height(-1)
            .build();

        let stack = gtk4::Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);

        let new_game = gio::SimpleAction::new("new-game", None);
        new_game.connect_activate({
            let stack = stack.clone();
            move |_, _| stack.set_visible_child_name("setup")
        });
        window.add_action(&new_game);
        app.set_accels_for_action("win.new-game", &["<Control>n"]);

        let saved = Self::load_autosave();
        let setup = Rc::new(SetupScreen::new(
            saved.as_ref().map(|game| game.config.clone()).unwrap_or_else(|| settings.last_config.clone()),
            saved.clone(),
            l10n.clone(),
            {
                let settings = settings.clone();
                let l10n = l10n.clone();
                let stack = stack.clone();
                move |config| {
                    Self::start_game(&stack, config.clone(), None, settings.clone(), l10n.clone(), mock);
                }
            },
            {
                let settings = settings.clone();
                let l10n = l10n.clone();
                let stack = stack.clone();
                move |saved| {
                    Self::start_game(&stack, saved.config.clone(), Some(saved.clone()), settings.clone(), l10n.clone(), mock);
                }
            },
        ));
        // The dual-facing setup page is tall by design (one card per
        // table edge); a scroll viewport keeps it usable on monitors
        // that are smaller than its natural height.
        let setup_scroll = gtk4::ScrolledWindow::new();
        setup_scroll.set_child(Some(&setup.widget));
        setup_scroll.set_hscrollbar_policy(gtk4::PolicyType::Never);
        setup_scroll.set_propagate_natural_height(false);
        stack.add_named(&setup_scroll, Some("setup"));
        window.set_content(Some(&stack));

        // The MCP endpoint binds at launch, not per game: the handshake
        // phase is exactly what remote agents want to watch, and the
        // listener must survive game → setup → game transitions.
        if let Some(addr) = mcp_listen {
            GameController::attach_mcp(addr);
        }

        // Probe the daemon's TCP reachability on a helper thread; the
        // result hops back over a channel and greys out the tangible
        // mode pickers when offline.
        let daemon_url = settings.daemon_url.clone();
        let l10n_probe = l10n.clone();
        let setup_probe = setup.clone();
        let (probe_tx, probe_rx) = async_channel::bounded(1);
        std::thread::spawn(move || {
            let _ = probe_tx.try_send(probe_endpoint(&daemon_url));
        });
        glib::MainContext::default().spawn_local(async move {
            if let Ok(false) = probe_rx.recv().await {
                setup_probe.set_daemon_available(false, &l10n_probe.msg("daemon-offline"));
            }
        });

        // Debug shortcut: TANGIBLES_DEBUG_GAME=1 skips the setup screen
        // and starts a game with the current config immediately.
        if std::env::var("TANGIBLES_DEBUG_GAME").is_ok() {
            let stack = stack.clone();
            let settings = settings.clone();
            let l10n = l10n.clone();
            let saved = saved.clone();
            glib::idle_add_local_once(move || {
                let saved_game = saved.clone();
                Self::start_game(&stack, settings.last_config.clone(), saved_game, settings.clone(), l10n.clone(), mock);
            });
        }

        Self { window }
    }

    /// Loads the autosave file, if a resumable game exists.
    fn load_autosave() -> Option<SavedGame> {
        let path = glib::user_data_dir().join("tangibles-chess").join("autosave.toml");
        let text = std::fs::read_to_string(path).ok()?;
        SavedGame::from_toml(&text).ok().filter(|game| !game.moves.is_empty())
    }

    /// Creates the session + screen + controller for a new or resumed game.
    fn start_game(stack: &gtk4::Stack, config: GameConfig, saved: Option<SavedGame>, mut settings: AppSettings, l10n: Rc<L10n>, mock: bool) {
        settings.last_config = config.clone();
        settings.sound_enabled = config.sound_enabled;
        settings.save();

        let actions = PanelActions {
            on_undo: Some(Rc::new(|| with_controller(|c| c.borrow_mut().undo()))),
            on_draw: Some(Rc::new(|| with_controller(|c| c.borrow_mut().draw()))),
            on_resign: Some(Rc::new(|color| with_controller(|c| c.borrow_mut().resign(color)))),
            on_rotate: Some(Rc::new(|| with_controller(|c| c.borrow_mut().rotate_cw()))),
        };

        let screen = Rc::new(GameScreen::new(&actions, &l10n, &config));
        let session = match &saved {
            Some(saved) => GameSession::from_saved(saved).unwrap_or_else(|_| GameSession::new(config.clone())),
            None => GameSession::new(config.clone()),
        };
        let controller = Rc::new(RefCell::new(GameController::new(session, screen.clone(), settings, l10n.clone(), None)));
        set_active_controller(controller.clone());

        screen.surface.scene.connect_square({
            let controller = controller.clone();
            move |square| controller.borrow_mut().on_square(square)
        });

        if let Some(old) = stack.child_by_name("game") {
            stack.remove(&old);
        }
        stack.add_named(&screen.widget, Some("game"));
        stack.set_visible_child_name("game");

        if mock {
            let source = MockSource::default();
            GameController::use_mock(&controller, source);
        }
        GameController::begin(controller);
    }
}

/// Probes whether the daemon's GraphQL endpoint answers a minimal query.
fn probe_endpoint(url: &str) -> bool {
    let trimmed = url.trim_start_matches("http://").trim_start_matches("https://");
    let host_port = trimmed.split('/').next().unwrap_or_default();
    let addr = if host_port.contains(':') {
        host_port.to_string()
    } else {
        format!("{host_port}:80")
    };
    use std::io::Read;
    use std::io::Write;
    use std::net::ToSocketAddrs;
    let Some(addr) = addr.to_socket_addrs().ok().and_then(|mut i| i.next()) else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_secs(2)) else {
        info!("daemon probe {addr}: unreachable");
        return false;
    };
    let body = r#"{"query":"{__typename}"}"#;
    let request = format!(
        "POST /graphql HTTP/1.1\r\nHost: {host_port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    let ok = response.contains("\"data\"") && !response.contains("\"errors\"");
    info!("daemon probe {addr}: graphql={ok}");
    ok
}
