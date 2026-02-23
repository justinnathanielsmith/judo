use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Clear, Row, Table, Widget},
};

use super::helpers::{centered_rect, draw_drop_shadow};

pub struct HelpModal<'a> {
    pub theme: &'a Theme,
}

impl Widget for HelpModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let help_area = centered_rect(70, 80, area);
        if help_area.width == 0 || help_area.height == 0 {
            return;
        }
        draw_drop_shadow(buf, help_area, area);
        Clear.render(help_area, buf);

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(" HELP - KEYBINDINGS ", self.theme.header_active),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_focus);

        let key_style = self.theme.footer_segment_key;
        let desc_style = self.theme.list_item;
        let category_style = self.theme.header_item;

        let rows = vec![
            // Navigation
            Row::new(vec![
                Cell::from(Span::styled("Navigation", category_style)),
                Cell::from(""),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" j / \u{2193}", key_style)),
                Cell::from(Span::styled("Select next revision", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" k / \u{2191}", key_style)),
                Cell::from(Span::styled("Select previous revision", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" Enter", key_style)),
                Cell::from(Span::styled("Toggle diff panel", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" Tab / l", key_style)),
                Cell::from(Span::styled("Focus diff panel", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" h", key_style)),
                Cell::from(Span::styled("Focus revision graph", desc_style)),
            ]),
            Row::new(vec![Cell::from(""), Cell::from("")]),
            // Operations
            Row::new(vec![
                Cell::from(Span::styled("Operations", category_style)),
                Cell::from(""),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" s", key_style)),
                Cell::from(Span::styled("Snapshot working copy", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" e", key_style)),
                Cell::from(Span::styled("Edit selected revision", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" n", key_style)),
                Cell::from(Span::styled("Create new child revision", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" d", key_style)),
                Cell::from(Span::styled("Describe revision", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" a", key_style)),
                Cell::from(Span::styled("Abandon revision", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" S", key_style)),
                Cell::from(Span::styled("Squash into parent", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" b", key_style)),
                Cell::from(Span::styled("Set bookmark", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" B", key_style)),
                Cell::from(Span::styled("Delete bookmark", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" u / U", key_style)),
                Cell::from(Span::styled("Undo / Redo", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" f / p", key_style)),
                Cell::from(Span::styled("Fetch / Push", desc_style)),
            ]),
            Row::new(vec![Cell::from(""), Cell::from("")]),
            // Filtering
            Row::new(vec![
                Cell::from(Span::styled("Filtering", category_style)),
                Cell::from(""),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" /", key_style)),
                Cell::from(Span::styled("Custom revset filter", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" m", key_style)),
                Cell::from(Span::styled("Filter: mine()", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" t", key_style)),
                Cell::from(Span::styled("Filter: trunk()", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" c", key_style)),
                Cell::from(Span::styled("Filter: conflicts()", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" C", key_style)),
                Cell::from(Span::styled("Clear active filter", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" Tab", key_style)),
                Cell::from(Span::styled(
                    "Toggle Recent/Presets (in / modal)",
                    desc_style,
                )),
            ]),
            Row::new(vec![Cell::from(""), Cell::from("")]),
            // General
            Row::new(vec![
                Cell::from(Span::styled("General", category_style)),
                Cell::from(""),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" ?", key_style)),
                Cell::from(Span::styled("Show this help", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" Esc", key_style)),
                Cell::from(Span::styled("Close modal / Clear errors", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" q", key_style)),
                Cell::from(Span::styled("Quit", desc_style)),
            ]),
        ];

        let table = Table::new(
            rows,
            [Constraint::Percentage(30), Constraint::Percentage(70)],
        )
        .block(block);

        table.render(help_area, buf);
    }
}
