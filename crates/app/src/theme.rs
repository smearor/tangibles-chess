use tangibles_chess_core::BoardTheme;

/// Color pair of one board theme plus accent colors for highlights.
///
/// The artistic direction: board as material (wood inlay, slate, felt),
/// restrained warm glows for legal targets and last-move traces.
#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    /// Light square color (r, g, b).
    pub light: (f64, f64, f64),
    /// Dark square color.
    pub dark: (f64, f64, f64),
    /// Border/frame color (the "wood" around the board).
    pub frame: (f64, f64, f64),
    /// Selected-square glow.
    pub selection: (f64, f64, f64),
    /// Legal-target glow.
    pub target: (f64, f64, f64),
    /// Last-move trace.
    pub last_move: (f64, f64, f64),
    /// Check highlight (warm red).
    pub check: (f64, f64, f64),
    /// Setup-ceremony "correctly placed" glow.
    pub ceremony: (f64, f64, f64),
}

impl ThemePalette {
    /// The palette for a theme.
    pub fn of(theme: BoardTheme) -> Self {
        match theme {
            BoardTheme::WalnutMaple => Self {
                light: (0.93, 0.83, 0.62),
                dark: (0.45, 0.30, 0.18),
                frame: (0.26, 0.16, 0.09),
                selection: (0.98, 0.82, 0.30),
                target: (0.98, 0.75, 0.28),
                last_move: (0.55, 0.78, 0.44),
                check: (0.90, 0.28, 0.22),
                ceremony: (0.98, 0.90, 0.55),
            },
            BoardTheme::Slate => Self {
                light: (0.78, 0.80, 0.84),
                dark: (0.24, 0.27, 0.32),
                frame: (0.13, 0.15, 0.18),
                selection: (0.55, 0.75, 0.98),
                target: (0.45, 0.70, 0.95),
                last_move: (0.50, 0.72, 0.62),
                check: (0.92, 0.34, 0.30),
                ceremony: (0.70, 0.85, 0.98),
            },
            BoardTheme::TournamentGreen => Self {
                light: (0.91, 0.88, 0.74),
                dark: (0.20, 0.42, 0.26),
                frame: (0.10, 0.24, 0.14),
                selection: (0.98, 0.85, 0.35),
                target: (0.95, 0.80, 0.30),
                last_move: (0.65, 0.82, 0.45),
                check: (0.88, 0.25, 0.22),
                ceremony: (0.95, 0.90, 0.50),
            },
        }
    }
}
