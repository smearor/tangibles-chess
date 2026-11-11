use smearor_commons_color::palette_definition;

palette_definition! {
    /// The semantic color slots of a board theme: the two square
    /// colors, the frame and the accent colors for highlights.
    pub enum ThemeSlot {
        /// Light square color.
        Light => "light",
        /// Dark square color.
        Dark => "dark",
        /// Border/frame color (the "wood" around the board).
        Frame => "frame",
        /// Selected-square glow.
        Selection => "selection",
        /// Legal-target glow.
        Target => "target",
        /// Last-move trace.
        LastMove => "last_move",
        /// Check highlight (warm red).
        Check => "check",
        /// Setup-ceremony "correctly placed" glow.
        Ceremony => "ceremony"
    }
}
