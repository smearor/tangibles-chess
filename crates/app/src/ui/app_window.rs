use std::cell::RefCell;
use std::rc::Rc;

use libadwaita as adw;
use libadwaita::prelude::*;
use tangibles_chess_core::GameConfig;
use tangibles_chess_core::GameSession;
use tangibles_chess_tangibles::MockSource;

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
    /// `mock` replaces the daemon with an injectable `MockSource`.
    pub fn new(app: &adw::Application, settings: AppSettings, l10n: Rc<L10n>, mock: bool) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(l10n.msg("app-title"))
            .default_width(-1)
            .default_height(-1)
            .fullscreened(true)
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

        let setup = SetupScreen::new(
            settings.last_config.clone(),
            l10n.clone(),
            {
                let settings = settings.clone();
                let l10n = l10n.clone();
                let stack = stack.clone();
                move |config| {
                    Self::start_game(&stack, config.clone(), settings.clone(), l10n.clone(), mock);
                }
            },
            true,
        );
        stack.add_named(&setup.widget, Some("setup"));
        window.set_content(Some(&stack));

        Self { window }
    }

    /// Creates the session + screen + controller for a new game.
    fn start_game(stack: &gtk4::Stack, config: GameConfig, mut settings: AppSettings, l10n: Rc<L10n>, mock: bool) {
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
        let session = GameSession::new(config);
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
