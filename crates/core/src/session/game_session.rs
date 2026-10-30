use shakmaty::Chess;
use shakmaty::EnPassantMode;
use shakmaty::KnownOutcome;
use shakmaty::Move;
use shakmaty::Position;
use shakmaty::Square;
use shakmaty::zobrist::Zobrist64;
use tangibles_chess_material::Piece;
use tangibles_chess_material::Role;
use tangibles_chess_material::Side;
use tangibles_chess_notation::Fen;
use tangibles_chess_notation::San;
use tangibles_chess_notation::Uci;

use crate::CheckProjection;
use crate::DrawReason;
use crate::GameConfig;
use crate::GameError;
use crate::GamePhase;
use crate::IllegalReason;
use crate::MoveOutcome;
use crate::MoveRequest;
use crate::PlayRejection;
use crate::Ply;
use crate::SavedGame;
use crate::UndoPlan;

/// A running or configured chess game.
///
/// Owns the position stack (needed for undo and threefold detection),
/// the SAN history and the phase state machine. In tangible modes the
/// session passes through `AwaitingPlacement` before moves are accepted.
#[derive(Debug, Clone)]
pub struct GameSession {
    /// Setup-screen configuration.
    config: GameConfig,
    /// Lifecycle phase.
    phase: GamePhase,
    /// Current position.
    position: Chess,
    /// Previous positions, oldest first (excludes the current one).
    past: Vec<Chess>,
    /// Undone moves available for redo, latest last: the undone
    /// position, its zobrist hash and its SAN/UCI strings.
    future: Vec<(Chess, Zobrist64, San, Uci)>,
    /// Position hashes incl. initial and current (repetition detection).
    hashes: Vec<Zobrist64>,
    /// SAN strings of played moves.
    sans: Vec<San>,
    /// UCI strings of played moves (for autosave).
    ucis: Vec<Uci>,
    /// Side that resigned, if any.
    resigned: Option<Side>,
    /// Both players agreed a draw.
    agreed_draw: bool,
}

impl GameSession {
    /// A session in `Setup` phase for the given configuration.
    pub fn new(config: GameConfig) -> Self {
        let position = Chess::default();
        Self {
            config,
            phase: GamePhase::Setup,
            hashes: vec![position.zobrist_hash::<Zobrist64>(EnPassantMode::Legal)],
            position,
            past: Vec::new(),
            future: Vec::new(),
            sans: Vec::new(),
            ucis: Vec::new(),
            resigned: None,
            agreed_draw: false,
        }
    }

    /// Rebuilds a session from an autosaved game.
    pub fn from_saved(saved: &SavedGame) -> Result<Self, GameError> {
        let positions = saved.replay()?;
        let mut session = Self::new(saved.config.clone());
        session.hashes = positions
            .iter()
            .map(|position| position.zobrist_hash::<Zobrist64>(EnPassantMode::Legal))
            .collect();
        session.position = positions.last().cloned().unwrap_or_default();
        session.past = positions[..positions.len().saturating_sub(1)].to_vec();
        session.sans = saved.sans.clone();
        session.ucis = saved.moves.clone();
        session.resigned = saved.resigned;
        session.agreed_draw = saved.agreed_draw;
        // A saved game that already ended restores its finished state;
        // `begin()` will not resurrect it.
        session.phase = GamePhase::InProgress;
        session.finish_if_terminal();
        Ok(session)
    }

    /// The configuration this session was created with.
    pub fn config(&self) -> &GameConfig {
        &self.config
    }

    /// Mutable access to the configuration - for applying live-safe
    /// option changes (theme, sound, undo) to a running game.
    pub fn config_mut(&mut self) -> &mut GameConfig {
        &mut self.config
    }

    /// Current lifecycle phase.
    pub fn phase(&self) -> GamePhase {
        self.phase
    }

    /// Starts the handshake for tangible modes, or the game directly in
    /// `NoTangibles` - and when `require_placement` is off, tangibles
    /// start immediately without the physical-setup handshake.
    pub fn begin(&mut self) {
        if self.phase == GamePhase::Finished {
            return;
        }
        self.phase = if self.config.tangible_mode.uses_tangibles() && self.config.require_placement {
            GamePhase::AwaitingPlacement
        } else {
            GamePhase::InProgress
        };
    }

    /// Releases the game once the physical board matches the position
    /// (start handshake completed).
    pub fn confirm_placement(&mut self) {
        if self.phase == GamePhase::AwaitingPlacement {
            self.phase = GamePhase::InProgress;
        }
    }

    /// The live position.
    pub fn position(&self) -> &Chess {
        &self.position
    }

    /// Number of plies played.
    pub fn ply_count(&self) -> Ply {
        Ply::from(self.past.len())
    }

    /// The position after `ply` moves ([`Ply::START`] is the initial
    /// position, `ply_count()` the current one) - for stepping through
    /// the game record in replay mode.
    pub fn position_at(&self, ply: Ply) -> Option<&Chess> {
        if ply.index() < self.past.len() {
            self.past.get(ply.index())
        } else if ply.index() == self.past.len() {
            Some(&self.position)
        } else {
            None
        }
    }

    /// Side to move.
    pub fn turn(&self) -> Side {
        Side::from(self.position.turn())
    }

    /// Pieces captured so far (material diff against the starting
    /// position), in canonical order: white's losses first, then
    /// black's, most valuable role first. Parking strips render them
    /// in this order.
    pub fn captured_pieces(&self) -> Vec<Piece> {
        let initial = self.past.first().unwrap_or(&self.position);
        let mut captured = Vec::new();
        for side in Side::ALL {
            for role in Role::CAPTURABLE {
                let piece = Piece { side, role };
                let query = shakmaty::Piece::from(piece);
                let before = initial.board().by_piece(query).count();
                let now = self.position.board().by_piece(query).count();
                captured.extend(std::iter::repeat_n(piece, before.saturating_sub(now)));
            }
        }
        captured
    }

    /// Legal destination squares for the piece on `square`, as the user
    /// would tap them (castling appears as the king's target).
    pub fn legal_targets(&self, square: Square) -> Vec<Square> {
        self.position
            .legal_moves()
            .into_iter()
            .filter(|mv| mv.from() == Some(square))
            .map(|mv| MoveRequest::target_of(&mv))
            .collect()
    }

    /// Validates and applies a requested move.
    ///
    /// No-op outside `InProgress` (returns `Illegal`). Promotion requests
    /// without a role return `NeedsPromotion`; the caller must re-request
    /// with `promotion` set.
    pub fn request_move(&mut self, request: MoveRequest) -> MoveOutcome {
        if self.phase != GamePhase::InProgress {
            return MoveOutcome::Illegal(IllegalReason::WrongPhase(self.phase));
        }
        let candidates: Vec<Move> = self.position.legal_moves().into_iter().filter(|mv| request.matches(mv)).collect();
        if candidates.is_empty() {
            return MoveOutcome::Illegal(IllegalReason::NotLegal(request));
        }
        if candidates.iter().any(|mv| mv.is_promotion()) && request.promotion.is_none() {
            return MoveOutcome::NeedsPromotion { to: request.to };
        }
        let Some(mv) = candidates.into_iter().find(|mv| promotion_role(mv) == request.promotion) else {
            return MoveOutcome::Illegal(IllegalReason::BadPromotion);
        };
        match self.position.clone().play(mv) {
            Ok(next) => {
                let san = San::from_move(self.position.clone(), mv);
                self.future.clear();
                self.past.push(self.position.clone());
                self.position = next.clone();
                self.hashes.push(next.zobrist_hash::<Zobrist64>(EnPassantMode::Legal));
                self.sans.push(san.clone());
                self.ucis.push(Uci::from_move(&mv));
                self.finish_if_terminal();
                MoveOutcome::Applied(san)
            }
            Err(err) => MoveOutcome::Illegal(IllegalReason::PlayRejected(PlayRejection::from(err))),
        }
    }

    /// Projects `request` naively (ignoring legality) and reports the
    /// check it would leave on the mover's own king, if any - the
    /// reason a move like "pinned piece" or "king into an attacked
    /// square" is illegal.
    ///
    /// Simplifications: castling maps to the king move only and an en
    /// passant capture keeps the captured pawn - both irrelevant for
    /// the check question in practice.
    pub fn projected_check(&self, request: &MoveRequest) -> Option<CheckProjection> {
        let piece = self.position.board().piece_at(request.from)?;
        if Side::from(piece.color) != self.turn() {
            return None;
        }
        let mut board = self.position.board().clone();
        board.discard_piece_at(request.from);
        board.set_piece_at(request.to, piece);
        let king = board.king_of(piece.color)?;
        let attackers: Vec<Square> = board.attacks_to(king, piece.color.other(), board.occupied()).into_iter().collect();
        (!attackers.is_empty()).then_some(CheckProjection { king, attackers })
    }

    /// Whether an undo is currently possible: allowed by the config,
    /// the game is running and there is a move to take back.
    pub fn can_undo(&self) -> bool {
        self.config.allow_undo && self.phase == GamePhase::InProgress && !self.past.is_empty()
    }

    /// Whether a redo is currently possible.
    pub fn can_redo(&self) -> bool {
        self.config.allow_undo && self.phase == GamePhase::InProgress && !self.future.is_empty()
    }

    /// Pops the last move, returning the [`UndoPlan`] describing the
    /// physical restoration. `None` when undo is not allowed, nothing
    /// was played or the game is finished.
    pub fn undo(&mut self) -> Option<UndoPlan> {
        if !self.can_undo() {
            return None;
        }
        let current = self.position.clone();
        self.position = self.past.pop()?;
        let hash = self.hashes.pop()?;
        let san = self.sans.pop()?;
        let uci = self.ucis.pop()?;
        self.future.push((current.clone(), hash, san, uci));
        Some(UndoPlan::between(&self.position, &current))
    }

    /// Re-applies the last undone move. `None` when nothing is
    /// available to redo.
    pub fn redo(&mut self) -> Option<UndoPlan> {
        if !self.can_redo() {
            return None;
        }
        let (position, hash, san, uci) = self.future.pop()?;
        let current = std::mem::replace(&mut self.position, position.clone());
        self.past.push(current.clone());
        self.hashes.push(hash);
        self.sans.push(san);
        self.ucis.push(uci);
        Some(UndoPlan::between(&position, &current))
    }

    /// Terminal outcome per shakmaty (mate, stalemate, material) plus
    /// session-level results (resignation, agreed draw, automatic draw
    /// rules). `None` while the game is ongoing.
    pub fn outcome(&self) -> Option<KnownOutcome> {
        if let Some(loser) = self.resigned {
            return Some(KnownOutcome::Decisive { winner: loser.other().into() });
        }
        if self.agreed_draw || self.automatic_draw().is_some() {
            return Some(KnownOutcome::Draw);
        }
        self.position.outcome().known()
    }

    /// Claimable draw reasons currently open to the side to move.
    pub fn claimable_draws(&self) -> Vec<DrawReason> {
        let mut reasons = Vec::new();
        if self.repetitions() >= 3 {
            reasons.push(DrawReason::ThreefoldRepetition);
        }
        if self.position.halfmoves() >= 100 {
            reasons.push(DrawReason::FiftyMoves);
        }
        reasons
    }

    /// Why the game ended drawn (agreement, an automatic rule, or
    /// stalemate), for result presentation. `None` while the game is
    /// ongoing or was decided.
    pub fn draw_reason(&self) -> Option<DrawReason> {
        if self.phase != GamePhase::Finished {
            return None;
        }
        if self.agreed_draw {
            return Some(DrawReason::Agreement);
        }
        if let Some(reason) = self.automatic_draw() {
            return Some(reason);
        }
        self.position.is_stalemate().then_some(DrawReason::Stalemate)
    }

    /// Automatic (unconditional) draw, if any applies.
    pub fn automatic_draw(&self) -> Option<DrawReason> {
        if self.position.is_insufficient_material() {
            return Some(DrawReason::InsufficientMaterial);
        }
        if self.repetitions() >= 5 {
            return Some(DrawReason::FivefoldRepetition);
        }
        if self.position.halfmoves() >= 150 {
            return Some(DrawReason::SeventyFiveMoves);
        }
        None
    }

    /// Resigns the game for `side`.
    pub fn resign(&mut self, side: Side) {
        self.resigned = Some(side);
        self.phase = GamePhase::Finished;
    }

    /// Marks the game drawn by agreement.
    pub fn agree_draw(&mut self) {
        self.agreed_draw = true;
        self.phase = GamePhase::Finished;
    }

    /// Marks a finished game (terminal outcome reached on the board).
    pub fn finish_if_terminal(&mut self) {
        if self.phase == GamePhase::InProgress && self.outcome().is_some() {
            self.phase = GamePhase::Finished;
        }
    }

    /// SAN strings of all played moves.
    pub fn san_history(&self) -> &[San] {
        &self.sans
    }

    /// Played moves in UCI notation (autosave/replay format).
    pub fn uci_history(&self) -> &[Uci] {
        &self.ucis
    }

    /// Current FEN.
    pub fn fen(&self) -> Fen {
        Fen::from_position(&self.position)
    }

    /// Serializes the session for autosave.
    pub fn to_saved(&self) -> SavedGame {
        SavedGame {
            config: self.config.clone(),
            moves: self.ucis.clone(),
            sans: self.sans.clone(),
            resigned: self.resigned,
            agreed_draw: self.agreed_draw,
        }
    }

    /// Number of times the current position occurred.
    fn repetitions(&self) -> usize {
        let Some(current) = self.hashes.last().copied() else {
            return 0;
        };
        self.hashes.iter().filter(|hash| **hash == current).count()
    }
}

/// Promoted role of a move, or `None`.
fn promotion_role(mv: &Move) -> Option<Role> {
    mv.promotion().map(Role::from)
}

#[cfg(test)]
mod tests {
    use shakmaty::Chess;
    use shakmaty::Color;
    use shakmaty::Position;
    use shakmaty::Square;

    use super::GameSession;
    use crate::GameConfig;
    use crate::GamePhase;
    use crate::MoveOutcome;
    use crate::MoveRequest;
    use crate::Ply;
    use crate::TangibleMode;
    use tangibles_chess_material::Role;
    use tangibles_chess_material::Side;
    use tangibles_chess_notation::San;

    fn session() -> GameSession {
        let mut session = GameSession::new(GameConfig::default());
        session.begin();
        session
    }

    #[test]
    fn touch_mode_starts_in_progress() {
        let session = session();
        assert_eq!(session.phase(), GamePhase::InProgress);
    }

    #[test]
    fn tangible_mode_awaits_placement() {
        let config = GameConfig {
            tangible_mode: TangibleMode::Tangibles,
            ..GameConfig::default()
        };
        let mut session = GameSession::new(config);
        session.begin();
        assert_eq!(session.phase(), GamePhase::AwaitingPlacement);
        session.confirm_placement();
        assert_eq!(session.phase(), GamePhase::InProgress);
    }

    #[test]
    fn legal_move_applies_and_records_san() {
        let mut session = session();
        let outcome = session.request_move(MoveRequest::new(Square::E2, Square::E4));
        assert!(matches!(outcome, MoveOutcome::Applied(_)));
        if let MoveOutcome::Applied(san) = outcome {
            assert_eq!(san.as_str(), "e4");
        }
        assert_eq!(session.turn(), Side::Black);
        assert_eq!(session.san_history(), &[San::from("e4")]);
    }

    #[test]
    fn illegal_move_is_rejected() {
        let mut session = session();
        let outcome = session.request_move(MoveRequest::new(Square::E2, Square::E5));
        assert!(matches!(outcome, MoveOutcome::Illegal(_)));
        assert_eq!(session.turn(), Side::White);
    }

    #[test]
    fn castle_via_king_target_applies() {
        let mut session = session();
        for mv in ["e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "g8f6"] {
            assert!(matches!(
                session.request_move(MoveRequest::new(mv[..2].parse().expect("sq"), mv[2..].parse().expect("sq"))),
                MoveOutcome::Applied { .. }
            ));
        }
        // Tap king e1, then its castling destination g1.
        let outcome = session.request_move(MoveRequest::new(Square::E1, Square::G1));
        assert!(matches!(outcome, MoveOutcome::Applied { .. }));
        assert_eq!(session.san_history().last().map(San::as_str), Some("O-O"));
    }

    #[test]
    fn promotion_requires_role_then_applies() {
        let mut session = session();
        // March the a-pawn up to b7, then promote by capturing on a8.
        for mv in ["a2a4", "b7b5", "a4b5", "g8f6", "b5b6", "a7a6", "b6b7", "f6e4"] {
            assert!(matches!(
                session.request_move(MoveRequest::new(mv[..2].parse().expect("sq"), mv[2..].parse().expect("sq"))),
                MoveOutcome::Applied { .. }
            ));
        }
        let outcome = session.request_move(MoveRequest::new(Square::B7, Square::A8));
        assert!(matches!(outcome, MoveOutcome::NeedsPromotion { .. }));
        let outcome = session.request_move(MoveRequest::promoting(Square::B7, Square::A8, Role::Queen));
        assert!(matches!(outcome, MoveOutcome::Applied { .. }));
        assert_eq!(session.san_history().last().map(San::as_str), Some("bxa8=Q"));
    }

    #[test]
    fn undo_restores_position_and_returns_plan() {
        let mut session = session();
        session.request_move(MoveRequest::new(Square::E2, Square::E4));
        let plan = session.undo().expect("undo");
        assert!(!plan.is_empty());
        assert_eq!(session.san_history().len(), 0);
        assert_eq!(session.turn(), Side::White);
    }

    #[test]
    fn undo_disabled_blocks_takebacks() {
        let mut session = GameSession::new(GameConfig {
            allow_undo: false,
            ..GameConfig::default()
        });
        session.begin();
        session.request_move(MoveRequest::new(Square::E2, Square::E4));
        assert!(!session.can_undo());
        assert!(session.undo().is_none());
        assert!(session.redo().is_none());
        assert_eq!(session.san_history().len(), 1);
    }

    #[test]
    fn redo_replays_undone_moves() {
        let mut session = session();
        session.request_move(MoveRequest::new(Square::E2, Square::E4));
        session.request_move(MoveRequest::new(Square::E7, Square::E5));
        assert!(session.undo().is_some());
        assert!(session.can_redo());
        assert!(session.redo().is_some());
        assert_eq!(session.san_history(), &[San::from("e4"), San::from("e5")]);
        assert_eq!(session.turn(), Side::White);
        assert!(!session.can_redo());
    }

    #[test]
    fn new_move_clears_redo_stack() {
        let mut session = session();
        session.request_move(MoveRequest::new(Square::E2, Square::E4));
        session.request_move(MoveRequest::new(Square::E7, Square::E5));
        session.undo();
        session.request_move(MoveRequest::new(Square::C7, Square::C5));
        assert!(!session.can_redo());
        assert_eq!(session.san_history(), &[San::from("e4"), San::from("c5")]);
    }

    #[test]
    fn projected_check_names_king_and_attackers() {
        let mut session = session();
        // Bb4 pins Nc3 against the e1 king on the b4-c3-d2-e1 diagonal
        // (d2 vacated by 2. d4).
        for mv in ["b1c3", "e7e5", "d2d4", "e5d4", "e2e4", "f8b4"] {
            assert!(matches!(
                session.request_move(MoveRequest::new(mv[..2].parse().expect("sq"), mv[2..].parse().expect("sq"))),
                MoveOutcome::Applied { .. }
            ));
        }
        // Moving the pinned knight off the diagonal exposes the king.
        let projection = session
            .projected_check(&MoveRequest::new(Square::C3, Square::E4))
            .expect("pinned move exposes check");
        assert_eq!(projection.king, Square::E1);
        assert_eq!(projection.attackers, vec![Square::B4]);
        // A king step off the diagonal leaves no check - no projection.
        assert!(session.projected_check(&MoveRequest::new(Square::E1, Square::E2)).is_none());
    }

    #[test]
    fn position_at_steps_through_history() {
        let mut session = session();
        session.request_move(MoveRequest::new(Square::E2, Square::E4));
        session.request_move(MoveRequest::new(Square::E7, Square::E5));
        assert_eq!(session.ply_count(), Ply::from(2));
        assert_eq!(session.position_at(Ply::START), Some(&Chess::default()));
        assert_eq!(session.position_at(Ply::from(2)), Some(session.position()));
        assert!(session.position_at(Ply::from(3)).is_none());
        assert_eq!(session.position_at(Ply::from(1)).map(|position| position.turn()), Some(Color::Black));
    }

    #[test]
    fn restored_finished_game_stays_finished() {
        let mut session = session();
        // Scholar's mate ends decisively inside four plies.
        for mv in ["e2e4", "e7e5", "f1c4", "b8c6", "d1h5", "g8f6", "h5f7"] {
            assert!(matches!(
                session.request_move(MoveRequest::new(mv[..2].parse().expect("sq"), mv[2..].parse().expect("sq"))),
                MoveOutcome::Applied { .. }
            ));
        }
        assert_eq!(session.phase(), GamePhase::Finished);
        let restored = GameSession::from_saved(&session.to_saved()).expect("restore");
        assert_eq!(restored.phase(), GamePhase::Finished);
        // `begin` must not resurrect the finished game.
        let mut resurrected = restored;
        resurrected.begin();
        assert_eq!(resurrected.phase(), GamePhase::Finished);
    }

    #[test]
    fn saved_roundtrip_replays_game() {
        let mut session = session();
        session.request_move(MoveRequest::new(Square::E2, Square::E4));
        session.request_move(MoveRequest::new(Square::E7, Square::E5));
        let saved = session.to_saved();
        let restored = GameSession::from_saved(&saved).expect("restore");
        assert_eq!(restored.fen(), session.fen());
        assert_eq!(restored.san_history(), session.san_history());
    }

    #[test]
    fn resignation_decides_game() {
        let mut session = session();
        session.resign(Side::Black);
        assert_eq!(session.phase(), GamePhase::Finished);
        assert!(session.outcome().is_some());
    }

    #[test]
    fn checkmate_is_detected() {
        let mut session = session();
        // Fool's mate.
        for (from, to) in [
            (Square::F2, Square::F3),
            (Square::E7, Square::E5),
            (Square::G2, Square::G4),
            (Square::D8, Square::H4),
        ] {
            assert!(matches!(session.request_move(MoveRequest::new(from, to)), MoveOutcome::Applied { .. }));
        }
        assert_eq!(session.phase(), GamePhase::Finished);
        assert!(session.position().is_checkmate());
    }
}
