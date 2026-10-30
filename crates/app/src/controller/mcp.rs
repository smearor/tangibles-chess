//! MCP endpoint: command dispatch, replies, screenshots, and the
//! game-state watch channel.

use gdk4::prelude::PaintableExt;
use gdk4::prelude::StaticType;
use gdk4::prelude::TextureExt;
use glib::MainContext;
use gtk4::prelude::Cast;
use gtk4::prelude::GskRendererExt;
use gtk4::prelude::NativeExt;
use gtk4::prelude::SnapshotExt;
use gtk4::prelude::WidgetExt;
use libadwaita as adw;
use shakmaty::Position;
use shakmaty::uci::UciMove;
use std::cell::RefCell;
use std::net::SocketAddr;
use tangibles_chess_material::Role;
use tangibles_chess_mcp::CommandError;
use tangibles_chess_mcp::CommandKind;
use tangibles_chess_mcp::CommandReply;
use tangibles_chess_mcp::DrawReply;
use tangibles_chess_mcp::GameStateReply;
use tangibles_chess_mcp::LegalMovesReply;
use tangibles_chess_mcp::MakeMoveReply;
use tangibles_chess_mcp::PlacementReply;
use tangibles_chess_mcp::ResignReply;
use tangibles_chess_mcp::ScreenshotReply;
use tangibles_chess_mcp::UndoReply;
use tangibles_chess_moves::MoveOutcome;
use tangibles_chess_moves::MoveRequest;
use tangibles_chess_notation::SquareName;
use tangibles_chess_notation::Uci;
use tangibles_chess_session::GameOutcome;
use tracing::info;

use super::game_controller::GameController;
use super::registry::with_controller;
use crate::playable::Playable;

thread_local! {
    /// Latest-state channel feeding MCP `chess://game-state` resource
    /// subscribers; installed by `attach_mcp`.
    static MCP_STATE_WATCH: RefCell<Option<tokio::sync::watch::Sender<serde_json::Value>>> = const { RefCell::new(None) };
}

impl GameController {
    /// Starts the MCP server on `addr` and drains its command channel on
    /// the GTK main loop. Each request is executed against the *active*
    /// controller - which only exists while a game runs; before that the
    /// endpoint replies `{"error": "no active game"}`.
    pub fn attach_mcp(addr: SocketAddr) {
        let (tx, rx) = async_channel::unbounded::<tangibles_chess_mcp::GameRequest>();
        let (watch_tx, watch_rx) = tokio::sync::watch::channel(serde_json::json!({}));
        MCP_STATE_WATCH.with(|slot| *slot.borrow_mut() = Some(watch_tx));
        tangibles_chess_mcp::spawn_http(tx, addr, watch_rx);
        MainContext::default().spawn_local(async move {
            while let Ok(request) = rx.recv().await {
                let mut reply = CommandReply::Error(CommandError::NoActiveGame);
                with_controller(|controller| {
                    reply = controller.borrow_mut().handle_mcp(&request.command);
                });
                let _ = request.reply.send(reply);
            }
        });
        info!(%addr, "MCP control endpoint enabled");
    }

    /// Executes one MCP command against the session and returns its
    /// typed reply.
    fn handle_mcp(&mut self, command: &CommandKind) -> CommandReply {
        match command {
            CommandKind::GameState => CommandReply::GameState(self.mcp_game_state()),
            CommandKind::LegalMoves { square } => {
                let position = self.session.position();
                let moves: Vec<Uci> = position
                    .legal_moves()
                    .iter()
                    .filter(|mv| square.is_none_or(|s| mv.from() == Some(s)))
                    .map(Uci::from_move)
                    .collect();
                CommandReply::LegalMoves(LegalMovesReply {
                    legal_moves: moves,
                    turn: self.session.turn(),
                })
            }
            CommandKind::MakeMove { uci } => {
                let parsed = UciMove::try_from(uci).ok().and_then(|mv| mv.to_move(self.session.position()).ok());
                let Some(mv) = parsed else {
                    return CommandReply::Error(CommandError::InvalidMove { uci: uci.clone() });
                };
                let request = MoveRequest {
                    from: mv.from().unwrap_or_else(|| mv.to()),
                    to: mv.to(),
                    promotion: mv.promotion().map(Role::from),
                };
                match self.apply_request(request) {
                    MoveOutcome::Applied(san) => CommandReply::MakeMove(MakeMoveReply::Applied {
                        applied: san,
                        fen: self.session.fen(),
                    }),
                    MoveOutcome::NeedsPromotion { to } => CommandReply::MakeMove(MakeMoveReply::NeedsPromotion {
                        needs_promotion: SquareName::from(to),
                        hint: "append a role letter, e.g. 'e7e8q'".to_string(),
                    }),
                    MoveOutcome::Illegal(reason) => CommandReply::MakeMove(MakeMoveReply::Illegal { illegal: reason.to_string() }),
                }
            }
            CommandKind::Undo => match self.session.undo() {
                Some(plan) => {
                    let steps: Vec<String> = plan.steps.iter().map(|step| step.to_string()).collect();
                    self.sync_scene();
                    CommandReply::Undo(UndoReply::Done {
                        undone: true,
                        steps,
                        fen: self.session.fen(),
                    })
                }
                None => CommandReply::Undo(UndoReply::Empty {
                    undone: false,
                    reason: "nothing to undo".to_string(),
                }),
            },
            CommandKind::Resign { color } => {
                self.session.resign(*color);
                self.session.finish_if_terminal();
                self.sync_scene();
                CommandReply::Resign(ResignReply {
                    resigned: *color,
                    outcome: self.session.outcome().map(GameOutcome::from),
                })
            }
            CommandKind::AgreeDraw => {
                self.session.agree_draw();
                self.session.finish_if_terminal();
                self.sync_scene();
                CommandReply::Draw(DrawReply {
                    draw_agreed: true,
                    outcome: self.session.outcome().map(GameOutcome::from),
                })
            }
            CommandKind::Screenshot => self.mcp_screenshot(),
            CommandKind::PlacementStatus => {
                let status = self.gate.check(self.session.position(), &self.probe);
                CommandReply::Placement(PlacementReply {
                    placed: status.placed(),
                    total: status.total,
                    missing: status.missing.iter().map(|s| SquareName::from(*s)).collect(),
                    unexpected: status.unexpected.iter().map(|s| SquareName::from(*s)).collect(),
                    consistent: status.is_consistent(),
                })
            }
        }
    }

    /// PNG screenshot of the window for the MCP `screenshot` tool.
    fn mcp_screenshot(&self) -> CommandReply {
        let Some(texture) = self.screenshot_texture() else {
            return CommandReply::Error(CommandError::WindowNotRealized);
        };
        let bytes = texture.save_to_png_bytes();
        CommandReply::Screenshot(ScreenshotReply {
            png_base64: gtk4::glib::base64_encode(bytes.as_ref()).to_string(),
            width: texture.width(),
            height: texture.height(),
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
            info!("screenshot saved to {}", path.display());
            self.screen.show_message(&self.l10n.msg("game-screenshot-saved"));
        }
    }

    /// Pushes a fresh state snapshot to MCP `chess://game-state` resource
    /// subscribers, if the endpoint and any subscribers exist.
    pub(super) fn notify_mcp_state(&self) {
        MCP_STATE_WATCH.with(|slot| {
            if let Some(tx) = slot.borrow().as_ref() {
                let _ = tx.send(tangibles_chess_mcp::to_value(&self.mcp_game_state()));
            }
        });
    }

    /// Game-state snapshot for the MCP `game_state` tool.
    fn mcp_game_state(&self) -> GameStateReply {
        let config = self.session.config();
        GameStateReply {
            phase: self.session.phase(),
            turn: self.session.turn(),
            fen: self.session.fen(),
            san_history: self.session.san_history().to_vec(),
            uci_history: self.session.uci_history().to_vec(),
            outcome: self.session.outcome().map(GameOutcome::from),
            tangible_mode: config.tangible_mode,
            claimable_draws: self.session.claimable_draws(),
        }
    }
}
