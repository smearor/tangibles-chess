//! The kind of a chess piece.

use serde::Deserialize;
use serde::Serialize;

use crate::Side;

/// The kind of a chess piece - the domain spelling used in move
/// requests, notices and icon names instead of `shakmaty::Role`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Pawn.
    Pawn,
    /// Knight.
    Knight,
    /// Bishop.
    Bishop,
    /// Rook.
    Rook,
    /// Queen.
    Queen,
    /// King.
    King,
}

impl Role {
    /// All roles, pawn first.
    pub const ALL: [Self; 6] = [Self::Pawn, Self::Knight, Self::Bishop, Self::Rook, Self::Queen, Self::King];

    /// Roles that can be captured (all except the king).
    pub const CAPTURABLE: [Self; 5] = [Self::Queen, Self::Rook, Self::Bishop, Self::Knight, Self::Pawn];

    /// Roles a pawn can promote to.
    pub const PROMOTABLE: [Self; 4] = [Self::Queen, Self::Rook, Self::Bishop, Self::Knight];

    /// The Fluent key naming this piece (`piece-pawn`..`piece-king`).
    pub fn l10n_key(&self) -> &'static str {
        match self {
            Self::Pawn => "piece-pawn",
            Self::Knight => "piece-knight",
            Self::Bishop => "piece-bishop",
            Self::Rook => "piece-rook",
            Self::Queen => "piece-queen",
            Self::King => "piece-king",
        }
    }

    /// The Cuernavaca icon name of `side`'s piece (`cuernavaca-k` ..
    /// `cuernavaca-o`).
    pub fn icon_name(&self, side: Side) -> &'static str {
        match (side, self) {
            (Side::White, Self::King) => "cuernavaca-k",
            (Side::White, Self::Queen) => "cuernavaca-q",
            (Side::White, Self::Rook) => "cuernavaca-r",
            (Side::White, Self::Bishop) => "cuernavaca-b",
            (Side::White, Self::Knight) => "cuernavaca-n",
            (Side::White, Self::Pawn) => "cuernavaca-p",
            (Side::Black, Self::King) => "cuernavaca-l",
            (Side::Black, Self::Queen) => "cuernavaca-w",
            (Side::Black, Self::Rook) => "cuernavaca-t",
            (Side::Black, Self::Bishop) => "cuernavaca-v",
            (Side::Black, Self::Knight) => "cuernavaca-m",
            (Side::Black, Self::Pawn) => "cuernavaca-o",
        }
    }
}

impl std::fmt::Display for Role {
    /// `"pawn"`/`"knight"`/... (matching `shakmaty::Role`'s spelling).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Pawn => "pawn",
            Self::Knight => "knight",
            Self::Bishop => "bishop",
            Self::Rook => "rook",
            Self::Queen => "queen",
            Self::King => "king",
        })
    }
}

impl From<Role> for shakmaty::Role {
    fn from(role: Role) -> Self {
        match role {
            Role::Pawn => Self::Pawn,
            Role::Knight => Self::Knight,
            Role::Bishop => Self::Bishop,
            Role::Rook => Self::Rook,
            Role::Queen => Self::Queen,
            Role::King => Self::King,
        }
    }
}

impl From<shakmaty::Role> for Role {
    fn from(role: shakmaty::Role) -> Self {
        match role {
            shakmaty::Role::Pawn => Self::Pawn,
            shakmaty::Role::Knight => Self::Knight,
            shakmaty::Role::Bishop => Self::Bishop,
            shakmaty::Role::Rook => Self::Rook,
            shakmaty::Role::Queen => Self::Queen,
            shakmaty::Role::King => Self::King,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Role;
    use crate::Side;

    #[test]
    fn converts_both_ways() {
        for role in [Role::Pawn, Role::Knight, Role::Bishop, Role::Rook, Role::Queen, Role::King] {
            assert_eq!(Role::from(shakmaty::Role::from(role)), role);
        }
    }

    #[test]
    fn cuernavaca_names_cover_all_twelve() {
        for side in Side::ALL {
            for role in [Role::Pawn, Role::Knight, Role::Bishop, Role::Rook, Role::Queen, Role::King] {
                assert!(role.icon_name(side).starts_with("cuernavaca-"));
            }
        }
    }
}
