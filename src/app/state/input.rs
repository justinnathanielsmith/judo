use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use std::ops::{Deref, DerefMut};
use tui_textarea::{CursorMove, TextArea};

#[derive(Default)]
pub struct AppTextArea<'a>(pub TextArea<'a>);

impl Clone for AppTextArea<'_> {
    fn clone(&self) -> Self {
        let mut area = TextArea::new(self.0.lines().to_vec());
        let (row, col) = self.0.cursor();
        area.move_cursor(CursorMove::Jump(row as u16, col as u16));
        Self(area)
    }
}

impl std::fmt::Debug for AppTextArea<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppTextArea")
            .field("lines", &self.0.lines())
            .field("cursor", &self.0.cursor())
            .finish()
    }
}

impl PartialEq for AppTextArea<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.lines() == other.0.lines() && self.0.cursor() == other.0.cursor()
    }
}

impl<'a> Deref for AppTextArea<'a> {
    type Target = TextArea<'a>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AppTextArea<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Widget for &AppTextArea<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Widget::render(&self.0, area, buf);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputState<'a> {
    pub text_area: AppTextArea<'a>,
}
