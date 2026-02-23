use super::palette::Palette;
use ratatui::style::Color;

pub const NORD: Palette = Palette {
    base: Color::Rgb(46, 52, 64),
    mantle: Color::Rgb(59, 66, 82),
    crust: Color::Rgb(43, 48, 59),
    text: Color::Rgb(236, 239, 244),
    subtext0: Color::Rgb(216, 222, 233),
    subtext1: Color::Rgb(229, 233, 240),
    surface0: Color::Rgb(76, 86, 106),
    surface1: Color::Rgb(59, 66, 82),
    surface2: Color::Rgb(67, 76, 94),
    overlay0: Color::Rgb(129, 161, 193),
    overlay1: Color::Rgb(136, 192, 208),
    overlay2: Color::Rgb(143, 188, 187),
    blue: Color::Rgb(129, 161, 193),
    lavender: Color::Rgb(180, 190, 254), // Approximation
    sapphire: Color::Rgb(136, 192, 208),
    sky: Color::Rgb(143, 188, 187),
    teal: Color::Rgb(143, 188, 187),
    green: Color::Rgb(163, 190, 140),
    yellow: Color::Rgb(235, 203, 139),
    peach: Color::Rgb(208, 135, 112),
    maroon: Color::Rgb(191, 97, 106),
    red: Color::Rgb(191, 97, 106),
    mauve: Color::Rgb(180, 142, 173),
    pink: Color::Rgb(180, 142, 173),      // Approximation
    flamingo: Color::Rgb(216, 222, 233),  // Approximation
    rosewater: Color::Rgb(216, 222, 233), // Approximation
};
