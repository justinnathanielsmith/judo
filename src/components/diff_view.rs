use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

pub struct DiffView<'a> {
    pub diff_content: Option<&'a str>,
    pub scroll_offset: u16,
    pub theme: &'a Theme,
}

impl<'a> Widget for DiffView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let content = self.diff_content.unwrap_or("No diff selected");

        let mut lines = Vec::new();
        for line in content.lines() {
            let style = if line.starts_with("diff ") || line.starts_with("index ") {
                self.theme.diff_header
            } else if line.starts_with("--- ") || line.starts_with("+++ ") {
                self.theme.diff_header
            } else if line.starts_with("@@") {
                self.theme.diff_hunk
            } else if line.starts_with('+') {
                self.theme.diff_add
            } else if line.starts_with('-') {
                self.theme.diff_remove
            } else if line.starts_with("File:") || line.starts_with("Diff for") {
                self.theme.diff_header
            } else {
                self.theme.diff_context
            };
            lines.push(Line::from(Span::styled(line, style)));
        }

        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0))
            .render(area, buf);
    }
}
