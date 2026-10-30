//! A chess piece with its side.

use crate::Role;
use crate::Side;

/// A chess piece with its side - the domain spelling used in captures,
/// trays and notices instead of `shakmaty::Piece`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    /// The side the piece belongs to.
    pub side: Side,
    /// The kind of piece.
    pub role: Role,
}

impl Piece {
    /// The Cuernavaca icon name of this piece.
    pub fn icon_name(&self) -> &'static str {
        self.role.icon_name(self.side)
    }
}

impl std::fmt::Display for Piece {
    /// `"white pawn"`/`"black queen"`/...
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.side, self.role)
    }
}

impl From<Piece> for shakmaty::Piece {
    fn from(piece: Piece) -> Self {
        Self {
            color: piece.side.into(),
            role: piece.role.into(),
        }
    }
}

impl From<shakmaty::Piece> for Piece {
    fn from(piece: shakmaty::Piece) -> Self {
        Self {
            side: piece.color.into(),
            role: piece.role.into(),
        }
    }
}
