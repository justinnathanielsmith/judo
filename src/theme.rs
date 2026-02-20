use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub border: Style,
    pub border_focus: Style,

    pub graph_node_wc: Style,
    pub graph_node_mutable: Style,
    pub graph_node_immutable: Style,
    pub graph_line: Style,

    pub change_id_mutable: Style,
    pub change_id_immutable: Style,
    pub bookmark: Style,

    pub diff_header: Style,
    pub diff_add: Style,
    pub diff_remove: Style,
    pub diff_hunk: Style,
    pub diff_context: Style,
    pub diff_modify: Style,

    pub author: Style,
    pub timestamp: Style,
    pub commit_id_dim: Style,

    pub header_logo: Style,
    pub header_item: Style,
    pub header: Style,

    pub footer_segment_key: Style,
    pub footer_segment_val: Style,
    pub footer: Style,

    pub status_ready: Style,
    pub status_info: Style,
    pub status_error: Style,

    pub highlight: Style,
    pub list_selected: Style,
    pub list_item: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border: Style::default().fg(Color::Rgb(60, 60, 60)),
            border_focus: Style::default().fg(Color::Rgb(137, 180, 250)), // Mauve/Lavender-ish

            graph_node_wc: Style::default()
                .fg(Color::Rgb(249, 226, 175)) // Yellow
                .add_modifier(Modifier::BOLD),
            graph_node_mutable: Style::default()
                .fg(Color::Rgb(203, 166, 247)) // Mauve
                .add_modifier(Modifier::BOLD),
            graph_node_immutable: Style::default().fg(Color::Rgb(137, 180, 250)), // Blue
            graph_line: Style::default().fg(Color::Rgb(108, 112, 134)),           // Surface

            change_id_mutable: Style::default().fg(Color::Rgb(203, 166, 247)),
            change_id_immutable: Style::default().fg(Color::Rgb(137, 180, 250)),
            bookmark: Style::default()
                .fg(Color::Rgb(166, 227, 161)) // Green
                .add_modifier(Modifier::BOLD),
            author: Style::default().fg(Color::Rgb(250, 179, 135)), // Peach
            timestamp: Style::default().fg(Color::Rgb(166, 173, 200)), // Subtext
            commit_id_dim: Style::default().fg(Color::Rgb(88, 91, 112)), // Surface

            diff_header: Style::default()
                .fg(Color::Rgb(137, 180, 250))
                .add_modifier(Modifier::BOLD),
            diff_add: Style::default().fg(Color::Rgb(166, 227, 161)),
            diff_remove: Style::default().fg(Color::Rgb(243, 139, 168)), // Red
            diff_hunk: Style::default().fg(Color::Rgb(148, 226, 213)),   // Teal
            diff_context: Style::default().fg(Color::Rgb(205, 214, 244)), // Text
            diff_modify: Style::default().fg(Color::Rgb(249, 226, 175)),

            header_logo: Style::default()
                .bg(Color::Rgb(137, 180, 250))
                .fg(Color::Rgb(17, 17, 27)) // Crust
                .add_modifier(Modifier::BOLD),
            header_item: Style::default()
                .bg(Color::Rgb(49, 50, 68)) // Surface
                .fg(Color::Rgb(205, 214, 244)),
            header: Style::default()
                .bg(Color::Rgb(30, 30, 46)) // Base
                .fg(Color::Rgb(205, 214, 244)),

            footer_segment_key: Style::default()
                .bg(Color::Rgb(49, 50, 68))
                .fg(Color::Rgb(137, 180, 250))
                .add_modifier(Modifier::BOLD),
            footer_segment_val: Style::default()
                .bg(Color::Rgb(30, 30, 46))
                .fg(Color::Rgb(205, 214, 244)),
            footer: Style::default()
                .bg(Color::Rgb(17, 17, 27))
                .fg(Color::Rgb(166, 173, 200)),

            status_ready: Style::default().fg(Color::Rgb(166, 227, 161)),
            status_info: Style::default().fg(Color::Rgb(137, 180, 250)),
            status_error: Style::default()
                .fg(Color::Rgb(243, 139, 168))
                .add_modifier(Modifier::BOLD),

            highlight: Style::default()
                .bg(Color::Rgb(49, 50, 68))
                .add_modifier(Modifier::BOLD),
            list_selected: Style::default()
                .bg(Color::Rgb(137, 180, 250))
                .fg(Color::Rgb(17, 17, 27))
                .add_modifier(Modifier::BOLD),
            list_item: Style::default().fg(Color::Rgb(205, 214, 244)),
        }
    }
}
