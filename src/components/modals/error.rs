use crate::app::state::{ErrorSeverity, ErrorState};
use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
};

use super::helpers::{centered_rect, draw_drop_shadow};

pub struct ErrorModal<'a> {
    pub theme: &'a Theme,
    pub error: &'a ErrorState,
}

impl Widget for ErrorModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(60, 20, area);
        if modal_area.width == 0 || modal_area.height == 0 {
            return;
        }

        draw_drop_shadow(buf, modal_area, area);
        Clear.render(modal_area, buf);

        let (title_text, title_style, icon) = match self.error.severity {
            ErrorSeverity::Info => (" INFO ", self.theme.header_item, "󰋼"),
            ErrorSeverity::Warning => (" WARNING ", self.theme.header_warn, "󱈸"),
            ErrorSeverity::Error => (" ERROR ", self.theme.status_error, "󰅚"),
            ErrorSeverity::Critical => (" CRITICAL ", self.theme.status_error, "󰀦"),
        };

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(title_text, title_style),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(title_style);

        let timestamp = self.error.timestamp.format("%H:%M:%S").to_string();

        let mut text_lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(format!("{icon} "), title_style),
                Span::styled(&self.error.message, self.theme.footer_segment_val),
            ]),
            Line::from(vec![Span::styled(
                format!("Occurred at: {timestamp}"),
                self.theme.list_item,
            )]),
            Line::from(""),
        ];

        if !self.error.suggestions.is_empty() {
            text_lines.push(Line::from(Span::styled(
                "Suggestions:",
                self.theme.header_item,
            )));
            for suggestion in &self.error.suggestions {
                text_lines.push(Line::from(vec![
                    Span::styled("  • ", self.theme.header_item),
                    Span::styled(suggestion, self.theme.footer_segment_key),
                ]));
            }
            text_lines.push(Line::from(""));
        }

        text_lines.push(Line::from(vec![
            Span::raw(" Press "),
            Span::styled("Esc", self.theme.footer_segment_key),
            Span::raw(" to acknowledge "),
        ]));

        let paragraph = Paragraph::new(text_lines)
            .alignment(ratatui::layout::Alignment::Center)
            .block(block);

        paragraph.render(modal_area, buf);
    }
}
