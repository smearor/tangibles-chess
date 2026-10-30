//! Tangible backend lifecycle and field-event handling.

use std::cell::RefCell;
use std::rc::Rc;

use gdk4::prelude::DisplayExt;
use gdk4::prelude::MonitorExt;
use gdk4::prelude::SurfaceExt;
use gtk4::prelude::NativeExt;
use gtk4::prelude::WidgetExt;
use shakmaty::Position;
use shakmaty::Square;
use tangibles_chess_config::TangibleMode;
use tangibles_chess_material::Side;
use tangibles_chess_moves::GamePhase;
use tangibles_chess_tangibles::BoardRectMm;
use tangibles_chess_tangibles::CapSide;
use tangibles_chess_tangibles::FieldEvent;
use tangibles_chess_tangibles::FieldEventKind;
use tangibles_chess_tangibles::FieldName;
use tangibles_chess_tangibles::FieldSetBuilder;
use tangibles_chess_tangibles::GraphQlSource;
use tangibles_chess_tangibles::InferenceOutput;
use tangibles_chess_tangibles::Millimeters;
use tangibles_chess_tangibles::MockSource;
use tangibles_chess_tangibles::PlacementStatus;
use tangibles_chess_tangibles::SurfaceMm;
use tangibles_chess_tangibles::TableEdge;
use tangibles_chess_tangibles::TangibleSource;
use tangibles_chess_tangibles::TouchPoint;
use tangibles_chess_tangibles::ZoneAction;
use tracing::info;
use tracing::warn;

use super::game_controller::GameController;
use super::source::Source;
use crate::playable::Playable;
use crate::tangible_host::TangibleHost;

impl GameController {
    /// Connects to the daemon, registers the chess field set and starts
    /// the event pump (all off the main thread; results hop back via
    /// `idle_add`).
    pub(super) fn activate_tangibles(controller: Rc<RefCell<Self>>) {
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
    pub(crate) fn register_when_ready(controller: Rc<RefCell<Self>>) {
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
        self.window_mm = SurfaceMm::from((screen_w_mm, screen_h_mm));
        if let Some((bx, by, bs)) = self.screen.surface.scene.board_rect_mm(mm_per_px) {
            self.board_mm = BoardRectMm::from((bx, by, bs));
        } else {
            // Scene not realized yet: approximate with a centered square.
            let side = panel_w.min(panel_h) * 0.88 * mm_per_px;
            self.board_mm = BoardRectMm::from(((screen_w_mm - side) / 2.0, (screen_h_mm - side) / 2.0, side));
        }
        info!(
            "field set board_mm: ({:.1}, {:.1}, {:.1}) on {:.1}x{:.1}mm surface",
            self.board_mm.x, self.board_mm.y, self.board_mm.side, screen_w_mm, screen_h_mm
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
    pub(super) fn update_placement(&mut self) {
        if self.session.phase() != GamePhase::AwaitingPlacement {
            return;
        }
        let status = self.gate.check(self.session.position(), &self.probe);
        self.screen.surface.scene.update(|state| {
            state.missing = status.missing.clone();
            state.foreign = status.unexpected.clone();
            state.ceremony = status.correct.iter().copied().collect();
        });
        // Per-side status lines: each player only sees what concerns
        // them - own placed count, own missing pieces, shared extras.
        let position = self.session.position().clone();
        for side in Side::ALL {
            let own = |square: Square| position.board().piece_at(square).is_some_and(|piece| Side::from(piece.color) == side);
            let placed = status.correct.iter().copied().filter(|square| own(*square)).count();
            let missing: Vec<Square> = status.missing.iter().copied().filter(|square| own(*square)).collect();
            let text = self.placement_line(placed, &missing, &status);
            self.screen.set_instruction(side, &text);
        }
        if status.is_consistent() {
            self.session.confirm_placement();
            info!("placement gate satisfied - game starts");
            self.sync_scene();
        }
    }

    /// One side's handshake status line: `placed/total` plus the first
    /// own missing hint and the first extra-piece hint - or `Ready`
    /// once this side has nothing left to fix.
    fn placement_line(&self, placed: usize, own_missing: &[Square], status: &PlacementStatus) -> String {
        if own_missing.is_empty() && status.unexpected.is_empty() {
            return self.l10n.msg("check-ready").to_string();
        }
        let mut parts = vec![format!("{placed}/{}", placed + own_missing.len())];
        for (key, square) in [("check-missing", own_missing.first()), ("check-unexpected", status.unexpected.first())] {
            if let Some(square) = square {
                let mut args = fluent::FluentArgs::new();
                args.set("square", square.to_string());
                parts.push(self.l10n.msg_args(key, &args).to_string());
            }
        }
        parts.join(" - ")
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
            && let Some(FieldName::Board(index)) = event.field.as_ref()
        {
            let square = self.geometry.field_to_square(*index);
            let own = self.session.position().board().piece_at(square);
            let lifted = own.filter(|p| Side::from(p.color) == self.session.turn()).map(|_| square);
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
        if let Some(rejected) = self.rejected_move
            && let Some(FieldName::Board(index)) = event.field.as_ref()
        {
            let square = self.geometry.field_to_square(*index);
            match event.kind {
                FieldEventKind::FieldLifted if square == rejected.to => return,
                FieldEventKind::FieldPlaced if square == rejected.from || square == rejected.to => {
                    // Back on its origin resolves the rejection; dropped
                    // on the same wrong square keeps it pending.
                    self.rejected_move = (square == rejected.to).then_some(rejected);
                    return;
                }
                FieldEventKind::FieldPlaced => self.rejected_move = None,
                _ => {}
            }
        }
        let position = self.session.position().clone();
        for output in self.inference.on_event(&event, &position) {
            match output {
                InferenceOutput::MoveIntent(request) => {
                    self.screen.surface.scene.update(|s| s.lifted = None);
                    self.apply_request(request);
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
                InferenceOutput::Zone(ZoneAction::CapCell { side, row, occupied }) => {
                    match side {
                        CapSide::West => self.parked_west.set_occupied(row as usize, occupied),
                        CapSide::East => self.parked_east.set_occupied(row as usize, occupied),
                    }
                    self.sync_scene();
                }
                InferenceOutput::Anomaly(reason) => {
                    warn!("board anomaly: {reason}");
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
                    let physical = self.geometry.square_to_field(*square).is_some_and(|f| self.probe.occupied().contains(&f));
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
        self.resync_parked(&touches);
        if self.session.phase() == GamePhase::AwaitingPlacement {
            self.update_placement();
        }
    }

    /// Rebuilds parking-cell occupancy from raw touches after a lagged
    /// subscription - the probe only maps board fields, so the cap
    /// strips get their own point-in-rect pass with the same bounds
    /// the field-set builder registers.
    fn resync_parked(&mut self, touches: &[TouchPoint]) {
        self.parked_west.clear();
        self.parked_east.clear();
        let board = self.board_mm;
        let cell = board.cell();
        if cell <= Millimeters::default() {
            return;
        }
        for touch in touches {
            let row = (touch.pos.y - board.y.as_f64()) / cell.as_f64();
            if !(0.0..8.0).contains(&row) {
                continue;
            }
            let row = row as usize;
            let px = Millimeters::new(touch.pos.x);
            if board.x >= cell && px >= board.x - cell && px < board.x {
                self.parked_west.set_occupied(row, true);
            }
            if board.x_max() + cell <= self.window_mm.width && px >= board.x_max() && px < board.x_max() + cell {
                self.parked_east.set_occupied(row, true);
            }
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
}
