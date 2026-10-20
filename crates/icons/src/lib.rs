//! Custom icon set for tangibles-chess.
//!
//! Built at compile time from `icons.toml` via
//! [`fonts_rs_generator::IconSetBuild`]: a whitelist of SVGs from the
//! fonts-rs family crates plus local `assets/icons` sources merged into
//! one GResource bundle with a type-safe `icon_names` API.
//!
//! ```rust,ignore
//! tangibles_chess_icons::TangiblesChessIcons::install().expect("icon set registration failed");
//! let image = gtk4::Image::from_icon_name(tangibles_chess_icons::icon_names::TCH_FLAG_SYMBOLIC);
//! ```

pub mod icons {
    include!(concat!(env!("OUT_DIR"), "/iconset.rs"));
}

pub use icons::*;
