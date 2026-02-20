use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub border: Style,
    pub border_focus: Style,

    pub graph_node_wc: Style,
    pub graph_node_mutable: Style,
    pub graph_node_immutable: Style,
    pub graph_line: Style,

    pub change_id: Style,
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
    pub header: Style,
    pub footer: Style,
    pub key_binding: Style,
    pub status_info: Style,
    pub status_error: Style,

    pub highlight: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border: Style::default().fg(Color::Rgb(80, 80, 80)),
            border_focus: Style::default().fg(Color::Cyan),

            graph_node_wc: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            graph_node_mutable: Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
            graph_node_immutable: Style::default().fg(Color::Rgb(100, 100, 100)),
            graph_line: Style::default().fg(Color::Rgb(100, 100, 100)),

            change_id: Style::default().fg(Color::Cyan),
            bookmark: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            author: Style::default().fg(Color::Rgb(200, 150, 100)), // Warm tan/orange
            timestamp: Style::default().fg(Color::Rgb(130, 130, 130)),
            commit_id_dim: Style::default().fg(Color::Rgb(80, 80, 80)),

            diff_header: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            diff_add: Style::default().fg(Color::Green),
            diff_remove: Style::default().fg(Color::Red),
            diff_hunk: Style::default().fg(Color::Cyan),
            diff_context: Style::default().fg(Color::Rgb(180, 180, 180)),
            diff_modify: Style::default().fg(Color::Yellow),

            header_logo: Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            header: Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White),
            footer: Style::default()
                .bg(Color::Rgb(30, 30, 30))
                .fg(Color::Rgb(150, 150, 150)),
            key_binding: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            status_info: Style::default().fg(Color::Green),
            status_error: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),

            highlight: Style::default()
                .bg(Color::Rgb(50, 50, 50))
                .add_modifier(Modifier::BOLD),
        }
    }
}
