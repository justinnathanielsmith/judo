use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Widget},
};

use super::helpers::{centered_rect, draw_drop_shadow};

pub struct OperationLogModal<'a> {
    pub theme: &'a Theme,
    pub state: &'a crate::app::state::OperationLogState,
}

impl Widget for OperationLogModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(80, 80, area);
        if modal_area.width == 0 || modal_area.height == 0 {
            return;
        }

        draw_drop_shadow(buf, modal_area, area);
        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(" OPERATION LOG ", self.theme.header_active),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_focus);

        let inner_area = block.inner(modal_area);
        block.render(modal_area, buf);

        let lines: Vec<ListItem> = self
            .state
            .content
            .iter()
            .map(|l: &String| ListItem::new(l.as_str()).style(self.theme.list_item))
            .collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(self.state.scroll as usize));

        let list = List::new(lines);
        ratatui::widgets::StatefulWidget::render(list, inner_area, buf, &mut list_state);

        // Render scroll hint/footer
        let hint_area = Rect::new(
            modal_area.x,
            modal_area.y + modal_area.height - 1,
            modal_area.width,
            1,
        );
        let hint = Line::from(vec![
            Span::raw(" Press "),
            Span::styled("Esc", self.theme.footer_segment_key),
            Span::raw(" to close | "),
            Span::styled("j/k", self.theme.footer_segment_key),
            Span::raw(" to scroll "),
        ]);
        let hint_paragraph = Paragraph::new(hint).alignment(ratatui::layout::Alignment::Center);
        hint_paragraph.render(hint_area, buf);
    }
}
