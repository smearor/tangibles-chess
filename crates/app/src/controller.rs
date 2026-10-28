use std::cell::RefCell;
use std::rc::Rc;

use gdk4::prelude::DisplayExt;
use gdk4::prelude::MonitorExt;
use gdk4::prelude::PaintableExt;
use gdk4::prelude::StaticType;
use gdk4::prelude::SurfaceExt;
use gdk4::prelude::TextureExt;
use gtk4::prelude::Cast;
use gtk4::prelude::GskRendererExt;
use gtk4::prelude::NativeExt;
use gtk4::prelude::SnapshotExt;
use gtk4::prelude::WidgetExt;
use libadwaita as adw;
use shakmaty::Color;
use shakmaty::KnownOutcome;
use shakmaty::Position;
use shakmaty::Square;
use tangibles_chess_core::DrawReason;
use tangibles_chess_core::GameConfig;
use tangibles_chess_core::GamePhase;
use tangibles_chess_core::GameSession;
use tangibles_chess_core::MoveOutcome;
use tangibles_chess_core::MoveRequest;
use tangibles_chess_core::TangibleMode;
use tangibles_chess_icons::icon_names;
use tangibles_chess_tangibles::BoardGeometry;
use tangibles_chess_tangibles::FieldEvent;
use tangibles_chess_tangibles::FieldEventKind;
use tangibles_chess_tangibles::FieldSetBuilder;
use tangibles_chess_tangibles::GraphQlSource;
use tangibles_chess_tangibles::InferenceOutput;
use tangibles_chess_tangibles::MockSource;
use tangibles_chess_tangibles::MoveInference;
use tangibles_chess_tangibles::OccupancyProbe;
use tangibles_chess_tangibles::PlacementGate;
use tangibles_chess_tangibles::TableEdge;
use tangibles_chess_tangibles::TangibleSource;
use tangibles_chess_tangibles::ZoneAction;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::l10n::L10n;
use crate::settings::AppSettings;
use crate::sound::SoundPlayer;
use crate::tangible_host::TangibleHost;
use crate::ui::chess_scene::FlashRole;
use crate::ui::chess_scene::IllegalFlash;
use crate::ui::game_result::GameResult;
use crate::ui::game_screen::GameScreen;
use crate::ui::promotion_picker::PromotionPicker;

/// Which tangible backend is active.
enum Source {
    /// Real daemon connection.
    Daemon(Rc<GraphQlSource>),
    /// Development mock.
    Mock(MockSource),
}

/// Live state of the whole game - the glue between `GameSession`
/// (rules), the `TangibleSource` (physical input) and the UI.
pub struct GameController {
    /// Chess session (position, history, phase).
    pub session: GameSession,
    /// Persisted settings (theme, sound, daemon url).
    pub settings: AppSettings,
    /// Texts.
    pub l10n: Rc<L10n>,
    /// The active game screen.
    pub screen: Rc<GameScreen>,
    /// Sound effects.
    pub sound: SoundPlayer,
    /// Tokio runtime + daemon connection.
    host: Option<TangibleHost>,
    /// Active tangible backend.
    source: Option<Source>,
    /// Field↔square mapping for the current orientation.
    geometry: BoardGeometry,
    /// Occupancy state of board fields and zones.
    probe: OccupancyProbe,
    /// Field events → move intents.
    inference: MoveInference,
    /// Start-handshake gate.
    gate: PlacementGate,
    /// Current touch selection (`NoTangibles`).
    selected: Option<Square>,
    /// Move awaiting a promotion pick.
    pending_promotion: Option<MoveRequest>,
    /// The promotion picker overlay widget (to remove on pick).
    promotion_widget: RefCell<Option<gtk4::Widget>>,
    /// Last applied move's squares for the highlight trace.
    last_move: Option<(Square, Square)>,
    /// Board rectangle in screen mm for the registered field set.
    board_mm: (f64, f64, f64),
    /// Whether the game-over overlay was already shown.
    result_shown: bool,
    /// Generation counter for the illegal-move flash.
    illegal_seq: u64,
    /// Last rejected tangible move (`from`, `to`): the physical marker
    /// is misplaced; a lift+place back onto `from` is accepted quietly.
    rejected_move: Option<(Square, Square)>,
    /// Ply cursor while replaying a finished game (`None` = live).
    replay_index: Option<usize>,
    /// The game-over overlay widget, kept for removal when entering
    /// replay mode.
    result_widget: Option<gtk4::Widget>,
}

/// How long a rejected move's squares flash red (ms).
const ILLEGAL_FLASH_MS: u64 = 800;

thread_local! {
    /// The active controller, reachable from widget callbacks that do
    /// not hold it directly (promotion picker, panel actions).
    static ACTIVE_CONTROLLER: RefCell<Option<Rc<RefCell<GameController>>>> = const { RefCell::new(None) };
}

/// Registers the running controller globally (single window app).
pub fn set_active_controller(controller: Rc<RefCell<GameController>>) {
    ACTIVE_CONTROLLER.with(|slot| *slot.borrow_mut() = Some(controller));
}

/// Runs `f` with the active controller, if registered.
pub fn with_controller(f: impl FnOnce(&Rc<RefCell<GameController>>)) {
    ACTIVE_CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow().as_ref() {
            f(controller);
        }
    });
}

impl GameController {
    /// A controller for a fresh session shown on `screen`.
    pub fn new(session: GameSession, screen: Rc<GameScreen>, settings: AppSettings, l10n: Rc<L10n>, host: Option<TangibleHost>) -> Self {
        let white_edge = TableEdge::South;
        let geometry = BoardGeometry::new(screen.orientation(), white_edge);
        let inference = MoveInference::new(geometry, session.config().commit_mode);
        let gate = PlacementGate::new(geometry);
        let sound = SoundPlayer::new(session.config().sound_enabled);
        // Score-sheet ply clicks jump the replay cursor (finished games
        // only - `replay_goto` guards otherwise).
        let on_ply = Rc::new(|ply: usize| {
            with_controller(|controller| controller.borrow_mut().replay_goto(ply));
        });
        screen.surface.white_sheet.set_on_ply(on_ply.clone());
        screen.surface.black_sheet.set_on_ply(on_ply);
        Self {
            session,
            settings,
            l10n,
            screen,
            sound,
            host,
            source: None,
            geometry,
            probe: OccupancyProbe::default(),
            inference,
            gate,
            selected: None,
            pending_promotion: None,
            promotion_widget: RefCell::new(None),
            last_move: None,
            board_mm: (0.0, 0.0, 0.0),
            result_shown: false,
            illegal_seq: 0,
            rejected_move: None,
            replay_index: None,
            result_widget: None,
        }
    }

    /// Starts the configured game: session phase transition, tangible
    /// activation when the mode needs the daemon.
    pub fn begin(controller: Rc<RefCell<Self>>) {
        let uses_tangibles = controller.borrow().session.config().tangible_mode.uses_tangibles();
        let has_mock = matches!(controller.borrow().source, Some(Source::Mock(_)));
        controller.borrow_mut().session.begin();
        controller.borrow_mut().sync_scene();
        if uses_tangibles {
            // Show the placement gate immediately - without a first
            // field event the phase would otherwise sit invisibly in
            // `AwaitingPlacement`.
            controller.borrow_mut().update_placement();
        }
        if uses_tangibles && has_mock && std::env::var_os("TANGIBLES_MOCK_MANUAL").is_none() {
            // Dev convenience: let the mock pretend the current position
            // is being placed piece by piece, so the handshake walks its
            // real path (event pump -> gate -> confirm) and the ceremony
            // lighting is visible without hardware. Set
            // TANGIBLES_MOCK_MANUAL=1 to keep the gate unsatisfied.
            let geometry = controller.borrow().geometry;
            let position = controller.borrow().session.position().clone();
            if let Some(Source::Mock(mock)) = &controller.borrow().source {
                let mock = mock.clone();
                let fields: Rc<RefCell<std::vec::IntoIter<String>>> = Rc::new(RefCell::new(
                    Square::ALL
                        .iter()
                        .filter(|square| position.board().piece_at(**square).is_some())
                        .filter_map(|square| BoardGeometry::field_name(geometry.square_to_field(*square)))
                        .collect::<Vec<_>>()
                        .into_iter(),
                ));
                glib::timeout_add_local(std::time::Duration::from_millis(120), move || match fields.borrow_mut().next() {
                    Some(field) => {
                        mock.place(&field);
                        glib::ControlFlow::Continue
                    }
                    None => glib::ControlFlow::Break,
                });
            }
        }
        if uses_tangibles && !has_mock {
            Self::activate_tangibles(controller.clone());
        }
        controller.borrow().screen.set_rotation_locked(uses_tangibles);
    }

    /// Connects to the daemon, registers the chess field set and starts
    /// the event pump (all off the main thread; results hop back via
    /// `idle_add`).
    fn activate_tangibles(controller: Rc<RefCell<Self>>) {
        let url = controller.borrow().settings.daemon_url.clone();
        if controller.borrow().host.is_none() {
            match TangibleHost::new() {
                Ok(host) => controller.borrow_mut().host = Some(host),
                Err(err) => {
                    warn!("no tokio runtime: {err}");
                    return;
                }
            }
        }
        // The connect + registration happen on the runtime but are
        // blocking calls; run them on a side thread to keep GTK fluid.
        let (tx, rx) = async_channel::bounded(1);
        std::thread::spawn(move || {
            let host = TangibleHost::new();
            let result = host.ok().and_then(|host| host.connect(&url).ok().map(|s| (host, s)));
            let _ = tx.try_send(result);
        });
        let controller_weak = Rc::downgrade(&controller);
        glib::MainContext::default().spawn_local(async move {
            let Ok(result) = rx.recv().await else {
                return;
            };
            let Some(controller) = controller_weak.upgrade() else {
                return;
            };
            match result {
                Some((host, source)) => {
                    controller.borrow_mut().host = Some(host);
                    Self::start_event_pump(controller.clone(), source.events());
                    controller.borrow_mut().source = Some(Source::Daemon(Rc::new(source)));
                    controller.borrow_mut().sync_scene();
                    info!("tangible daemon connected");
                    Self::register_when_ready(controller.clone());
                }
                None => {
                    controller.borrow().screen.daemon_banner.set_revealed(true);
                    warn!("daemon connection failed");
                }
            }
        });
    }

    /// Registers the chess field set once the scene has a real
    /// allocation - before that, `board_rect_mm` has nothing to scale
    /// and would register a degenerate board. Polls a few times, then
    /// logs and gives up.
    fn register_when_ready(controller: Rc<RefCell<Self>>) {
        let mut tries = 0u8;
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            tries += 1;
            // `board_rect()` answers `Some` even at a 1px allocation -
            // require a real size so the registered fields cover real
            // screen area.
            let ready = controller.borrow().screen.surface.scene.width() >= 100;
            if !ready {
                return if tries < 50 {
                    glib::ControlFlow::Continue
                } else {
                    warn!("field set registration skipped: scene never realized");
                    glib::ControlFlow::Break
                };
            }
            let mut controller_ref = controller.borrow_mut();
            let Some(Source::Daemon(source)) = &controller_ref.source else {
                return glib::ControlFlow::Break;
            };
            let source = source.clone();
            let spec = controller_ref.build_field_set(&source);
            let result = controller_ref.host.as_ref().map(|host| {
                // A previous registration (stale session, re-register
                // after rotation) would collide on the set name.
                let _ = host.unregister(&source);
                host.register(&source, &spec)
            });
            match result {
                Some(Ok(())) => {
                    info!("field set registered");
                    controller_ref.update_placement();
                }
                Some(Err(err)) => warn!("field set registration failed: {err}"),
                None => warn!("field set registration skipped: no tangible host"),
            }
            glib::ControlFlow::Break
        });
    }

    /// Connects a `MockSource` for development (`--mock`).
    pub fn use_mock(controller: &Rc<RefCell<Self>>, mock: MockSource) {
        Self::start_event_pump(controller.clone(), mock.events());
        controller.borrow_mut().source = Some(Source::Mock(mock));
    }

    /// Starts the MCP server on `addr` and drains its command channel on
    /// the GTK main loop. Each request is executed against the *active*
    /// controller - which only exists while a game runs; before that the
    /// endpoint replies `{"error": "no active game"}`.
    pub fn attach_mcp(addr: std::net::SocketAddr) {
        let (tx, rx) = async_channel::unbounded::<tangibles_chess_mcp::GameRequest>();
        tangibles_chess_mcp::spawn_http(tx, addr);
        glib::MainContext::default().spawn_local(async move {
            while let Ok(request) = rx.recv().await {
                let mut reply = serde_json::json!({ "error": "no active game" });
                with_controller(|controller| {
                    reply = controller.borrow_mut().handle_mcp(&request.command);
                });
                let _ = request.reply.send(reply);
            }
        });
        info!(%addr, "MCP control endpoint enabled");
    }

    /// Executes one MCP command against the session and returns its
    /// JSON reply.
    fn handle_mcp(&mut self, command: &tangibles_chess_mcp::CommandKind) -> serde_json::Value {
        use tangibles_chess_mcp::CommandKind;
        match command {
            CommandKind::GameState => self.mcp_game_state(),
            CommandKind::LegalMoves { square } => {
                let position = self.session.position();
                let moves: Vec<String> = position
                    .legal_moves()
                    .iter()
                    .filter(|mv| match square.as_deref() {
                        Some(name) => name.parse::<Square>().ok() == mv.from(),
                        None => true,
                    })
                    .map(|mv| mv.to_uci(shakmaty::CastlingMode::Standard).to_string())
                    .collect();
                serde_json::json!({ "legal_moves": moves, "turn": self.session.turn().to_string() })
            }
            CommandKind::MakeMove { uci } => {
                let parsed = match uci.parse::<shakmaty::uci::UciMove>() {
                    Ok(mv) => mv.to_move(self.session.position()).ok(),
                    Err(_) => None,
                };
                let Some(mv) = parsed else {
                    return serde_json::json!({ "error": format!("invalid move '{uci}'") });
                };
                let request = MoveRequest {
                    from: mv.from().unwrap_or_else(|| mv.to()),
                    to: mv.to(),
                    promotion: mv.promotion(),
                };
                match self.apply_request(request) {
                    MoveOutcome::Applied { san } => {
                        serde_json::json!({ "applied": san, "fen": self.session.fen() })
                    }
                    MoveOutcome::NeedsPromotion { to } => serde_json::json!({
                        "needs_promotion": to.to_string(),
                        "hint": "append a role letter, e.g. 'e7e8q'",
                    }),
                    MoveOutcome::Illegal { reason } => serde_json::json!({ "illegal": reason }),
                }
            }
            CommandKind::Undo => match self.session.undo() {
                Some(plan) => {
                    let steps: Vec<String> = plan
                        .steps
                        .iter()
                        .map(|step| match step {
                            tangibles_chess_core::UndoStep::MoveBack { from, to } => {
                                format!("move {to} back to {from}")
                            }
                            tangibles_chess_core::UndoStep::Restore { square, piece } => {
                                format!("restore {piece:?} on {square}")
                            }
                        })
                        .collect();
                    self.sync_scene();
                    serde_json::json!({ "undone": true, "steps": steps, "fen": self.session.fen() })
                }
                None => serde_json::json!({ "undone": false, "reason": "nothing to undo" }),
            },
            CommandKind::Resign { color } => {
                let color = match color.as_str() {
                    "black" => Color::Black,
                    _ => Color::White,
                };
                self.session.resign(color);
                self.session.finish_if_terminal();
                self.sync_scene();
                serde_json::json!({ "resigned": color.to_string(), "outcome": self.session.outcome().map(|o| format!("{o:?}")) })
            }
            CommandKind::AgreeDraw => {
                self.session.agree_draw();
                self.session.finish_if_terminal();
                self.sync_scene();
                serde_json::json!({ "draw_agreed": true, "outcome": self.session.outcome().map(|o| format!("{o:?}")) })
            }
            CommandKind::Screenshot => self.mcp_screenshot(),
            CommandKind::PlacementStatus => {
                let status = self.gate.check(self.session.position(), &self.probe);
                serde_json::json!({
                    "placed": status.placed(),
                    "total": status.total,
                    "missing": status.missing.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                    "unexpected": status.unexpected.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                    "consistent": status.is_consistent(),
                })
            }
        }
    }

    /// PNG screenshot of the window for the MCP `screenshot` tool.
    fn mcp_screenshot(&self) -> serde_json::Value {
        let Some(texture) = self.screenshot_texture() else {
            return serde_json::json!({ "error": "window is not realized" });
        };
        let bytes = texture.save_to_png_bytes();
        serde_json::json!({
            "png_base64": gtk4::glib::base64_encode(bytes.as_ref()).as_str(),
            "width": texture.width(),
            "height": texture.height(),
        })
    }

    /// Renders the whole window into a texture (snapshot path shared by
    /// the MCP tool and the save-to-file action).
    fn screenshot_texture(&self) -> Option<gtk4::gdk::Texture> {
        let window = self
            .screen
            .widget
            .ancestor(adw::ApplicationWindow::static_type())?
            .downcast::<adw::ApplicationWindow>()
            .ok()?;
        let surface = window.surface()?;
        let snapshot = gtk4::Snapshot::new();
        gtk4::WidgetPaintable::new(Some(&window)).snapshot(&snapshot, f64::from(window.width()), f64::from(window.height()));
        let node = snapshot.to_node()?;
        let renderer = gtk4::gsk::Renderer::for_surface(&surface)?;
        let viewport = gtk4::graphene::Rect::new(0.0, 0.0, window.width() as f32, window.height() as f32);
        let texture = renderer.render_texture(&node, Some(&viewport));
        // A renderer may only be disposed while unrealized - dropping it
        // while it is still realized on the surface asserts in GSK.
        renderer.unrealize();
        Some(texture)
    }

    /// Saves a window screenshot into the app data directory (win.screenshot).
    pub fn screenshot(&self) {
        let Some(texture) = self.screenshot_texture() else {
            return;
        };
        let dir = glib::user_data_dir().join("tangibles-chess").join("screenshots");
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs())
            .unwrap_or(0);
        let path = dir.join(format!("screenshot-{stamp}.png"));
        if std::fs::write(&path, texture.save_to_png_bytes().as_ref()).is_ok() {
            tracing::info!("screenshot saved to {}", path.display());
            self.screen.show_message(&self.l10n.msg("game-screenshot-saved"));
        }
    }

    /// Game-state snapshot for the MCP `game_state` tool.
    fn mcp_game_state(&self) -> serde_json::Value {
        let config = self.session.config();
        serde_json::json!({
            "phase": format!("{:?}", self.session.phase()),
            "turn": self.session.turn().to_string(),
            "fen": self.session.fen(),
            "san_history": self.session.san_history(),
            "outcome": self.session.outcome().map(|o| format!("{o:?}")),
            "tangible_mode": format!("{:?}", config.tangible_mode),
            "claimable_draws": self.session.claimable_draws().iter().map(|d| format!("{d:?}")).collect::<Vec<_>>(),
        })
    }

    /// Drains the field-event channel into the controller on the main
    /// loop.
    fn start_event_pump(controller: Rc<RefCell<Self>>, events: async_channel::Receiver<FieldEvent>) {
        glib::MainContext::default().spawn_local(async move {
            while let Ok(event) = events.recv().await {
                controller.borrow_mut().on_field_event(event);
            }
        });
    }

    /// Computes the board rect in screen millimeters and builds the
    /// field set registration.
    fn build_field_set(&mut self, source: &GraphQlSource) -> tangibles_chess_tangibles::FieldSet {
        // Window size in screen mm comes from the device calibration;
        // the board rect in mm is the rendered board rect scaled by
        // mm-per-pixel of the panel. The px denominator is the monitor
        // geometry - the app runs fullscreen on the desk surface - not
        // the widget size, which can lag behind layout or diverge.
        let Some(host) = &self.host else {
            return FieldSetBuilder::new((0.0, 0.0, 100.0), (100.0, 100.0)).build();
        };
        let info = host.device_info(source).unwrap_or_default();
        let (screen_w_mm, screen_h_mm) = info.screen_mm().unwrap_or((1000.0, 600.0));
        let (panel_w, panel_h) = self
            .panel_px()
            .unwrap_or_else(|| (self.screen.widget.width().max(1) as f64, self.screen.widget.height().max(1) as f64));
        let mm_per_px_x = screen_w_mm / panel_w;
        let mm_per_px_y = screen_h_mm / panel_h;
        let mm_per_px = (mm_per_px_x + mm_per_px_y) / 2.0;
        let widget_size = (self.screen.widget.width(), self.screen.widget.height());
        let scene_rect = self.screen.surface.scene.board_rect();
        info!(
            "field set build: panel={panel_w:.0}x{panel_h:.0}px -> {:.3}mm/px (x={mm_per_px_x:.3} y={mm_per_px_y:.3}), \
             widget={}x{}, scene_board={scene_rect:?}",
            widget_size.0, widget_size.1, mm_per_px
        );
        if let Some((bx, by, bs)) = self.screen.surface.scene.board_rect_mm(mm_per_px) {
            self.board_mm = (bx, by, bs);
        } else {
            // Scene not realized yet: approximate with a centered square.
            let side = panel_w.min(panel_h) * 0.88 * mm_per_px;
            self.board_mm = ((screen_w_mm - side) / 2.0, (screen_h_mm - side) / 2.0, side);
        }
        info!(
            "field set board_mm: ({:.1}, {:.1}, {:.1}) on {:.1}x{:.1}mm surface",
            self.board_mm.0, self.board_mm.1, self.board_mm.2, screen_w_mm, screen_h_mm
        );
        FieldSetBuilder::new(self.board_mm, (screen_w_mm, screen_h_mm)).build()
    }

    /// The monitor's pixel size that hosts our surface - the panel the
    /// touch coordinates are calibrated against.
    fn panel_px(&self) -> Option<(f64, f64)> {
        let surface = self.screen.widget.native()?.surface()?;
        let monitor = surface.display().monitor_at_surface(&surface)?;
        let geom = monitor.geometry();
        Some((geom.width() as f64, geom.height() as f64))
    }

    /// One field event from the daemon.
    fn on_field_event(&mut self, event: FieldEvent) {
        self.probe.on_event(&event);
        match self.session.phase() {
            GamePhase::AwaitingPlacement => self.update_placement(),
            GamePhase::InProgress => self.on_game_field_event(event),
            _ => {}
        }
    }

    /// Placement gate evaluation during the start handshake.
    fn update_placement(&mut self) {
        let status = self.gate.check(self.session.position(), &self.probe);
        self.screen.board_check.set_status(&status, &self.l10n);
        self.screen.surface.scene.update(|state| {
            state.missing = status.missing.clone();
            state.foreign = status.unexpected.clone();
            state.ceremony = status.correct.iter().copied().collect();
        });
        if status.is_consistent() {
            self.session.confirm_placement();
            info!("placement gate satisfied - game starts");
            self.sync_scene();
        }
    }

    /// Field events during the game: move inference and zones.
    fn on_game_field_event(&mut self, event: FieldEvent) {
        // While replaying, the physical board only feeds the
        // discrepancy display - no move inference.
        if self.replay_index.is_some() {
            self.update_discrepancy();
            return;
        }
        // Visual feedback: the lifted square glows while a piece is in
        // the air ("light under the figure").
        if event.kind == FieldEventKind::FieldLifted
            && let Some(name) = &event.field
            && let Some(index) = BoardGeometry::field_index(name)
            && let Some(square) = self.geometry.field_to_square(index)
        {
            let own = self.session.position().board().piece_at(square);
            let lifted = own.filter(|p| p.color == self.session.turn()).map(|_| square);
            self.screen.surface.scene.update(|state| {
                state.lifted = lifted;
                if let Some(lifted) = lifted {
                    state.legal_targets = self.legal_targets_cached(lifted);
                }
            });
        }
        // Graceful return after a rejected intent: lifting the misplaced
        // marker and placing it back on its origin restores the board
        // quietly instead of producing lift/place anomalies.
        if let Some((from, to)) = self.rejected_move
            && let Some(name) = event.field.as_deref()
            && let Some(index) = BoardGeometry::field_index(name)
            && let Some(square) = self.geometry.field_to_square(index)
        {
            match event.kind {
                FieldEventKind::FieldLifted if square == to => return,
                FieldEventKind::FieldPlaced if square == from || square == to => {
                    // Back on its origin resolves the rejection; dropped
                    // on the same wrong square keeps it pending.
                    self.rejected_move = (square == to).then_some((from, to));
                    return;
                }
                FieldEventKind::FieldPlaced => self.rejected_move = None,
                _ => {}
            }
        }
        let position = self.session.position().clone();
        for output in self.inference.on_event(&event, &position) {
            match output {
                InferenceOutput::MoveIntent { from, to } => {
                    self.screen.surface.scene.update(|s| s.lifted = None);
                    self.apply_request(MoveRequest::new(from, to));
                }
                InferenceOutput::Cancelled => {
                    self.screen.surface.scene.update(|s| {
                        s.lifted = None;
                        s.legal_targets.clear();
                    });
                }
                InferenceOutput::Zone(ZoneAction::ConfirmMove { .. }) => {}
                InferenceOutput::Zone(ZoneAction::TrayCount { edge, count }) => {
                    self.update_tray(edge, count);
                }
                InferenceOutput::Anomaly { detail } => {
                    warn!("board anomaly: {detail}");
                    self.screen.show_message_for(self.session.turn(), &self.l10n.msg("anomaly-detail"));
                }
                InferenceOutput::Lagged => self.resync_occupancy(),
            }
        }
        self.update_discrepancy();
    }

    /// Physical-vs-logical board mismatch markers (`TangiblesRendered`).
    ///
    /// In the rendered mode the scene draws both layers, so squares
    /// where the physical occupancy diverges from the position get a
    /// persistent ring until the marker is corrected.
    fn update_discrepancy(&self) {
        let discrepant = if self.session.config().tangible_mode == TangibleMode::TangiblesRendered {
            Square::ALL
                .into_iter()
                .filter(|square| {
                    let logical = self.session.position().board().piece_at(*square).is_some();
                    let physical = self.probe.occupied().contains(&self.geometry.square_to_field(*square));
                    logical != physical
                })
                .collect()
        } else {
            Vec::new()
        };
        self.screen.surface.scene.update(|state| state.discrepant = discrepant);
    }

    /// Releases this app's daemon registrations (the chess field set) -
    /// called on shutdown so the daemon drops the occupancy zones
    /// instead of keeping a stale `chess` set alive.
    pub fn release_registrations(&mut self) {
        let (Some(host), Some(Source::Daemon(source))) = (&self.host, &self.source) else {
            return;
        };
        match host.unregister(source) {
            Ok(()) => info!("field set unregistered"),
            Err(err) => warn!("unregister failed: {err}"),
        }
    }

    /// Re-queries touch points after a lagged subscription.
    fn resync_occupancy(&mut self) {
        let touches = match (&self.host, &self.source) {
            (Some(host), Some(Source::Daemon(source))) => match host.touch_points(source) {
                Ok(touches) => touches,
                Err(err) => {
                    warn!("touch_points resync failed: {err}");
                    return;
                }
            },
            (_, Some(Source::Mock(mock))) => {
                let context = glib::MainContext::default();
                context.block_on(mock.touch_points()).unwrap_or_default()
            }
            _ => return,
        };
        self.probe.refresh(&touches, self.board_mm);
        if self.session.phase() == GamePhase::AwaitingPlacement {
            self.update_placement();
        }
    }

    /// Updates the captured-material label for a tray zone.
    fn update_tray(&self, edge: TableEdge, count: u32) {
        let panel = if edge == self.geometry.white_edge() {
            &self.screen.surface.white_panel
        } else {
            &self.screen.surface.black_panel
        };
        panel.set_captured(&count.to_string());
    }

    /// Legal targets of a square, cached helper for the scene.
    fn legal_targets_cached(&self, square: Square) -> Vec<Square> {
        self.session.legal_targets(square)
    }

    /// Touch tap on a scene square (`NoTangibles` mode only).
    pub fn on_square(&mut self, square: Square) {
        if self.session.config().tangible_mode != TangibleMode::NoTangibles {
            return;
        }
        if self.session.phase() != GamePhase::InProgress {
            return;
        }
        if let Some(from) = self.selected {
            if square == from {
                return;
            }
            if self.session.legal_targets(from).contains(&square) {
                self.apply_request(MoveRequest::new(from, square));
                return;
            }
            let own = self
                .session
                .position()
                .board()
                .piece_at(square)
                .is_some_and(|piece| piece.color == self.session.turn());
            if !own {
                // Tapped an invalid destination for the selection -
                // flash the rejected square and keep the selection.
                self.flash_illegal(MoveRequest::new(from, square));
                return;
            }
            // Otherwise fall through and reselect the tapped own piece.
        }
        // Select own piece, otherwise clear.
        let own = self
            .session
            .position()
            .board()
            .piece_at(square)
            .is_some_and(|piece| piece.color == self.session.turn());
        self.selected = own.then_some(square);
        let targets = self.selected.map(|s| self.session.legal_targets(s)).unwrap_or_default();
        self.screen.surface.scene.update(|state| {
            state.selected = self.selected;
            state.legal_targets = targets;
        });
    }

    /// Validates and applies a move request, dispatching the outcome to
    /// the UI. Returns the session's outcome for remote callers.
    pub fn apply_request(&mut self, request: MoveRequest) -> MoveOutcome {
        let outcome = self.session.request_move(request);
        match &outcome {
            MoveOutcome::Applied { san } => {
                debug!("move applied: {san}");
                self.selected = None;
                self.last_move = Some((request.from, request.to));
                if self.session.position().is_check() {
                    self.sound.play("check");
                } else if san.contains('x') {
                    self.sound.play("capture");
                } else {
                    self.sound.play("move");
                }
                self.sync_scene();
                self.autosave();
                if self.session.phase() == GamePhase::Finished {
                    self.sound.play("end");
                }
            }
            MoveOutcome::NeedsPromotion { to } => {
                self.pending_promotion = Some(request);
                self.show_promotion_picker(*to);
            }
            MoveOutcome::Illegal { reason } => {
                warn!("illegal move rejected: {reason}");
                self.rejected_move = Some((request.from, request.to));
                self.screen.show_message_for(self.session.turn(), reason);
                self.flash_illegal(request);
            }
        }
        outcome
    }

    /// Briefly flashes the rejected move's target square red - plus,
    /// when the naive move would have left the own king in check, the
    /// threatened king and its attackers.
    fn flash_illegal(&mut self, request: MoveRequest) {
        self.illegal_seq += 1;
        let seq = self.illegal_seq;
        let mut squares = vec![(request.to, FlashRole::Target)];
        if let Some(projection) = self.session.projected_check(&request) {
            squares.push((projection.king, FlashRole::King));
            squares.extend(projection.attackers.iter().map(|attacker| (*attacker, FlashRole::Attacker)));
        }
        self.screen.surface.scene.update(move |state| {
            state.illegal = Some(IllegalFlash { squares, generation: seq });
        });
        let scene = self.screen.surface.scene.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(ILLEGAL_FLASH_MS), move || {
            scene.update(|state| {
                if state.illegal.as_ref().is_some_and(|flash| flash.generation == seq) {
                    state.illegal = None;
                }
            });
        });
    }

    /// Shows the promotion picker facing the moving player.
    fn show_promotion_picker(&self, _to: Square) {
        let mover = self.session.turn();
        let facing_north = match mover {
            Color::White => self.geometry.white_edge() == TableEdge::North,
            Color::Black => self.geometry.black_edge() == TableEdge::North,
        };
        let picker = PromotionPicker::new(
            mover,
            facing_north,
            move |role| {
                with_controller(|controller| {
                    let mut controller = controller.borrow_mut();
                    let Some(request) = controller.pending_promotion.take() else {
                        return;
                    };
                    if let Some(widget) = controller.promotion_widget.borrow_mut().take() {
                        controller.screen.pop_overlay(&widget);
                    }
                    controller.apply_request(MoveRequest {
                        from: request.from,
                        to: request.to,
                        promotion: Some(role),
                    });
                });
            },
            &self.l10n,
        );
        self.screen.push_overlay(&picker.widget);
        *self.promotion_widget.borrow_mut() = Some(picker.widget.clone());
    }

    /// Undo: logical revert + (M2) physical guide. In replay mode the
    /// button steps the cursor back instead.
    pub fn undo(&mut self) {
        if self.replay_index.is_some() {
            self.replay_step(-1);
            return;
        }
        if let Some(plan) = self.session.undo() {
            for step in &plan.steps {
                debug!("undo step: {step:?}");
            }
            self.last_move = None;
            self.sync_scene();
            self.autosave();
        }
    }

    /// Redo: re-applies a move undone by `undo`. In replay mode the
    /// button steps the cursor forward instead.
    pub fn redo(&mut self) {
        if self.replay_index.is_some() {
            self.replay_step(1);
            return;
        }
        if let Some(plan) = self.session.redo() {
            for step in &plan.steps {
                debug!("redo step: {step:?}");
            }
            self.last_move = None;
            self.sync_scene();
            self.autosave();
        }
    }

    /// Enters replay mode on a finished game: dismisses the result
    /// overlay and lets the undo/redo buttons step through the recorded
    /// plies without mutating the session.
    pub fn enter_replay(&mut self) {
        if self.session.phase() != GamePhase::Finished || self.session.ply_count() == 0 {
            return;
        }
        if let Some(widget) = self.result_widget.take() {
            self.screen.pop_overlay(&widget);
        }
        self.replay_index = Some(self.session.ply_count());
        self.sync_scene();
    }

    /// Jumps the replay cursor straight to the position after
    /// `move_index`'s move (score-sheet click, 0-based ply index).
    /// Enters replay mode implicitly on a finished game.
    pub fn replay_goto(&mut self, move_index: usize) {
        if self.session.phase() != GamePhase::Finished {
            return;
        }
        if let Some(widget) = self.result_widget.take() {
            self.screen.pop_overlay(&widget);
        }
        self.replay_index = Some((move_index + 1).min(self.session.ply_count()));
        self.sync_scene();
    }

    /// Moves the replay cursor by `delta` plies (clamped).
    fn replay_step(&mut self, delta: i64) {
        let Some(index) = self.replay_index else {
            return;
        };
        let next = index.saturating_add_signed(delta as isize).min(self.session.ply_count());
        self.replay_index = Some(next);
        self.sync_scene();
    }

    /// Leaves replay mode and returns to the result overlay.
    pub fn exit_replay(&mut self) {
        if self.replay_index.is_none() {
            return;
        }
        self.replay_index = None;
        self.result_shown = false;
        self.sync_scene();
    }

    /// Applies the setup options that are safe to change mid-game
    /// (board theme, sound, undo) to the running session. Structural
    /// options (input mode, commit mode, handshake, orientation,
    /// opponents) only take effect on the next game - they live in
    /// daemon registrations, field mapping and session setup.
    pub fn apply_live_config(&mut self, config: &GameConfig) {
        {
            let session_config = self.session.config_mut();
            session_config.theme = config.theme;
            session_config.sound_enabled = config.sound_enabled;
            session_config.allow_undo = config.allow_undo;
        }
        self.settings.sound_enabled = config.sound_enabled;
        self.settings.save();
        self.sound.set_enabled(config.sound_enabled);
        self.screen.surface.scene.update(|state| state.theme = config.theme);
        self.sync_scene();
        self.autosave();
    }

    /// Rotates the board one quarter turn clockwise.
    pub fn rotate_cw(&mut self) {
        self.rotate(true);
    }

    /// Applies a rotation step, updating the field mapping. Locked while
    /// a tangible game is in progress (resting markers would remap).
    fn rotate(&mut self, clockwise: bool) {
        let locked = self.session.config().tangible_mode.uses_tangibles() && self.session.phase() == GamePhase::InProgress;
        if locked {
            self.screen.show_message(&self.l10n.msg("game-rotation-locked"));
            return;
        }
        self.screen.rotate_step(clockwise);
        // The physical field mapping follows the rotated board - re-register
        // once the rotation animation has settled.
        glib::timeout_add_local_once(std::time::Duration::from_millis(600), || {
            with_controller(|controller| Self::register_when_ready(controller.clone()));
        });
        self.geometry = BoardGeometry::new(self.screen.orientation(), self.geometry.white_edge());
        self.inference = MoveInference::new(self.geometry, self.session.config().commit_mode);
        self.gate = PlacementGate::new(self.geometry);
    }

    /// Resign for `color`.
    pub fn resign(&mut self, color: Color) {
        if self.replay_index.is_some() {
            return;
        }
        self.session.resign(color);
        self.sync_scene();
        self.show_result();
    }

    /// Agree a draw.
    pub fn draw(&mut self) {
        if self.replay_index.is_some() {
            return;
        }
        self.session.agree_draw();
        self.sync_scene();
        self.show_result();
    }

    /// Syncs scene + panels + sheets with the session state.
    pub fn sync_scene(&mut self) {
        // In replay mode the scene shows the cursor's historical
        // position instead of the live one.
        let position = self
            .replay_index
            .and_then(|index| self.session.position_at(index))
            .unwrap_or_else(|| self.session.position())
            .clone();
        let check = position.is_check().then(|| position.board().king_of(position.turn()));
        let last_move = (self.replay_index.is_none() && self.session.san_history().len().gt(&0))
            .then(|| self.last_move_squares())
            .flatten();
        self.screen.surface.scene.update(|state| {
            state.position = Some(position.clone());
            state.check = check.flatten();
            state.last_move = last_move;
            state.selected = self.selected;
            state.legal_targets = self.selected.map(|s| self.session.legal_targets(s)).unwrap_or_default();
        });
        let sans = self.session.san_history();
        self.screen.surface.white_sheet.set_moves(sans);
        self.screen.surface.black_sheet.set_moves(sans);
        // In replay mode the cursor's producing move is highlighted:
        // position after ply `n` comes from move `n - 1`.
        let active_ply = self.replay_index.and_then(|index| index.checked_sub(1));
        self.screen.surface.white_sheet.set_active_ply(active_ply);
        self.screen.surface.black_sheet.set_active_ply(active_ply);
        let (turn_text, active) = match position.turn() {
            Color::White => (self.l10n.msg("game-white-to-move"), true),
            Color::Black => (self.l10n.msg("game-black-to-move"), false),
        };
        if self.session.phase() == GamePhase::Finished {
            if let Some(index) = self.replay_index {
                let label = format!("{} {}/{}", self.l10n.msg("game-replay"), index, self.session.ply_count());
                self.screen.set_instruction(Color::White, &label);
                self.screen.set_instruction(Color::Black, &label);
            } else {
                self.show_result();
            }
        } else {
            self.screen.set_instruction(Color::White, if active { &turn_text } else { "" });
            self.screen.set_instruction(Color::Black, if !active { &turn_text } else { "" });
        }
        self.screen.surface.white_panel.set_active(active);
        self.screen.surface.black_panel.set_active(!active);
        let (can_undo, can_redo) = match self.replay_index {
            // In replay the buttons walk the cursor instead of the undo
            // stack.
            Some(index) => (index > 0, index < self.session.ply_count()),
            None => (self.session.can_undo(), self.session.can_redo()),
        };
        self.screen.surface.white_panel.set_undo_enabled(can_undo);
        self.screen.surface.white_panel.set_redo_enabled(can_redo);
        self.screen.surface.black_panel.set_undo_enabled(can_undo);
        self.screen.surface.black_panel.set_redo_enabled(can_redo);
        let play_enabled = self.replay_index.is_none() && self.session.phase() == GamePhase::InProgress;
        self.screen.surface.white_panel.set_play_enabled(play_enabled);
        self.screen.surface.black_panel.set_play_enabled(play_enabled);
        self.screen.surface.white_panel.set_replay_mode(self.replay_index.is_some());
        self.screen.surface.black_panel.set_replay_mode(self.replay_index.is_some());
    }

    /// Squares of the last applied move (tracked in `apply_request`).
    fn last_move_squares(&self) -> Option<(Square, Square)> {
        self.last_move
    }

    /// Presents the game result once: a modal, dual-facing overlay
    /// (outcome icon + confirm button leading to the setup dialog).
    fn show_result(&mut self) {
        if self.result_shown {
            return;
        }
        let (text, icon) = match self.session.outcome() {
            Some(KnownOutcome::Decisive { winner }) => match winner {
                Color::White => (self.l10n.msg("game-white-wins"), icon_names::TCH_TROPHY_SYMBOLIC),
                Color::Black => (self.l10n.msg("game-black-wins"), icon_names::TCH_TROPHY_SYMBOLIC),
            },
            Some(KnownOutcome::Draw) => {
                // The overlay states the draw's rule (stalemate,
                // repetition, material, agreement, ...) when known.
                let reason = self.session.draw_reason().map(|reason| match reason {
                    DrawReason::Stalemate => self.l10n.msg("game-draw-stalemate"),
                    DrawReason::InsufficientMaterial => self.l10n.msg("game-draw-insufficient"),
                    DrawReason::ThreefoldRepetition => self.l10n.msg("game-draw-threefold"),
                    DrawReason::FivefoldRepetition => self.l10n.msg("game-draw-fivefold"),
                    DrawReason::FiftyMoves => self.l10n.msg("game-draw-fifty-moves"),
                    DrawReason::SeventyFiveMoves => self.l10n.msg("game-draw-seventy-five-moves"),
                    DrawReason::Agreement => self.l10n.msg("game-draw-agreement"),
                });
                let mut text = self.l10n.msg("game-drawn");
                if let Some(reason) = reason {
                    text = format!("{text} — {reason}");
                }
                (text, icon_names::TCH_HANDSHAKE_SYMBOLIC)
            }
            None => return,
        };
        self.result_shown = true;
        let confirm = self.l10n.msg("game-new");
        let replay = self.l10n.msg("game-replay");
        let quit = self.l10n.msg("setup-quit");
        let result = GameResult::new(&text, icon, &confirm, Some(&replay), &quit);
        self.screen.push_overlay(&result.widget);
        self.result_widget = Some(result.widget);
    }

    /// Writes the autosave file.
    fn autosave(&self) {
        let dir = glib::user_data_dir().join("tangibles-chess");
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(text) = self.session.to_saved().to_toml() {
            let _ = std::fs::write(dir.join("autosave.toml"), text);
        }
    }
}
