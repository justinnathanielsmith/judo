use crate::app::state::{AppMode, AppState};
use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
};

use super::command_palette::CommandPaletteModal;
use super::context_menu::ContextMenuModal;
use super::error::ErrorModal;
use super::evolog::EvologModal;
use super::help::HelpModal;
use super::helpers::{dim_area, draw_drop_shadow, render_revset_categories};
use super::operation_log::OperationLogModal;
use super::text_input::TextInputModal;
use super::theme_selection::ThemeSelectionModal;

pub struct ModalManager<'a> {
    pub theme: &'a Theme,
    pub app_state: &'a AppState<'a>,
}

impl Widget for ModalManager<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // --- Visual Dimming ---
        let is_modal_active = !matches!(
            self.app_state.mode,
            AppMode::Normal | AppMode::Diff | AppMode::NoRepo | AppMode::Loading
        ) || self.app_state.last_error.is_some();

        if is_modal_active {
            dim_area(buf, area);
        }

        // --- Modals ---
        if self.app_state.mode == AppMode::Help {
            HelpModal { theme: self.theme }.render(area, buf);
        }

        // --- Input Modals (Describe, Bookmark, Filter) ---
        match self.app_state.mode {
            AppMode::Input | AppMode::BookmarkInput => {
                if let Some(input) = &self.app_state.input {
                    let title = if self.app_state.mode == AppMode::BookmarkInput {
                        " SET BOOKMARK "
                    } else if self.app_state.mode == AppMode::RebaseInput {
                        " REBASE DESTINATION "
                    } else {
                        " DESCRIBE REVISION "
                    };
                    TextInputModal {
                        theme: self.theme,
                        title,
                        text_area: &input.text_area,
                        height_percent: 20,
                    }
                    .render(area, buf);
                }
            }
            AppMode::RebaseSelect => {
                let modal_area = super::helpers::centered_rect_fixed_height(80, 5, area);
                draw_drop_shadow(buf, modal_area, area);
                Clear.render(modal_area, buf);
                let block = Block::default()
                    .title(Line::from(vec![
                        Span::raw(" "),
                        Span::styled(" SELECT REBASE DESTINATION ", self.theme.header_active),
                        Span::raw(" "),
                    ]))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(self.theme.border_focus);
                let text = vec![
                    Line::from("Select the destination revision in the log and press Enter."),
                    Line::from("Or press Esc to cancel."),
                ];
                Paragraph::new(text)
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(block)
                    .render(modal_area, buf);
            }
            AppMode::FilterInput => {
                if let Some(input) = &self.app_state.input {
                    let modal_area = super::helpers::centered_rect(80, 80, area);
                    draw_drop_shadow(buf, modal_area, area);
                    Clear.render(modal_area, buf);

                    // Title with active filter indicator
                    let title_spans = if let Some(active) = &self.app_state.revset {
                        vec![
                            Span::raw(" "),
                            Span::styled(" FILTER (REVSET) ", self.theme.header_active),
                            Span::raw(" "),
                            Span::styled(format!(" Active: {active} "), self.theme.header_warn),
                            Span::raw(" "),
                        ]
                    } else {
                        vec![
                            Span::raw(" "),
                            Span::styled(" FILTER (REVSET) ", self.theme.header_active),
                            Span::raw(" "),
                        ]
                    };

                    let block = Block::default()
                        .title(Line::from(title_spans))
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(self.theme.border_focus);

                    let inner_area = block.inner(modal_area);
                    block.render(modal_area, buf);

                    // Layout: Input | Separator | Lists side-by-side | Separator | Reference | Hints
                    let main_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(1), // Input
                            Constraint::Length(1), // Separator
                            Constraint::Length(8), // Recent + Preset lists
                            Constraint::Length(1), // Separator
                            Constraint::Min(0),    // Reference
                            Constraint::Length(1), // Hint bar
                        ])
                        .split(inner_area);

                    // Render Input
                    Widget::render(&input.text_area, main_layout[0], buf);

                    // Separator
                    let separator = "─".repeat(main_layout[1].width as usize);
                    buf.set_string(
                        main_layout[1].x,
                        main_layout[1].y,
                        separator,
                        self.theme.border_focus,
                    );

                    // Side-by-side: Recent Filters | Preset Filters
                    let list_layout = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(main_layout[2]);

                    let recent_style = if !self.app_state.is_selecting_presets {
                        self.theme.header_active
                    } else {
                        self.theme.header_item
                    };

                    let recent_items: Vec<ratatui::widgets::ListItem> = self
                        .app_state
                        .recent_filters
                        .iter()
                        .enumerate()
                        .map(|(i, f)| {
                            let is_selected = Some(i) == self.app_state.selected_filter_index
                                && !self.app_state.is_selecting_presets;
                            let style = if is_selected {
                                self.theme.list_selected
                            } else {
                                self.theme.list_item
                            };
                            let prefix = if is_selected { "▸ " } else { "  " };
                            ratatui::widgets::ListItem::new(format!("{prefix}{f}")).style(style)
                        })
                        .collect();

                    let recent_list = ratatui::widgets::List::new(recent_items).block(
                        Block::default()
                            .title(Span::styled(" Recent ◂Tab▸ ", recent_style))
                            .borders(Borders::RIGHT),
                    );
                    recent_list.render(list_layout[0], buf);

                    let preset_style = if self.app_state.is_selecting_presets {
                        self.theme.header_active
                    } else {
                        self.theme.header_item
                    };

                    let preset_items: Vec<ratatui::widgets::ListItem> = self
                        .app_state
                        .preset_filters
                        .iter()
                        .enumerate()
                        .map(|(i, f)| {
                            let is_selected = Some(i) == self.app_state.selected_filter_index
                                && self.app_state.is_selecting_presets;
                            let style = if is_selected {
                                self.theme.list_selected
                            } else {
                                self.theme.list_item
                            };
                            let prefix = if is_selected { "▸ " } else { "  " };
                            ratatui::widgets::ListItem::new(format!("{prefix}{f}")).style(style)
                        })
                        .collect();

                    let preset_list = ratatui::widgets::List::new(preset_items).block(
                        Block::default()
                            .title(Span::styled(" Presets ◂Tab▸ ", preset_style))
                            .borders(Borders::NONE),
                    );
                    preset_list.render(list_layout[1], buf);

                    // Separator
                    let separator = "─".repeat(main_layout[3].width as usize);
                    buf.set_string(
                        main_layout[3].x,
                        main_layout[3].y,
                        separator,
                        self.theme.border_focus,
                    );

                    // Render Categorized Revset Reference
                    let reference = crate::app::state::get_revset_reference();
                    let ref_area = main_layout[4];

                    // Split reference into columns for better use of horizontal space
                    let ref_cols = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(ref_area);

                    let half = reference.len().div_ceil(2);
                    let left_cats = &reference[..half.min(reference.len())];
                    let right_cats = if half < reference.len() {
                        &reference[half..]
                    } else {
                        &[]
                    };

                    render_revset_categories(buf, ref_cols[0], left_cats, self.theme);
                    render_revset_categories(buf, ref_cols[1], right_cats, self.theme);

                    // Hint bar
                    let hints = Line::from(vec![
                        Span::styled(" Tab", self.theme.footer_segment_key),
                        Span::styled(" Toggle  ", self.theme.list_item),
                        Span::styled("↑↓", self.theme.footer_segment_key),
                        Span::styled(" Navigate  ", self.theme.list_item),
                        Span::styled("Enter", self.theme.footer_segment_key),
                        Span::styled(" Apply  ", self.theme.list_item),
                        Span::styled("Esc", self.theme.footer_segment_key),
                        Span::styled(" Cancel", self.theme.list_item),
                    ]);
                    buf.set_line(
                        main_layout[5].x,
                        main_layout[5].y,
                        &hints,
                        main_layout[5].width,
                    );
                }
            }
            _ => {}
        }

        // --- Context Menu Popup ---
        if let (AppMode::ContextMenu, Some(menu)) =
            (self.app_state.mode, &self.app_state.context_menu)
        {
            ContextMenuModal {
                theme: self.theme,
                state: menu,
            }
            .render(area, buf);
        }

        // --- Command Palette ---
        if let (AppMode::CommandPalette, Some(cp)) =
            (self.app_state.mode, &self.app_state.command_palette)
        {
            CommandPaletteModal {
                theme: self.theme,
                state: cp,
            }
            .render(area, buf);
        }

        // --- Theme Selection ---
        if let (AppMode::ThemeSelection, Some(ts)) =
            (self.app_state.mode, &self.app_state.theme_selection)
        {
            ThemeSelectionModal {
                theme: self.theme,
                state: ts,
            }
            .render(area, buf);
        }

        // --- Evolog ---
        if let (AppMode::Evolog, Some(ev)) = (self.app_state.mode, &self.app_state.evolog_state) {
            EvologModal {
                theme: self.theme,
                state: ev,
            }
            .render(area, buf);
        }

        // --- Operation Log ---
        if let (AppMode::OperationLog, Some(op)) =
            (self.app_state.mode, &self.app_state.operation_log_state)
        {
            OperationLogModal {
                theme: self.theme,
                state: op,
            }
            .render(area, buf);
        }

        // --- Error Modal ---
        if let Some(err) = &self.app_state.last_error {
            ErrorModal {
                theme: self.theme,
                error: err,
            }
            .render(area, buf);
        }
    }
}
