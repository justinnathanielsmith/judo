use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

pub struct DiffView<'a> {
    pub diff_content: Option<&'a str>,
}

impl<'a> Widget for DiffView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use ratatui::style::{Color, Style};
        use ratatui::widgets::{Paragraph, Wrap};

        let content = self.diff_content.unwrap_or("No diff selected");

        let mut lines = Vec::new();
        for line in content.lines() {
            let style = if line.starts_with("diff ")
                || line.starts_with("index ")
                || line.starts_with("--- ")
                || line.starts_with("+++ ")
            {
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else if line.starts_with("@@") {
                Style::default().fg(Color::Cyan)
            } else if line.starts_with('+') {
                Style::default().fg(Color::Green)
            } else if line.starts_with('-') {
                Style::default().fg(Color::Red)
            } else if line.starts_with("File:") || line.starts_with("Diff for") {
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default()
            };
            lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
                line, style,
            )));
        }

        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}
