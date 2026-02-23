use super::super::action::Action;
use crate::domain::models::CommitId;
use ratatui::layout::Rect;

#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenuState {
    pub commit_id: CommitId,
    pub x: u16,
    pub y: u16,
    pub selected_index: usize,
    pub actions: Vec<(String, Action)>,
}

impl ContextMenuState {
    #[must_use]
    pub fn calculate_rect(&self, terminal_area: Rect) -> Rect {
        let longest_action = self
            .actions
            .iter()
            .map(|(name, _)| name.len())
            .max()
            .unwrap_or(0) as u16;
        let menu_width = (longest_action + 6).min(terminal_area.width);
        let menu_height = (self.actions.len() as u16 + 2).min(terminal_area.height);

        let mut x = self.x;
        let mut y = self.y;

        if x + menu_width > terminal_area.width {
            x = terminal_area.width.saturating_sub(menu_width);
        }

        if y + menu_height > terminal_area.height {
            y = y.saturating_sub(menu_height);
        }

        Rect::new(x, y, menu_width, menu_height)
    }
}
