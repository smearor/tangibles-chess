//! The `GameController` type and its startup path.

use glib::ControlFlow;
use gtk4::Widget;
use shakmaty::Position;
use shakmaty::Square;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use std::vec::IntoIter;
use tangibles_chess_core::GameSession;
use tangibles_chess_moves::MoveRequest;
use tangibles_chess_moves::Ply;
use tangibles_chess_tangibles::BoardGeometry;
use tangibles_chess_tangibles::BoardRectMm;
use tangibles_chess_tangibles::MoveInference;
use tangibles_chess_tangibles::OccupancyProbe;
use tangibles_chess_tangibles::PlacementGate;
use tangibles_chess_tangibles::SurfaceMm;
use tangibles_chess_tangibles::TableEdge;

use super::registry::with_controller;
use super::source::Source;
use crate::l10n::L10n;
use crate::replayable::Replayable;
use crate::settings::AppSettings;
use crate::sound::SoundPlayer;
use crate::tangible_host::TangibleHost;
use crate::ui::game_screen::GameScreen;
use crate::ui::parking_strip::ParkingStrip;

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
    pub(super) host: Option<TangibleHost>,
    /// Active tangible backend.
    pub(super) source: Option<Source>,
    /// Field↔square mapping for the current orientation.
    pub(super) geometry: BoardGeometry,
    /// Occupancy state of board fields and zones.
    pub(super) probe: OccupancyProbe,
    /// Field events → move intents.
    pub(super) inference: MoveInference,
    /// Start-handshake gate.
    pub(super) gate: PlacementGate,
    /// Current touch selection (`NoTangibles`).
    pub(super) selected: Option<Square>,
    /// Move awaiting a promotion pick.
    pub(super) pending_promotion: Option<MoveRequest>,
    /// The promotion picker overlay widget (to remove on pick).
    pub(super) promotion_widget: RefCell<Option<Widget>>,
    /// Last applied move's squares for the highlight trace.
    pub(super) last_move: Option<(Square, Square)>,
    /// Board rectangle in screen mm for the registered field set.
    pub(super) board_mm: BoardRectMm,
    /// Surface size in screen mm (bounds for the parking strips).
    pub(super) window_mm: SurfaceMm,
    /// West parking strip beside the board (`cap-west-*` fields) -
    /// occupancy plus rendered captured pieces.
    pub(super) parked_west: ParkingStrip,
    /// East parking strip, mirroring `parked_west`.
    pub(super) parked_east: ParkingStrip,
    /// Whether the game-over overlay was already shown.
    pub(super) result_shown: bool,
    /// Generation counter for the illegal-move flash.
    pub(super) illegal_seq: u64,
    /// Last rejected tangible move (`from`, `to`): the physical marker
    /// is misplaced; a lift+place back onto `from` is accepted quietly.
    pub(super) rejected_move: Option<MoveRequest>,
    /// Ply cursor while replaying a finished game (`None` = live).
    pub(super) replay_index: Option<Ply>,
    /// The game-over overlay widget, kept for removal when entering
    /// replay mode.
    pub(super) result_widget: Option<Widget>,
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
        let on_ply = Rc::new(|ply: Ply| {
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
            board_mm: BoardRectMm::default(),
            window_mm: SurfaceMm::default(),
            parked_west: ParkingStrip::default(),
            parked_east: ParkingStrip::default(),
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
                let fields: Rc<RefCell<IntoIter<String>>> = Rc::new(RefCell::new(
                    Square::ALL
                        .iter()
                        .filter(|square| position.board().piece_at(**square).is_some())
                        .filter_map(|square| geometry.square_to_field(*square).map(|index| index.to_string()))
                        .collect::<Vec<_>>()
                        .into_iter(),
                ));
                glib::timeout_add_local(Duration::from_millis(120), move || match fields.borrow_mut().next() {
                    Some(field) => {
                        mock.place(&field);
                        ControlFlow::Continue
                    }
                    None => ControlFlow::Break,
                });
            }
        }
        if uses_tangibles && !has_mock {
            Self::activate_tangibles(controller.clone());
        }
        controller.borrow().screen.set_rotation_locked(uses_tangibles);
    }
}
