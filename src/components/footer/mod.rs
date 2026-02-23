mod groups;
mod types;

use crate::app::state::AppState;
use crate::theme::Theme;
pub use types::{FooterGroup, FooterItem};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct Footer<'a> {
    pub state: &'a AppState<'a>,
    pub theme: &'a Theme,
}

impl Widget for Footer<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let state = self.state;

        // Active filter indicator
        let filter_span = if let Some(revset) = &state.revset {
            Span::styled(format!("  FILTER: {revset}  "), theme.header_warn)
        } else {
            Span::raw("")
        };

        // Status segment
        let status_span = if let Some(err) = &state.last_error {
            Span::styled(format!("  ERROR: {}  ", err.message), theme.status_error)
        } else if let Some(msg) = &state.status_message {
            Span::styled(format!("  {msg}  "), theme.status_info)
        } else {
            Span::styled("  READY  ", theme.status_ready)
        };

        let mut spans = vec![status_span, Span::raw(" ")];

        // Show active filter badge
        if !filter_span.content.is_empty() {
            spans.push(filter_span);
            spans.push(Span::raw(" "));
        }

        // Repo context (Workspace, WC & Operation)
        if !state.workspace_id.is_empty() {
            spans.push(Span::styled(
                format!(" {} ", state.workspace_id),
                theme.header_item,
            ));
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(&state.header_state.wc_text, theme.header_item));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(&state.header_state.op_text, theme.header_item));
        spans.push(Span::raw(" "));

        // Background tasks
        if state.active_tasks.is_empty() {
            spans.push(Span::raw("  "));
        } else {
            let tasks_text = format!(
                " {} tasks: {} ",
                state.spinner,
                state.active_tasks.join(", ")
            );
            spans.push(Span::styled(tasks_text, theme.status_info));
            spans.push(Span::raw("  "));
        }

        let groups = groups::get_groups(state);

        let available_width = area.width.saturating_sub(4); // Margin
        let mut current_width = spans
            .iter()
            .map(ratatui::prelude::Span::width)
            .sum::<usize>();

        for group in groups {
            if group.items.is_empty() {
                continue;
            }

            // Check if we can fit at least the first item of the group
            let first_item = &group.items[0];
            let first_item_width = first_item.key.len() + first_item.desc.len() + 4;

            if current_width + first_item_width > available_width as usize {
                break;
            }

            // Add group name as a subtle label if there's plenty of space
            if area.width > 100 {
                let group_label =
                    Span::styled(format!("{}: ", group.name), theme.footer_group_name);
                if current_width + group_label.width() + first_item_width < available_width as usize
                {
                    spans.push(group_label);
                    current_width += group.name.len() + 2;
                }
            }

            for item in group.items {
                let key_str = format!(" {} ", item.key);
                let desc_str = format!(" {} ", item.desc);

                let item_width = key_str.len() + desc_str.len();
                if current_width + item_width + 1 > available_width as usize {
                    break;
                }

                let key_style = if item.highlighted {
                    theme.header_active
                } else {
                    theme.footer_segment_key
                };

                let val_style = if item.highlighted {
                    theme
                        .header_active
                        .add_modifier(ratatui::style::Modifier::DIM)
                } else {
                    theme.footer_segment_val
                };

                spans.push(Span::styled(key_str, key_style));
                spans.push(Span::styled(desc_str, val_style));
                spans.push(Span::raw(" "));
                current_width += item_width + 1;
            }
            spans.push(Span::raw("  "));
            current_width += 2;
        }

        Paragraph::new(Line::from(spans))
            .style(theme.footer)
            .render(area, buf);
    }
}
