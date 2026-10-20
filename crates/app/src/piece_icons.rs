use shakmaty::Color;
use shakmaty::Piece;
use shakmaty::Role;

/// Maps a chess piece to its Cuernavaca icon name.
///
/// Cuernavaca encodes white pieces as `cuernavaca-{k,q,r,b,n,p}` and
/// black pieces as `cuernavaca-{l,w,t,v,m,o}`.
pub fn piece_icon_name(piece: Piece) -> &'static str {
    match (piece.color, piece.role) {
        (Color::White, Role::King) => "cuernavaca-k",
        (Color::White, Role::Queen) => "cuernavaca-q",
        (Color::White, Role::Rook) => "cuernavaca-r",
        (Color::White, Role::Bishop) => "cuernavaca-b",
        (Color::White, Role::Knight) => "cuernavaca-n",
        (Color::White, Role::Pawn) => "cuernavaca-p",
        (Color::Black, Role::King) => "cuernavaca-l",
        (Color::Black, Role::Queen) => "cuernavaca-w",
        (Color::Black, Role::Rook) => "cuernavaca-t",
        (Color::Black, Role::Bishop) => "cuernavaca-v",
        (Color::Black, Role::Knight) => "cuernavaca-m",
        (Color::Black, Role::Pawn) => "cuernavaca-o",
    }
}
