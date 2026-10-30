use crate::Role;
use shakmaty::File;
use shakmaty::Move;
use shakmaty::Square;

/// A requested move between two squares, as produced by touch input or
/// the tangible move inference.
///
/// `promotion` is only consulted when `from`/`to` describe a promoting
/// pawn move; `None` there yields [`crate::MoveOutcome::NeedsPromotion`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveRequest {
    /// Origin square.
    pub from: Square,
    /// Target square.
    pub to: Square,
    /// Promotion role for pawn moves to the last rank.
    pub promotion: Option<Role>,
}

impl MoveRequest {
    /// A move request without promotion.
    pub fn new(from: Square, to: Square) -> Self {
        Self { from, to, promotion: None }
    }

    /// A move request with promotion role.
    pub fn promoting(from: Square, to: Square, promotion: Role) -> Self {
        Self {
            from,
            to,
            promotion: Some(promotion),
        }
    }

    /// Whether the legal move `mv` satisfies this request: same origin
    /// square, and the request's `to` matches either the user-facing
    /// target (king's destination for castling) or the raw `mv.to()`.
    pub fn matches(&self, mv: &Move) -> bool {
        mv.from() == Some(self.from) && (Self::target_of(mv) == self.to || mv.to() == self.to)
    }

    /// The user-facing target square of a legal move: the king's
    /// destination for castling (the user taps g1/c1 or the rook's
    /// origin), `mv.to()` for all other moves.
    pub fn target_of(mv: &Move) -> Square {
        match mv {
            // Castle carries king/rook *origin* squares; the user taps
            // the king's destination instead.
            Move::Castle { king, rook } => {
                let file = if rook > king { File::G } else { File::C };
                Square::from_coords(file, king.rank())
            }
            _ => mv.to(),
        }
    }

    /// The physical rook displacement completing a castle `mv` (rook
    /// origin to rook target); `None` for non-castling moves.
    pub fn castle_rook(mv: &Move) -> Option<MoveRequest> {
        match mv {
            // Castle `king`/`rook` are origin squares; the rook lands
            // between them on the king's rank.
            Move::Castle { king, rook } => {
                let file = if rook > king { File::F } else { File::D };
                Some(MoveRequest::new(*rook, Square::from_coords(file, king.rank())))
            }
            _ => None,
        }
    }
}
