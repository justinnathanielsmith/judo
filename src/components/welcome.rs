use crate::app::state::AppState;
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct Welcome<'a> {
    pub app_state: &'a AppState<'a>,
    pub theme: &'a Theme,
}

impl Widget for Welcome<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let logo_ascii = [
            r"   _ _   _ ___   ___ ",
            r"  | | | | |   \ / _ ",
            r" _| | |_| | |) | (_) |",
            r"|___|_____|___/ \___/ ",
        ];

        let mut lines: Vec<Line> = logo_ascii
            .iter()
            .map(|l| Line::from(Span::styled(*l, self.theme.header_logo)))
            .collect();

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(" JUDO ", self.theme.header_logo),
            Span::raw(" - The Jujutsu TUI"),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "No Jujutsu repository found in the current directory.",
            self.theme.status_error,
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("Press "),
            Span::styled("i", self.theme.footer_segment_key),
            Span::raw(" or "),
            Span::styled("Enter", self.theme.footer_segment_key),
            Span::raw(" to initialize a new colocated repository"),
        ]));
        lines.push(Line::from(vec![Span::styled(
            " (jj git init --colocate) ",
            self.theme.header_item,
        )]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("Press "),
            Span::styled("q", self.theme.footer_segment_key),
            Span::raw(" or "),
            Span::styled("Esc", self.theme.footer_segment_key),
            Span::raw(" to quit"),
        ]));

        if let Some(err) = &self.app_state.last_error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Error: {}", err.message),
                self.theme.status_error,
            )));
        }

        let paragraph = Paragraph::new(lines).alignment(Alignment::Center);

        let logo_height = 15;
        let centered_area = Rect {
            x: area.x,
            y: (area.y + area.height / 2).saturating_sub(logo_height / 2),
            width: area.width,
            height: logo_height.min(area.height),
        };

        if centered_area.width > 0 && centered_area.height > 0 {
            paragraph.render(centered_area, buf);
        }
    }
}
