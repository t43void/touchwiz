//! Color themes. All colors are named constants (Quality rule 10); no RGB
//! literals appear inline in rendering code.
//!
//! Three themes ship: `void` (default, dark), `light`, and `monochrome`. The
//! active theme is cycled with `Ctrl+T`.

use engine::metrics::Finger;
use ratatui::style::Color;

/// A complete set of UI colors.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Human-readable theme name (shown in settings).
    pub name: &'static str,
    /// Window background.
    pub background: Color,
    /// Raised surfaces / panels.
    pub surface: Color,
    /// Borders and dividers.
    pub border: Color,
    /// Primary text.
    pub text_primary: Color,
    /// De-emphasized text and upcoming characters.
    pub text_muted: Color,
    /// Active cursor and live WPM.
    pub accent_cyan: Color,
    /// Personal bests and achievements.
    pub accent_gold: Color,
    /// Correctly typed characters.
    pub accent_emerald: Color,
    /// Errors and wrong keystrokes.
    pub error_red: Color,
    /// Approaching an error/latency threshold.
    pub warning_amber: Color,
    /// Eight distinct per-finger colors (index 0..8), pinky→index per hand.
    pub finger_palette: [Color; 8],
}

/// The default `void` theme (specification Section 6).
pub const VOID: Theme = Theme {
    name: "void",
    background: Color::Rgb(0x0d, 0x0d, 0x0f),
    surface: Color::Rgb(0x16, 0x16, 0x1c),
    border: Color::Rgb(0x3b, 0x3b, 0x4a),
    text_primary: Color::Rgb(0xe2, 0xe2, 0xe8),
    text_muted: Color::Rgb(0x6b, 0x6b, 0x78),
    accent_cyan: Color::Rgb(0x00, 0xd4, 0xd8),
    accent_gold: Color::Rgb(0xf0, 0xb4, 0x29),
    accent_emerald: Color::Rgb(0x10, 0xb9, 0x81),
    error_red: Color::Rgb(0xf8, 0x71, 0x71),
    warning_amber: Color::Rgb(0xfb, 0xbf, 0x24),
    finger_palette: [
        Color::Rgb(0xef, 0x44, 0x44), // left pinky
        Color::Rgb(0xf9, 0x73, 0x16), // left ring
        Color::Rgb(0xea, 0xb3, 0x08), // left middle
        Color::Rgb(0x22, 0xc5, 0x5e), // left index
        Color::Rgb(0x06, 0xb6, 0xd4), // right index
        Color::Rgb(0x3b, 0x82, 0xf6), // right middle
        Color::Rgb(0x8b, 0x5c, 0xf6), // right ring
        Color::Rgb(0xec, 0x48, 0x99), // right pinky
    ],
};

/// A bright `light` theme.
pub const LIGHT: Theme = Theme {
    name: "light",
    background: Color::Rgb(0xf5, 0xf5, 0xf7),
    surface: Color::Rgb(0xff, 0xff, 0xff),
    border: Color::Rgb(0xd0, 0xd0, 0xd8),
    text_primary: Color::Rgb(0x1a, 0x1a, 0x1f),
    text_muted: Color::Rgb(0x8a, 0x8a, 0x96),
    accent_cyan: Color::Rgb(0x08, 0x91, 0xb2),
    accent_gold: Color::Rgb(0xb4, 0x53, 0x09),
    accent_emerald: Color::Rgb(0x05, 0x96, 0x69),
    error_red: Color::Rgb(0xdc, 0x26, 0x26),
    warning_amber: Color::Rgb(0xd9, 0x77, 0x06),
    finger_palette: [
        Color::Rgb(0xb9, 0x1c, 0x1c),
        Color::Rgb(0xc2, 0x41, 0x0c),
        Color::Rgb(0xa1, 0x62, 0x07),
        Color::Rgb(0x15, 0x80, 0x3d),
        Color::Rgb(0x0e, 0x74, 0x90),
        Color::Rgb(0x1d, 0x4e, 0xd8),
        Color::Rgb(0x6d, 0x28, 0xd9),
        Color::Rgb(0xbe, 0x18, 0x5d),
    ],
};

/// A grayscale `monochrome` theme.
pub const MONOCHROME: Theme = Theme {
    name: "monochrome",
    background: Color::Rgb(0x00, 0x00, 0x00),
    surface: Color::Rgb(0x0c, 0x0c, 0x0c),
    border: Color::Rgb(0x55, 0x55, 0x55),
    text_primary: Color::Rgb(0xf0, 0xf0, 0xf0),
    text_muted: Color::Rgb(0x80, 0x80, 0x80),
    accent_cyan: Color::Rgb(0xe6, 0xe6, 0xe6),
    accent_gold: Color::Rgb(0xff, 0xff, 0xff),
    accent_emerald: Color::Rgb(0xc0, 0xc0, 0xc0),
    error_red: Color::Rgb(0x70, 0x70, 0x70),
    warning_amber: Color::Rgb(0xa0, 0xa0, 0xa0),
    finger_palette: [
        Color::Rgb(0x55, 0x55, 0x55),
        Color::Rgb(0x6e, 0x6e, 0x6e),
        Color::Rgb(0x87, 0x87, 0x87),
        Color::Rgb(0xa0, 0xa0, 0xa0),
        Color::Rgb(0xb0, 0xb0, 0xb0),
        Color::Rgb(0x97, 0x97, 0x97),
        Color::Rgb(0x7e, 0x7e, 0x7e),
        Color::Rgb(0x66, 0x66, 0x66),
    ],
};

/// All themes in cycle order (`Ctrl+T` advances through these).
pub const THEMES: [Theme; 3] = [VOID, LIGHT, MONOCHROME];

impl Theme {
    /// The color assigned to a finger, drawn from [`Theme::finger_palette`].
    /// Thumbs (space) use the muted text color.
    pub fn finger_color(&self, finger: Finger) -> Color {
        match finger {
            Finger::LeftPinky => self.finger_palette[0],
            Finger::LeftRing => self.finger_palette[1],
            Finger::LeftMiddle => self.finger_palette[2],
            Finger::LeftIndex => self.finger_palette[3],
            Finger::RightIndex => self.finger_palette[4],
            Finger::RightMiddle => self.finger_palette[5],
            Finger::RightRing => self.finger_palette[6],
            Finger::RightPinky => self.finger_palette[7],
            Finger::LeftThumb | Finger::RightThumb => self.text_muted,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        VOID
    }
}
