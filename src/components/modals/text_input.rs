use crate::app::state::AppTextArea;
use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Widget},
};

use super::helpers::{centered_rect, centered_rect_fixed_height, draw_drop_shadow};

pub struct TextInputModal<'a> {
    pub theme: &'a Theme,
    pub title: &'a str,
    pub text_area: &'a AppTextArea<'a>,
    pub height_percent: u16,
}

impl Widget for TextInputModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = if self.height_percent == 0 {
            centered_rect_fixed_height(60, 3, area)
        } else {
            centered_rect(60, self.height_percent, area)
        };

        if modal_area.width == 0 || modal_area.height == 0 {
            return;
        }

        draw_drop_shadow(buf, modal_area, area);
        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(self.title, self.theme.header_active),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_focus);

        let inner_area = block.inner(modal_area);
        block.render(modal_area, buf);

        let padded_area = if self.height_percent > 0 {
            Rect {
                x: inner_area.x + 1,
                y: inner_area.y + 1,
                width: inner_area.width.saturating_sub(2),
                height: inner_area.height.saturating_sub(2),
            }
        } else {
            inner_area
        };

        if padded_area.width > 0 && padded_area.height > 0 {
            Widget::render(self.text_area, padded_area, buf);
        }
    }
}
