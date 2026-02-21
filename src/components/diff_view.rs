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
        let content = match self.diff_content {
            Some(c) if !c.is_empty() => c,
            _ => {
                let text = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  No diff selected",
                        self.theme.diff_header.add_modifier(ratatui::style::Modifier::BOLD),
                    )),
                    Line::from(Span::styled(
                        "  Select a revision in the log to view its changes.",
                        self.theme.timestamp,
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  (Press 'Enter' or click a revision)",
                        self.theme.timestamp.add_modifier(ratatui::style::Modifier::ITALIC),
                    )),
                ];
                Paragraph::new(text)
                    .render(area, buf);
                return;
            }
        };

        let mut lines = Vec::new();
        for line in content.lines() {
            let style = if line.starts_with("Commit ID:")
                || line.starts_with("Change ID:")
                || line.starts_with("Author:")
                || line.starts_with("Author   :")
                || line.starts_with("Committer:")
                || line.starts_with("Timestamp:")
                || line.starts_with("Date:")
            {
                self.theme.diff_header
            } else if line.starts_with("+ Added") {
                self.theme.diff_add
            } else if line.starts_with("- Deleted") {
                self.theme.diff_remove
            } else if line.starts_with("~ Modified") {
                self.theme.diff_modify
            } else if line.starts_with("    ...") {
                self.theme.diff_hunk
            } else if line.len() >= 10 && line.as_bytes().get(9) == Some(&b':') {
                // It's a diff line. Check for addition/deletion.
                // In jj, a deletion has spaces in the second column (index 4-8)
                // An addition has spaces in the first column (index 0-3)
                let first_col = &line[0..4];
                let second_col = &line[4..9];
                if first_col.trim().is_empty() {
                    self.theme.diff_add
                } else if second_col.trim().is_empty() {
                    self.theme.diff_remove
                } else {
                    self.theme.diff_context
                }
            } else if line.starts_with("    ") {
                // This is likely the description part
                self.theme.diff_context
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
