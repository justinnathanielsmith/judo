use crate::app::state::HeaderState;
use crate::theme::{glyphs, Theme};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct Header<'a> {
    pub state: &'a HeaderState,
    pub theme: &'a Theme,
    pub terminal_width: u16,
}

impl<'a> Widget for Header<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Segment background colors for separator transitions
        let logo_bg = self.theme.header_logo.bg.unwrap_or(Color::Reset);
        let repo_bg = self.theme.header_repo.bg.unwrap_or(Color::Reset);
        let branch_bg = self.theme.header_branch.bg.unwrap_or(Color::Reset);
        let stats_bg = self.theme.header_stats.bg.unwrap_or(Color::Reset);
        let base_bg = self.theme.header.bg.unwrap_or(Color::Reset);

        // Separator styles: fg = current segment bg, bg = next segment bg
        let sep_logo_repo = Style::default().fg(logo_bg).bg(repo_bg);
        let sep_repo_branch = Style::default().fg(repo_bg).bg(branch_bg);
        let sep_branch_stats = Style::default().fg(branch_bg).bg(stats_bg);
        let sep_stats_base = Style::default().fg(stats_bg).bg(base_bg);

        let repo_name = if self.state.repo_name.is_empty() {
            "no repo".to_string()
        } else {
            self.state.repo_name.clone()
        };
        let branch_name = if self.state.branch.is_empty() {
            "(detached)".to_string()
        } else {
            self.state.branch.clone()
        };
        let stats_text = self.state.stats.clone();

        let spans = vec![
            // Logo segment
            Span::styled(format!(" {} JUDO ", glyphs::REPO), self.theme.header_logo),
            Span::styled(glyphs::SEP_RIGHT, sep_logo_repo),
            // Repo segment
            Span::styled(format!(" {} ", repo_name), self.theme.header_repo),
            Span::styled(glyphs::SEP_RIGHT, sep_repo_branch),
            // Branch segment
            Span::styled(
                format!(" {} {} ", glyphs::BRANCH, branch_name),
                self.theme.header_branch,
            ),
            Span::styled(glyphs::SEP_RIGHT, sep_branch_stats),
            // Stats segment
            Span::styled(format!(" {} ", stats_text), self.theme.header_stats),
            Span::styled(glyphs::SEP_RIGHT, sep_stats_base),
            // Fill rest of line
            Span::styled(" ".repeat(self.terminal_width as usize), self.theme.header),
        ];

        Paragraph::new(Line::from(spans))
            .style(self.theme.header)
            .render(area, buf);
    }
}
