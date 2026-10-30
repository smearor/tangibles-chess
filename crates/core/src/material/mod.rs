//! Board material vocabulary - who (`Side`) and what (`Role`)
//! occupies squares, combined as `Piece`.

mod piece;
mod role;
mod side;

pub use piece::Piece;
pub use role::Role;
pub use side::Side;
