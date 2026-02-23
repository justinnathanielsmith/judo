use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Widget},
};

use super::helpers::{centered_rect, draw_drop_shadow};

pub struct CommandPaletteModal<'a> {
    pub theme: &'a Theme,
    pub state: &'a crate::app::state::CommandPaletteState,
}

impl Widget for CommandPaletteModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(60, 40, area);
        if modal_area.width == 0 || modal_area.height == 0 {
            return;
        }

        draw_drop_shadow(buf, modal_area, area);
        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(" COMMAND PALETTE ", self.theme.header_active),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_focus);

        let inner_area = block.inner(modal_area);
        block.render(modal_area, buf);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Query input
                Constraint::Length(1), // Separator
                Constraint::Min(0),    // Results
            ])
            .split(inner_area);

        // Render Query
        let query_line = Line::from(vec![
            Span::styled(" > ", self.theme.footer_segment_key),
            Span::styled(&self.state.query, self.theme.footer_segment_val),
            Span::styled(
                "_",
                self.theme
                    .footer_segment_val
                    .add_modifier(ratatui::style::Modifier::SLOW_BLINK),
            ),
        ]);
        buf.set_line(layout[0].x, layout[0].y, &query_line, layout[0].width);

        // Render Separator
        let separator = "─".repeat(layout[1].width as usize);
        buf.set_string(layout[1].x, layout[1].y, separator, self.theme.border_focus);

        // Render Results
        let commands = crate::app::command_palette::get_commands();
        let items: Vec<ListItem> = self
            .state
            .matches
            .iter()
            .enumerate()
            .map(|(i, &cmd_idx)| {
                let cmd = &commands[cmd_idx];
                let style = if i == self.state.selected_index {
                    self.theme.list_selected
                } else {
                    self.theme.list_item
                };

                let prefix = if i == self.state.selected_index {
                    "> "
                } else {
                    "  "
                };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(format!("{:<15}", cmd.name), style),
                    Span::styled(
                        format!(" - {}", cmd.description),
                        self.theme
                            .list_item
                            .add_modifier(ratatui::style::Modifier::DIM),
                    ),
                ]))
            })
            .collect();

        if items.is_empty() {
            let no_results = Line::from(vec![Span::styled(
                "  No commands found.",
                self.theme
                    .list_item
                    .add_modifier(ratatui::style::Modifier::DIM),
            )]);
            buf.set_line(layout[2].x, layout[2].y + 1, &no_results, layout[2].width);
        } else {
            let list = List::new(items);
            list.render(layout[2], buf);
        }
    }
}
