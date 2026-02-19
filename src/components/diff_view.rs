use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::Widget,
    text::Text,
};

pub struct DiffView<'a> {
    pub diff_content: Option<&'a str>,
}

impl<'a> Widget for DiffView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let content = self.diff_content.unwrap_or("No diff selected");
        let text = Text::raw(content);
        // Using Paragraph or similar
        ratatui::widgets::Paragraph::new(text).render(area, buf);
    }
}
