use ratatui::style::{Color, Modifier, Style};

/// Scale an `Rgb` color's channels by `factor` (0.0 = black, 1.0 = unchanged).
/// Used to derive subtle background tints from palette foreground colors.
/// Non-Rgb `Color` variants are returned as-is (they don't appear in these palettes).
fn dim_color(c: Color, factor: f32) -> Color {
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

pub mod glyphs {
    pub const REPO: &str = "󰏗";
    pub const BRANCH: &str = "";
    pub const COMMIT: &str = "󰊄";
    pub const DIFF: &str = "";
    pub const FOCUS: &str = "▌";
    pub const SEP_RIGHT: &str = "";
    pub const SEP_LEFT: &str = "";
}

pub const CATPPUCCIN_MOCHA: Palette = Palette {
    base: Color::Rgb(30, 30, 46),
    mantle: Color::Rgb(24, 24, 37),
    crust: Color::Rgb(17, 17, 27),
    text: Color::Rgb(205, 214, 244),
    subtext0: Color::Rgb(166, 173, 200),
    subtext1: Color::Rgb(186, 194, 222),
    surface0: Color::Rgb(49, 50, 68),
    surface1: Color::Rgb(69, 71, 90),
    surface2: Color::Rgb(88, 91, 112),
    overlay0: Color::Rgb(108, 112, 134),
    overlay1: Color::Rgb(127, 132, 156),
    overlay2: Color::Rgb(147, 153, 178),
    blue: Color::Rgb(137, 180, 250),
    lavender: Color::Rgb(180, 190, 254),
    sapphire: Color::Rgb(116, 199, 236),
    sky: Color::Rgb(137, 220, 235),
    teal: Color::Rgb(148, 226, 213),
    green: Color::Rgb(166, 227, 161),
    yellow: Color::Rgb(249, 226, 175),
    peach: Color::Rgb(250, 179, 135),
    maroon: Color::Rgb(235, 160, 172),
    red: Color::Rgb(243, 139, 168),
    mauve: Color::Rgb(203, 166, 247),
    pink: Color::Rgb(245, 194, 231),
    flamingo: Color::Rgb(242, 205, 205),
    rosewater: Color::Rgb(245, 224, 220),
};

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

pub const GRUVBOX: Palette = Palette {
    base: Color::Rgb(40, 40, 40),
    mantle: Color::Rgb(29, 32, 33),
    crust: Color::Rgb(20, 20, 20),
    text: Color::Rgb(235, 219, 178),
    subtext0: Color::Rgb(189, 174, 147),
    subtext1: Color::Rgb(168, 153, 132),
    surface0: Color::Rgb(60, 56, 54),
    surface1: Color::Rgb(80, 73, 69),
    surface2: Color::Rgb(102, 92, 84),
    overlay0: Color::Rgb(146, 131, 116),
    overlay1: Color::Rgb(168, 153, 132),
    overlay2: Color::Rgb(189, 174, 147),
    blue: Color::Rgb(131, 165, 152),
    lavender: Color::Rgb(177, 98, 134),
    sapphire: Color::Rgb(104, 157, 106),
    sky: Color::Rgb(142, 192, 124),
    teal: Color::Rgb(142, 192, 124),
    green: Color::Rgb(184, 187, 38),
    yellow: Color::Rgb(250, 189, 47),
    peach: Color::Rgb(254, 128, 25),
    maroon: Color::Rgb(251, 73, 52),
    red: Color::Rgb(204, 36, 29),
    mauve: Color::Rgb(211, 134, 155),
    pink: Color::Rgb(211, 134, 155),
    flamingo: Color::Rgb(214, 93, 14),
    rosewater: Color::Rgb(168, 153, 132),
};

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub border: Style,
    pub border_focus: Style,

    pub graph_node_wc: Style,
    pub graph_node_mutable: Style,
    pub graph_node_immutable: Style,
    pub graph_line: Style,
    pub graph_lanes: Vec<Style>,

    pub change_id_mutable: Style,
    pub change_id_immutable: Style,
    pub change_id_wc: Style,
    pub bookmark: Style,

    pub diff_header: Style,
    pub diff_add: Style,
    pub diff_add_bg: Style,
    pub diff_remove: Style,
    pub diff_remove_bg: Style,
    pub diff_hunk: Style,
    pub diff_context: Style,
    pub diff_modify: Style,
    pub diff_conflict: Style,

    pub author: Style,
    pub timestamp: Style,
    pub commit_id_dim: Style,

    pub status_ready: Style,
    pub status_info: Style,
    pub status_warn: Style,
    pub status_error: Style,

    pub header_logo: Style,
    pub header_repo: Style,
    pub header_branch: Style,
    pub header_stats: Style,
    pub header_active: Style,
    pub header_warn: Style,
    pub header_item: Style,
    pub header: Style,

    pub footer_segment_key: Style,
    pub footer_segment_val: Style,
    pub footer_group_name: Style,
    pub footer: Style,

    pub highlight: Style,
    pub list_selected: Style,
    pub list_item: Style,
    pub dimmed: Style,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PaletteType {
    CatppuccinMocha,
    Nord,
    Gruvbox,
}

impl PaletteType {
    #[must_use] 
    pub fn label(&self) -> &'static str {
        match self {
            PaletteType::CatppuccinMocha => "Catppuccin (Mocha)",
            PaletteType::Nord => "Nord",
            PaletteType::Gruvbox => "Gruvbox",
        }
    }

    #[must_use] 
    pub fn all() -> &'static [PaletteType] {
        &[
            PaletteType::CatppuccinMocha,
            PaletteType::Nord,
            PaletteType::Gruvbox,
        ]
    }
}

impl Theme {
    #[must_use] 
    pub fn from_palette_type(t: PaletteType) -> Self {
        match t {
            PaletteType::CatppuccinMocha => Self::from_palette(&CATPPUCCIN_MOCHA),
            PaletteType::Nord => Self::from_palette(&NORD),
            PaletteType::Gruvbox => Self::from_palette(&GRUVBOX),
        }
    }

    #[must_use] 
    pub fn from_palette(p: &Palette) -> Self {
        Self {
            border: Style::default().fg(p.surface2),
            border_focus: Style::default().fg(p.blue),

            graph_node_wc: Style::default().fg(p.blue).add_modifier(Modifier::BOLD),
            graph_node_mutable: Style::default().fg(p.mauve).add_modifier(Modifier::BOLD),
            graph_node_immutable: Style::default().fg(p.overlay1).add_modifier(Modifier::BOLD),
            graph_line: Style::default().fg(p.overlay0),
            graph_lanes: vec![
                Style::default().fg(p.red),
                Style::default().fg(p.green),
                Style::default().fg(p.yellow),
                Style::default().fg(p.blue),
                Style::default().fg(p.mauve),
                Style::default().fg(p.teal),
                Style::default().fg(p.peach),
            ],

            change_id_mutable: Style::default().fg(p.mauve).add_modifier(Modifier::BOLD),
            change_id_immutable: Style::default().fg(p.overlay1).add_modifier(Modifier::BOLD),
            change_id_wc: Style::default().fg(p.blue).add_modifier(Modifier::BOLD),
            bookmark: Style::default().fg(p.green).add_modifier(Modifier::BOLD),
            author: Style::default().fg(p.peach).add_modifier(Modifier::BOLD),
            timestamp: Style::default().fg(p.subtext0).add_modifier(Modifier::DIM),
            commit_id_dim: Style::default().fg(p.surface2).add_modifier(Modifier::DIM),

            diff_header: Style::default().fg(p.blue).add_modifier(Modifier::BOLD),
            diff_add: Style::default().fg(p.green),
            diff_add_bg: Style::default().fg(p.green).bg(dim_color(p.green, 0.18)),
            diff_remove: Style::default().fg(p.red),
            diff_remove_bg: Style::default().fg(p.red).bg(dim_color(p.red, 0.18)),
            diff_hunk: Style::default().fg(p.teal),
            diff_context: Style::default().fg(p.text),
            diff_modify: Style::default().fg(p.yellow),
            diff_conflict: Style::default().fg(p.red).add_modifier(Modifier::BOLD),

            status_ready: Style::default()
                .bg(p.green)
                .fg(p.crust)
                .add_modifier(Modifier::BOLD),
            status_info: Style::default()
                .bg(p.blue)
                .fg(p.crust)
                .add_modifier(Modifier::BOLD),
            status_warn: Style::default()
                .bg(p.yellow)
                .fg(p.crust)
                .add_modifier(Modifier::BOLD),
            status_error: Style::default()
                .bg(p.red)
                .fg(p.crust)
                .add_modifier(Modifier::BOLD),

            header_logo: Style::default()
                .bg(p.blue)
                .fg(p.crust)
                .add_modifier(Modifier::BOLD),
            header_repo: Style::default()
                .bg(p.surface1)
                .fg(p.text)
                .add_modifier(Modifier::BOLD),
            header_branch: Style::default()
                .bg(p.mauve)
                .fg(p.crust)
                .add_modifier(Modifier::BOLD),
            header_stats: Style::default().bg(p.surface0).fg(p.subtext1),
            header_active: Style::default()
                .bg(p.green)
                .fg(p.crust)
                .add_modifier(Modifier::BOLD),
            header_warn: Style::default()
                .bg(p.yellow)
                .fg(p.crust)
                .add_modifier(Modifier::BOLD),
            header_item: Style::default().bg(p.surface0).fg(p.text),
            header: Style::default().bg(p.base).fg(p.text),

            footer_segment_key: Style::default()
                .bg(p.surface0)
                .fg(p.blue)
                .add_modifier(Modifier::BOLD),
            footer_segment_val: Style::default().bg(p.base).fg(p.text),
            footer_group_name: Style::default().fg(p.subtext0).add_modifier(Modifier::DIM),
            footer: Style::default().bg(p.crust).fg(p.subtext0),

            highlight: Style::default().bg(p.surface0).add_modifier(Modifier::BOLD),
            list_selected: Style::default()
                .bg(p.blue)
                .fg(p.crust)
                .add_modifier(Modifier::BOLD),
            list_item: Style::default().fg(p.text),
            dimmed: Style::default().fg(p.overlay0).add_modifier(Modifier::DIM),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_palette_type(PaletteType::CatppuccinMocha)
    }
}
