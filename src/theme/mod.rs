use ratatui::style::{Modifier, Style};
use serde::{Deserialize, Serialize};

pub mod catppuccin;
pub mod glyphs;
pub mod gruvbox;
pub mod nord;
pub mod palette;

pub use palette::{dim_color, Palette};

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub border: Style,
    pub border_focus: Style,

    pub graph_node_wc: Style,
    pub graph_node_mutable: Style,
    pub graph_node_immutable: Style,
    pub graph_node_conflict: Style,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
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
            PaletteType::CatppuccinMocha => Self::from_palette(&catppuccin::CATPPUCCIN_MOCHA),
            PaletteType::Nord => Self::from_palette(&nord::NORD),
            PaletteType::Gruvbox => Self::from_palette(&gruvbox::GRUVBOX),
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
            graph_node_conflict: Style::default().fg(p.red).add_modifier(Modifier::BOLD),
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
