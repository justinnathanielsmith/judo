use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};
use std::time::Instant;

pub struct DiffView<'a> {
    pub diff_content: Option<&'a str>,
    pub scroll_offset: u16,
    pub theme: &'a Theme,
    pub hunk_highlight_time: Option<Instant>,
}

impl<'a> Widget for DiffView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let content = match self.diff_content {
            Some(c) if !c.is_empty() => c,
            _ => {
                let text = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  No diff selected",
                        self.theme
                            .diff_header
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    )),
                    Line::from(Span::styled(
                        "  Select a revision in the log to view its changes.",
                        self.theme.timestamp,
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  (Press 'Enter' or click a revision)",
                        self.theme
                            .timestamp
                            .add_modifier(ratatui::style::Modifier::ITALIC),
                    )),
                ];
                Paragraph::new(text).render(area, buf);
                return;
            }
        };

        let is_highlighting = self
            .hunk_highlight_time
            .map(|t| t.elapsed().as_millis() < 200)
            .unwrap_or(false);

        let mut lines = Vec::new();
        for (i, line) in content.lines().enumerate() {
            let mut style = if line.starts_with("Bookmarks:") {
                self.theme.bookmark
            } else if line.starts_with("Commit ID:")
                || line.starts_with("Change ID:")
                || line.starts_with("Author:")
                || line.starts_with("Author   :")
                || line.starts_with("Committer:")
                || line.starts_with("Timestamp:")
                || line.starts_with("Date:")
                || line.starts_with("File:")
                || line.starts_with("Status:")
            {
                self.theme.diff_header
            } else if line.starts_with('+') {
                self.theme.diff_add
            } else if line.starts_with('-') {
                self.theme.diff_remove
            } else if line.starts_with("@@") {
                self.theme.diff_hunk
            } else if line.starts_with("    ") {
                // Description or code context
                self.theme.diff_context
            } else {
                self.theme.diff_context
            };

            if is_highlighting && i == self.scroll_offset as usize {
                style = style.add_modifier(ratatui::style::Modifier::REVERSED);
            }

            lines.push(Line::from(Span::styled(line, style)));
        }

        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0))
            .render(area, buf);
    }
}
