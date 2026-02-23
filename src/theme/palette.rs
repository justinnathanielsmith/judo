use ratatui::style::Color;

pub struct Palette {
    pub base: Color,
    pub mantle: Color,
    pub crust: Color,
    pub text: Color,
    pub subtext0: Color,
    pub subtext1: Color,
    pub surface0: Color,
    pub surface1: Color,
    pub surface2: Color,
    pub overlay0: Color,
    pub overlay1: Color,
    pub overlay2: Color,
    pub blue: Color,
    pub lavender: Color,
    pub sapphire: Color,
    pub sky: Color,
    pub teal: Color,
    pub green: Color,
    pub yellow: Color,
    pub peach: Color,
    pub maroon: Color,
    pub red: Color,
    pub mauve: Color,
    pub pink: Color,
    pub flamingo: Color,
    pub rosewater: Color,
}

/// Scale an `Rgb` color's channels by `factor` (0.0 = black, 1.0 = unchanged).
/// Used to derive subtle background tints from palette foreground colors.
/// Non-Rgb `Color` variants are returned as-is (they don't appear in these palettes).
pub fn dim_color(c: Color, factor: f32) -> Color {
    if let Color::Rgb(r, g, b) = c {
        Color::Rgb(
            (f32::from(r) * factor) as u8,
            (f32::from(g) * factor) as u8,
            (f32::from(b) * factor) as u8,
        )
    } else {
        c
    }
}
