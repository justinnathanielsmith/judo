use crate::app::state::{AppMode, Panel};
use crate::theme::{glyphs, Theme};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap},
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

        let width = area.width as usize;

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
                // Full-line green background tint for additions
                self.theme.diff_add_bg
            } else if line.starts_with('-') {
                // Full-line red background tint for removals
                self.theme.diff_remove_bg
            } else if line.starts_with("@@") {
                self.theme.diff_hunk
            } else {
                self.theme.diff_context
            };

            if is_highlighting && i == self.scroll_offset as usize {
                style = style.add_modifier(ratatui::style::Modifier::REVERSED);
            }

            // Pad the line to the full terminal width so the background tint fills the row.
            let padded = if line.len() < width {
                format!("{:<width$}", line, width = width)
            } else {
                line.to_string()
            };

            lines.push(Line::from(Span::styled(padded, style)));
        }

        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0))
            .render(area, buf);
    }
}

/// Panel wrapper for the diff view that owns the Block, borders, and focus styling.
/// Used by `ui.rs` in place of the previously inlined logic.
pub struct DiffViewPanel<'a> {
    pub diff_content: Option<&'a str>,
    pub scroll_offset: u16,
    pub theme: &'a Theme,
    pub hunk_highlight_time: Option<Instant>,
    pub focused_panel: Panel,
    pub mode: AppMode,
}

impl<'a> Widget for DiffViewPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let is_diff_focused = self.focused_panel == Panel::Diff;
        let is_body_active = self.mode == AppMode::Normal || self.mode == AppMode::Diff;

        let (border_style, title_style, borders, border_type) = if is_diff_focused && is_body_active
        {
            (
                self.theme.border_focus,
                self.theme.header_active,
                Borders::ALL,
                BorderType::Thick,
            )
        } else if is_body_active {
            (
                self.theme.border,
                self.theme.header_item,
                Borders::LEFT,
                BorderType::Plain,
            )
        } else {
            (
                self.theme.commit_id_dim,
                self.theme.header_item,
                Borders::LEFT,
                BorderType::Rounded,
            )
        };

        let title_spans = if is_diff_focused && is_body_active {
            vec![
                Span::styled(format!(" {} ", glyphs::FOCUS), self.theme.border_focus),
                Span::styled(format!("{} DIFF VIEW", glyphs::DIFF), title_style),
                Span::raw(" "),
            ]
        } else {
            vec![
                Span::raw(" "),
                Span::styled(format!("{} DIFF VIEW", glyphs::DIFF), title_style),
                Span::raw(" "),
            ]
        };

        let block = Block::default()
            .title(Line::from(title_spans))
            .title_bottom(Line::from(vec![
                Span::raw(" "),
                Span::styled("PgUp/PgDn", self.theme.footer_segment_key),
                Span::raw(": scroll "),
                Span::styled("[/]", self.theme.footer_segment_key),
                Span::raw(": hunks "),
            ]))
            .borders(borders)
            .border_type(border_type)
            .border_style(border_style);

        let inner = block.inner(area);

        let diff_view = DiffView {
            diff_content: self.diff_content,
            scroll_offset: self.scroll_offset,
            theme: self.theme,
            hunk_highlight_time: self.hunk_highlight_time,
        };
        Widget::render(diff_view, inner, buf);
        block.render(area, buf);
    }
}
