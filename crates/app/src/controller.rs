use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::WidgetExt;
use shakmaty::Color;
use shakmaty::KnownOutcome;
use shakmaty::Position;
use shakmaty::Square;
use tangibles_chess_core::GamePhase;
use tangibles_chess_core::GameSession;
use tangibles_chess_core::MoveOutcome;
use tangibles_chess_core::MoveRequest;
use tangibles_chess_core::TangibleMode;
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
}

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
                    let spec = controller.borrow_mut().build_field_set(&source);
                    if let Err(err) = controller.borrow().register(spec, &source) {
                        warn!("field set registration failed: {err}");
                        return;
                    }
                    Self::start_event_pump(controller.clone(), source.events());
                    controller.borrow_mut().source = Some(Source::Daemon(Rc::new(source)));
                    controller.borrow_mut().sync_scene();
                    info!("tangible daemon connected and field set registered");
                }
                None => {
                    controller.borrow().screen.daemon_banner.set_revealed(true);
                    warn!("daemon connection failed");
                }
            }
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
        // mm-per-pixel of the panel.
        let Some(host) = &self.host else {
            return FieldSetBuilder::new((0.0, 0.0, 100.0), (100.0, 100.0)).build();
        };
        let info = host.device_info(source).unwrap_or_default();
        let (screen_w_mm, screen_h_mm) = info.screen_mm().unwrap_or((1000.0, 600.0));
        let widget_w = self.screen.widget.width().max(1) as f64;
        let widget_h = self.screen.widget.height().max(1) as f64;
        let mm_per_px_x = screen_w_mm / widget_w;
        let mm_per_px_y = screen_h_mm / widget_h;
        let mm_per_px = (mm_per_px_x + mm_per_px_y) / 2.0;
        if let Some((bx, by, bs)) = self.screen.surface.scene.board_rect_mm(mm_per_px) {
            self.board_mm = (bx, by, bs);
        } else {
            // Scene not realized yet: approximate with a centered square.
            let side = widget_w.min(widget_h) * 0.88 * mm_per_px;
            self.board_mm = ((screen_w_mm - side) / 2.0, (screen_h_mm - side) / 2.0, side);
        }
        FieldSetBuilder::new(self.board_mm, (screen_w_mm, screen_h_mm)).build()
    }

    /// Registers the field set through the host runtime.
    fn register(&self, spec: tangibles_chess_tangibles::FieldSet, source: &GraphQlSource) -> Result<(), tangibles_chess_tangibles::DaemonError> {
        self.host
            .as_ref()
            .ok_or_else(|| tangibles_chess_tangibles::DaemonError::Unavailable("no host".to_string()))?
            .register(source, &spec)
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
                    self.screen.toast(&self.l10n.msg("anomaly-detail"));
                }
                InferenceOutput::Lagged => self.resync_occupancy(),
            }
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
        if let Some(from) = self.selected
            && (self.session.legal_targets(from).contains(&square) || square == from)
        {
            if square != from {
                self.apply_request(MoveRequest::new(from, square));
            }
            return;
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
                self.screen.toast(reason);
            }
        }
        outcome
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

    /// Undo: logical revert + (M2) physical guide.
    pub fn undo(&mut self) {
        if let Some(plan) = self.session.undo() {
            for step in &plan.steps {
                debug!("undo step: {step:?}");
            }
            self.last_move = None;
            self.sync_scene();
            self.autosave();
        }
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
            self.screen.toast("rotation locked during play");
            return;
        }
        self.screen.rotate_step(clockwise);
        self.geometry = BoardGeometry::new(self.screen.orientation(), self.geometry.white_edge());
        self.inference = MoveInference::new(self.geometry, self.session.config().commit_mode);
        self.gate = PlacementGate::new(self.geometry);
    }

    /// Resign for `color`.
    pub fn resign(&mut self, color: Color) {
        self.session.resign(color);
        self.sync_scene();
        self.show_result();
    }

    /// Agree a draw.
    pub fn draw(&mut self) {
        self.session.agree_draw();
        self.sync_scene();
        self.show_result();
    }

    /// Syncs scene + panels + sheets with the session state.
    pub fn sync_scene(&mut self) {
        let position = self.session.position().clone();
        let check = position.is_check().then(|| position.board().king_of(position.turn()));
        let last_move = self.session.san_history().len().gt(&0).then(|| self.last_move_squares()).flatten();
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
        let (turn_text, active) = match self.session.turn() {
            Color::White => (self.l10n.msg("game-white-to-move"), true),
            Color::Black => (self.l10n.msg("game-black-to-move"), false),
        };
        if self.session.phase() == GamePhase::Finished {
            self.show_result();
        } else {
            self.screen.surface.white_panel.set_status(if active { &turn_text } else { "" });
            self.screen.surface.black_panel.set_status(if !active { &turn_text } else { "" });
        }
        self.screen.surface.white_panel.set_active(active);
        self.screen.surface.black_panel.set_active(!active);
    }

    /// Squares of the last applied move (tracked in `apply_request`).
    fn last_move_squares(&self) -> Option<(Square, Square)> {
        self.last_move
    }

    /// Presents the game result.
    fn show_result(&self) {
        let text = match self.session.outcome() {
            Some(KnownOutcome::Decisive { winner }) => match winner {
                Color::White => self.l10n.msg("game-white-wins"),
                Color::Black => self.l10n.msg("game-black-wins"),
            },
            Some(KnownOutcome::Draw) => self.l10n.msg("game-drawn"),
            None => return,
        };
        self.screen.toast(&text);
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
