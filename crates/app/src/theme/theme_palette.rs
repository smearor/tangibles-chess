use crate::theme::TextureLayout;
use crate::theme::ThemeSlot;
use smearor_commons_color::Color;
use smearor_commons_color::Palette;
use smearor_commons_color::palette;
use tangibles_chess_config::BoardTheme;

/// Color set of one board theme plus its texture metadata.
///
/// The artistic direction: board as material (wood inlay, slate, felt),
/// restrained warm glows for legal targets and last-move traces.
#[derive(Debug, Clone)]
pub struct ThemePalette {
    /// The theme's colors keyed by their semantic slot.
    colors: Palette<ThemeSlot, f64>,
    /// Embedded luminance texture for the squares (`"wood"`, `"slate"`,
    /// `"fabric"`, `"marble"`), multiply-composited over the board.
    pub texture: Option<&'static str>,
    /// How the texture maps onto the board.
    pub texture_layout: TextureLayout,
}

impl ThemePalette {
    /// The palette for a theme.
    pub fn of(theme: BoardTheme) -> Self {
        let (colors, texture, texture_layout) = match theme {
            BoardTheme::WalnutMaple => (
                palette! {
                    ThemeSlot::Light => Color::<f64>::new(0.93, 0.83, 0.62),
                    ThemeSlot::Dark => Color::<f64>::new(0.45, 0.30, 0.18),
                    ThemeSlot::Frame => Color::<f64>::new(0.26, 0.16, 0.09),
                    ThemeSlot::Selection => Color::<f64>::new(0.98, 0.82, 0.30),
                    ThemeSlot::Target => Color::<f64>::new(0.98, 0.75, 0.28),
                    ThemeSlot::LastMove => Color::<f64>::new(0.55, 0.78, 0.44),
                    ThemeSlot::Check => Color::<f64>::new(0.90, 0.28, 0.22),
                    ThemeSlot::Ceremony => Color::<f64>::new(0.98, 0.90, 0.55),
                },
                Some("wood"),
                TextureLayout::PerSquare,
            ),
            BoardTheme::Slate => (
                palette! {
                    ThemeSlot::Light => Color::<f64>::new(0.78, 0.80, 0.84),
                    ThemeSlot::Dark => Color::<f64>::new(0.24, 0.27, 0.32),
                    ThemeSlot::Frame => Color::<f64>::new(0.13, 0.15, 0.18),
                    ThemeSlot::Selection => Color::<f64>::new(0.55, 0.75, 0.98),
                    ThemeSlot::Target => Color::<f64>::new(0.45, 0.70, 0.95),
                    ThemeSlot::LastMove => Color::<f64>::new(0.50, 0.72, 0.62),
                    ThemeSlot::Check => Color::<f64>::new(0.92, 0.34, 0.30),
                    ThemeSlot::Ceremony => Color::<f64>::new(0.70, 0.85, 0.98),
                },
                Some("slate"),
                TextureLayout::BoardWide,
            ),
            BoardTheme::TournamentGreen => (
                palette! {
                    ThemeSlot::Light => Color::<f64>::new(0.91, 0.88, 0.74),
                    ThemeSlot::Dark => Color::<f64>::new(0.20, 0.42, 0.26),
                    ThemeSlot::Frame => Color::<f64>::new(0.10, 0.24, 0.14),
                    ThemeSlot::Selection => Color::<f64>::new(0.98, 0.85, 0.35),
                    ThemeSlot::Target => Color::<f64>::new(0.95, 0.80, 0.30),
                    ThemeSlot::LastMove => Color::<f64>::new(0.65, 0.82, 0.45),
                    ThemeSlot::Check => Color::<f64>::new(0.88, 0.25, 0.22),
                    ThemeSlot::Ceremony => Color::<f64>::new(0.95, 0.90, 0.50),
                },
                Some("fabric"),
                TextureLayout::BoardWide,
            ),
            BoardTheme::Marble => (
                palette! {
                    ThemeSlot::Light => Color::<f64>::new(0.85, 0.84, 0.80),
                    ThemeSlot::Dark => Color::<f64>::new(0.30, 0.32, 0.34),
                    ThemeSlot::Frame => Color::<f64>::new(0.18, 0.19, 0.21),
                    ThemeSlot::Selection => Color::<f64>::new(0.95, 0.75, 0.35),
                    ThemeSlot::Target => Color::<f64>::new(0.88, 0.68, 0.32),
                    ThemeSlot::LastMove => Color::<f64>::new(0.60, 0.70, 0.55),
                    ThemeSlot::Check => Color::<f64>::new(0.88, 0.28, 0.24),
                    ThemeSlot::Ceremony => Color::<f64>::new(0.90, 0.85, 0.55),
                },
                Some("marble"),
                TextureLayout::BoardWide,
            ),
        };
        let palette = Self {
            colors,
            texture,
            texture_layout,
        };
        debug_assert!(palette.colors.is_complete());
        palette
    }

    /// The color registered for `slot`.
    ///
    /// All palettes built by [`ThemePalette::of`] are complete; a
    /// missing slot falls back to black rather than panicking.
    pub fn color(&self, slot: ThemeSlot) -> Color<f64> {
        self.colors.get_or(&slot, Color::<f64>::BLACK)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every theme fills all declared color slots.
    #[test]
    fn all_themes_have_complete_palettes() {
        for theme in [BoardTheme::WalnutMaple, BoardTheme::Slate, BoardTheme::TournamentGreen, BoardTheme::Marble] {
            let palette = ThemePalette::of(theme);
            assert!(palette.colors.is_complete(), "{theme:?} misses {:?}", palette.colors.missing());
        }
    }
}
