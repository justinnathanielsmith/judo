use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Widget},
};

use super::helpers::{centered_rect, draw_drop_shadow};

pub struct ThemeSelectionModal<'a> {
    pub theme: &'a Theme,
    pub state: &'a crate::app::state::ThemeSelectionState,
}

impl Widget for ThemeSelectionModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(40, 30, area);
        if modal_area.width == 0 || modal_area.height == 0 {
            return;
        }

        draw_drop_shadow(buf, modal_area, area);
        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(" SELECT THEME ", self.theme.header_active),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_focus);

        let items: Vec<ListItem> = self
            .state
            .themes
            .iter()
            .enumerate()
            .map(|(i, palette)| {
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
                    Span::styled(palette.label(), style),
                ]))
            })
            .collect();

        let list = List::new(items).block(block);
        list.render(modal_area, buf);
    }
}
