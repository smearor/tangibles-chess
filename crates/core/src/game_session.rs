use shakmaty::CastlingMode;
use shakmaty::Chess;
use shakmaty::Color;
use shakmaty::EnPassantMode;
use shakmaty::File;
use shakmaty::KnownOutcome;
use shakmaty::Move;
use shakmaty::Position;
use shakmaty::Role;
use shakmaty::Square;
use shakmaty::fen::Fen;
use shakmaty::san::SanPlus;
use shakmaty::zobrist::Zobrist64;

use crate::DrawReason;
use crate::GameConfig;
use crate::GameError;
use crate::GamePhase;
use crate::MoveOutcome;
use crate::MoveRequest;
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
    /// Position hashes incl. initial and current (repetition detection).
    hashes: Vec<Zobrist64>,
    /// SAN strings of played moves.
    sans: Vec<String>,
    /// UCI strings of played moves (for autosave).
    ucis: Vec<String>,
    /// Side that resigned, if any.
    resigned: Option<Color>,
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
        session.resigned = saved.resigned.as_deref().and_then(|s| s.parse::<Color>().ok());
        session.agreed_draw = saved.agreed_draw;
        Ok(session)
    }

    /// The configuration this session was created with.
    pub fn config(&self) -> &GameConfig {
        &self.config
    }

    /// Current lifecycle phase.
    pub fn phase(&self) -> GamePhase {
        self.phase
    }

    /// Starts the handshake for tangible modes, or the game directly in
    /// `NoTangibles` - and when `require_placement` is off, tangibles
    /// start immediately without the physical-setup handshake.
    pub fn begin(&mut self) {
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

    /// Side to move.
    pub fn turn(&self) -> Color {
        self.position.turn()
    }

    /// The user-facing target square of a move: for castling this is the
    /// king's destination (the user taps g1/c1 or the rook itself), for
    /// all other moves `to()`.
    fn target_of(mv: &Move) -> Square {
        match mv {
            // Castle carries king/rook *origin* squares; the user taps the
            // king's destination instead.
            Move::Castle { king, rook } => {
                let file = if rook > king { File::G } else { File::C };
                Square::from_coords(file, king.rank())
            }
            _ => mv.to(),
        }
    }

    /// Legal destination squares for the piece on `square`, as the user
    /// would tap them (castling appears as the king's target).
    pub fn legal_targets(&self, square: Square) -> Vec<Square> {
        self.position
            .legal_moves()
            .into_iter()
            .filter(|mv| mv.from() == Some(square))
            .map(|mv| Self::target_of(&mv))
            .collect()
    }

    /// Validates and applies a requested move.
    ///
    /// No-op outside `InProgress` (returns `Illegal`). Promotion requests
    /// without a role return `NeedsPromotion`; the caller must re-request
    /// with `promotion` set.
    pub fn request_move(&mut self, request: MoveRequest) -> MoveOutcome {
        if self.phase != GamePhase::InProgress {
            return MoveOutcome::Illegal {
                reason: format!("game is in phase {:?}", self.phase),
            };
        }
        let candidates: Vec<Move> = self
            .position
            .legal_moves()
            .into_iter()
            .filter(|mv| mv.from() == Some(request.from) && (Self::target_of(mv) == request.to || mv.to() == request.to))
            .collect();
        if candidates.is_empty() {
            return MoveOutcome::Illegal {
                reason: format!("{}-{} is not a legal move", request.from, request.to),
            };
        }
        if candidates.iter().any(|mv| mv.is_promotion()) && request.promotion.is_none() {
            return MoveOutcome::NeedsPromotion { to: request.to };
        }
        let Some(mv) = candidates.into_iter().find(|mv| promotion_role(mv) == request.promotion) else {
            return MoveOutcome::Illegal {
                reason: "no matching move for the given promotion".to_string(),
            };
        };
        match self.position.clone().play(mv) {
            Ok(next) => {
                let san = SanPlus::from_move(self.position.clone(), mv).to_string();
                self.past.push(self.position.clone());
                self.position = next.clone();
                self.hashes.push(next.zobrist_hash::<Zobrist64>(EnPassantMode::Legal));
                self.sans.push(san.clone());
                self.ucis.push(mv.to_uci(CastlingMode::Standard).to_string());
                self.finish_if_terminal();
                MoveOutcome::Applied { san }
            }
            Err(err) => MoveOutcome::Illegal { reason: err.to_string() },
        }
    }

    /// Pops the last move, returning the [`UndoPlan`] describing the
    /// physical restoration. `None` when nothing was played or the game
    /// is finished.
    pub fn undo(&mut self) -> Option<UndoPlan> {
        if self.phase == GamePhase::Finished || self.past.is_empty() {
            return None;
        }
        let current = self.position.clone();
        self.position = self.past.pop()?;
        self.hashes.pop();
        self.sans.pop();
        self.ucis.pop();
        Some(UndoPlan::between(&self.position, &current))
    }

    /// Terminal outcome per shakmaty (mate, stalemate, material) plus
    /// session-level results (resignation, agreed draw, automatic draw
    /// rules). `None` while the game is ongoing.
    pub fn outcome(&self) -> Option<KnownOutcome> {
        if let Some(loser) = self.resigned {
            return Some(KnownOutcome::Decisive { winner: loser.other() });
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

    /// Resigns the game for `color`.
    pub fn resign(&mut self, color: Color) {
        self.resigned = Some(color);
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
    pub fn san_history(&self) -> &[String] {
        &self.sans
    }

    /// Current FEN.
    pub fn fen(&self) -> String {
        Fen::from_position(&self.position, EnPassantMode::Legal).to_string()
    }

    /// Serializes the session for autosave.
    pub fn to_saved(&self) -> SavedGame {
        SavedGame {
            config: self.config.clone(),
            moves: self.ucis.clone(),
            sans: self.sans.clone(),
            resigned: self.resigned.map(|c| c.to_string()),
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
    mv.promotion()
}

#[cfg(test)]
mod tests {
    use shakmaty::Color;
    use shakmaty::Position;
    use shakmaty::Role;
    use shakmaty::Square;

    use super::GameSession;
    use crate::GameConfig;
    use crate::GamePhase;
    use crate::MoveOutcome;
    use crate::MoveRequest;
    use crate::TangibleMode;

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
        assert!(matches!(outcome, MoveOutcome::Applied { .. }));
        if let MoveOutcome::Applied { san } = outcome {
            assert_eq!(san, "e4");
        }
        assert_eq!(session.turn(), Color::Black);
        assert_eq!(session.san_history(), &["e4".to_string()]);
    }

    #[test]
    fn illegal_move_is_rejected() {
        let mut session = session();
        let outcome = session.request_move(MoveRequest::new(Square::E2, Square::E5));
        assert!(matches!(outcome, MoveOutcome::Illegal { .. }));
        assert_eq!(session.turn(), Color::White);
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
        assert_eq!(session.san_history().last().map(String::as_str), Some("O-O"));
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
        assert_eq!(session.san_history().last().map(String::as_str), Some("bxa8=Q"));
    }

    #[test]
    fn undo_restores_position_and_returns_plan() {
        let mut session = session();
        session.request_move(MoveRequest::new(Square::E2, Square::E4));
        let plan = session.undo().expect("undo");
        assert!(!plan.is_empty());
        assert_eq!(session.san_history().len(), 0);
        assert_eq!(session.turn(), Color::White);
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
        session.resign(Color::Black);
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
