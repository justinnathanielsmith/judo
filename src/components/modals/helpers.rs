use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    text::{Line, Span},
};

pub fn dim_area(buf: &mut Buffer, area: Rect) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.set_style(cell.style().add_modifier(ratatui::style::Modifier::DIM));
        }
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(100u16.saturating_sub(percent_y) / 2),
            Constraint::Percentage(percent_y.min(100)),
            Constraint::Percentage(100u16.saturating_sub(percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(100u16.saturating_sub(percent_x) / 2),
            Constraint::Percentage(percent_x.min(100)),
            Constraint::Percentage(100u16.saturating_sub(percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn centered_rect_fixed_height(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height.min(r.height)),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(100u16.saturating_sub(percent_x) / 2),
            Constraint::Percentage(percent_x.min(100)),
            Constraint::Percentage(100u16.saturating_sub(percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn draw_drop_shadow(buf: &mut Buffer, area: Rect, terminal_area: Rect) {
    let shadow_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width,
        height: area.height,
    };

    let shadow_area = shadow_area.intersection(terminal_area);

    for y in shadow_area.top()..shadow_area.bottom() {
        for x in shadow_area.left()..shadow_area.right() {
            let cell = &mut buf[(x, y)];
            cell.set_style(ratatui::style::Style::default().bg(Color::Black));
            cell.set_symbol(" ");
        }
    }
}

pub fn render_revset_categories(
    buf: &mut Buffer,
    area: Rect,
    categories: &[crate::app::state::RevsetCategory],
    theme: &Theme,
) {
    let mut y = area.y;
    let max_y = area.y + area.height;

    for cat in categories {
        if y >= max_y {
            break;
        }

        // Category header
        let header = Line::from(Span::styled(format!(" {} ", cat.name), theme.header_item));
        buf.set_line(area.x, y, &header, area.width);
        y += 1;

        // Entries
        for entry in &cat.entries {
            if y >= max_y {
                break;
            }
            let line = Line::from(vec![
                Span::styled(format!("  {:<22}", entry.name), theme.footer_segment_key),
                Span::styled(
                    entry.description,
                    theme.list_item.add_modifier(ratatui::style::Modifier::DIM),
                ),
            ]);
            buf.set_line(area.x, y, &line, area.width);
            y += 1;
        }

        // Spacing between categories
        y += 1;
    }
}
