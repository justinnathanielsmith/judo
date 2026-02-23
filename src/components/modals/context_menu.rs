use crate::app::state::ContextMenuState;
use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Widget},
};

use super::helpers::draw_drop_shadow;

pub struct ContextMenuModal<'a> {
    pub theme: &'a Theme,
    pub state: &'a ContextMenuState,
}

impl Widget for ContextMenuModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let menu_area = self.state.calculate_rect(area);
        if menu_area.width == 0 || menu_area.height == 0 {
            return;
        }

        draw_drop_shadow(buf, menu_area, area);
        Clear.render(menu_area, buf);

        let items: Vec<ListItem> = self
            .state
            .actions
            .iter()
            .enumerate()
            .map(|(i, (name, _))| {
                if i == self.state.selected_index {
                    ListItem::new(format!("> {name}")).style(self.theme.list_selected)
                } else {
                    ListItem::new(format!("  {name}")).style(self.theme.list_item)
                }
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(self.theme.border_focus),
        );
        list.render(menu_area, buf);
    }
}
