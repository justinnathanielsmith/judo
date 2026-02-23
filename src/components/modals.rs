use crate::app::state::{AppTextArea, ContextMenuState, ErrorSeverity, ErrorState};
use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Widget,
    },
};

pub struct HelpModal<'a> {
    pub theme: &'a Theme,
}

impl Widget for HelpModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let help_area = centered_rect(70, 80, area);
        if help_area.width == 0 || help_area.height == 0 {
            return;
        }
        draw_drop_shadow(buf, help_area, area);
        Clear.render(help_area, buf);

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(" HELP - KEYBINDINGS ", self.theme.header_active),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_focus);

        let key_style = self.theme.footer_segment_key;
        let desc_style = self.theme.list_item;
        let category_style = self.theme.header_item;

        let rows = vec![
            // Navigation
            Row::new(vec![
                Cell::from(Span::styled("Navigation", category_style)),
                Cell::from(""),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" j / ↓", key_style)),
                Cell::from(Span::styled("Select next revision", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" k / ↑", key_style)),
                Cell::from(Span::styled("Select previous revision", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" Enter", key_style)),
                Cell::from(Span::styled("Toggle diff panel", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" Tab / l", key_style)),
                Cell::from(Span::styled("Focus diff panel", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" h", key_style)),
                Cell::from(Span::styled("Focus revision graph", desc_style)),
            ]),
            Row::new(vec![Cell::from(""), Cell::from("")]),
            // Operations
            Row::new(vec![
                Cell::from(Span::styled("Operations", category_style)),
                Cell::from(""),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" s", key_style)),
                Cell::from(Span::styled("Snapshot working copy", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" e", key_style)),
                Cell::from(Span::styled("Edit selected revision", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" n", key_style)),
                Cell::from(Span::styled("Create new child revision", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" d", key_style)),
                Cell::from(Span::styled("Describe revision", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" a", key_style)),
                Cell::from(Span::styled("Abandon revision", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" S", key_style)),
                Cell::from(Span::styled("Squash into parent", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" b", key_style)),
                Cell::from(Span::styled("Set bookmark", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" B", key_style)),
                Cell::from(Span::styled("Delete bookmark", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" u / U", key_style)),
                Cell::from(Span::styled("Undo / Redo", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" f / p", key_style)),
                Cell::from(Span::styled("Fetch / Push", desc_style)),
            ]),
            Row::new(vec![Cell::from(""), Cell::from("")]),
            // Filtering
            Row::new(vec![
                Cell::from(Span::styled("Filtering", category_style)),
                Cell::from(""),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" /", key_style)),
                Cell::from(Span::styled("Custom revset filter", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" m", key_style)),
                Cell::from(Span::styled("Filter: mine()", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" t", key_style)),
                Cell::from(Span::styled("Filter: trunk()", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" c", key_style)),
                Cell::from(Span::styled("Filter: conflicts()", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" C", key_style)),
                Cell::from(Span::styled("Clear active filter", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" Tab", key_style)),
                Cell::from(Span::styled(
                    "Toggle Recent/Presets (in / modal)",
                    desc_style,
                )),
            ]),
            Row::new(vec![Cell::from(""), Cell::from("")]),
            // General
            Row::new(vec![
                Cell::from(Span::styled("General", category_style)),
                Cell::from(""),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" ?", key_style)),
                Cell::from(Span::styled("Show this help", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" Esc", key_style)),
                Cell::from(Span::styled("Close modal / Clear errors", desc_style)),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(" q", key_style)),
                Cell::from(Span::styled("Quit", desc_style)),
            ]),
        ];

        let table = Table::new(
            rows,
            [Constraint::Percentage(30), Constraint::Percentage(70)],
        )
        .block(block);

        table.render(help_area, buf);
    }
}

pub struct TextInputModal<'a> {
    pub theme: &'a Theme,
    pub title: &'a str,
    pub text_area: &'a AppTextArea<'a>,
    pub height_percent: u16,
}

impl Widget for TextInputModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = if self.height_percent == 0 {
            centered_rect_fixed_height(60, 3, area)
        } else {
            centered_rect(60, self.height_percent, area)
        };

        if modal_area.width == 0 || modal_area.height == 0 {
            return;
        }

        draw_drop_shadow(buf, modal_area, area);
        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(self.title, self.theme.header_active),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_focus);

        let inner_area = block.inner(modal_area);
        block.render(modal_area, buf);

        let padded_area = if self.height_percent > 0 {
            Rect {
                x: inner_area.x + 1,
                y: inner_area.y + 1,
                width: inner_area.width.saturating_sub(2),
                height: inner_area.height.saturating_sub(2),
            }
        } else {
            inner_area
        };

        if padded_area.width > 0 && padded_area.height > 0 {
            Widget::render(self.text_area, padded_area, buf);
        }
    }
}

pub struct ContextMenuModal<'a> {
    pub theme: &'a Theme,
    pub state: &'a ContextMenuState,
}

impl Widget for ContextMenuModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let menu_area = self.state.calculate_rect(area);
        if menu_area.width == 0 || menu_area.height == 0 {
            return;
        }

        draw_drop_shadow(buf, menu_area, area);
        Clear.render(menu_area, buf);

        let items: Vec<ListItem> = self
            .state
            .actions
            .iter()
            .enumerate()
            .map(|(i, (name, _))| {
                if i == self.state.selected_index {
                    ListItem::new(format!("> {name}")).style(self.theme.list_selected)
                } else {
                    ListItem::new(format!("  {name}")).style(self.theme.list_item)
                }
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(self.theme.border_focus),
        );
        list.render(menu_area, buf);
    }
}

pub struct CommandPaletteModal<'a> {
    pub theme: &'a Theme,
    pub state: &'a crate::app::state::CommandPaletteState,
}

impl Widget for CommandPaletteModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(60, 40, area);
        if modal_area.width == 0 || modal_area.height == 0 {
            return;
        }

        draw_drop_shadow(buf, modal_area, area);
        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(" COMMAND PALETTE ", self.theme.header_active),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_focus);

        let inner_area = block.inner(modal_area);
        block.render(modal_area, buf);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Query input
                Constraint::Length(1), // Separator
                Constraint::Min(0),    // Results
            ])
            .split(inner_area);

        // Render Query
        let query_line = Line::from(vec![
            Span::styled(" > ", self.theme.footer_segment_key),
            Span::styled(&self.state.query, self.theme.footer_segment_val),
            Span::styled(
                "_",
                self.theme
                    .footer_segment_val
                    .add_modifier(ratatui::style::Modifier::SLOW_BLINK),
            ),
        ]);
        buf.set_line(layout[0].x, layout[0].y, &query_line, layout[0].width);

        // Render Separator
        let separator = "─".repeat(layout[1].width as usize);
        buf.set_string(layout[1].x, layout[1].y, separator, self.theme.border_focus);

        // Render Results
        let commands = crate::app::command_palette::get_commands();
        let items: Vec<ListItem> = self
            .state
            .matches
            .iter()
            .enumerate()
            .map(|(i, &cmd_idx)| {
                let cmd = &commands[cmd_idx];
                let style = if i == self.state.selected_index {
                    self.theme.list_selected
                } else {
                    self.theme.list_item
                };

                let prefix = if i == self.state.selected_index {
                    "> "
                } else {
                    "  "
                };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(format!("{:<15}", cmd.name), style),
                    Span::styled(
                        format!(" - {}", cmd.description),
                        self.theme
                            .list_item
                            .add_modifier(ratatui::style::Modifier::DIM),
                    ),
                ]))
            })
            .collect();

        if items.is_empty() {
            let no_results = Line::from(vec![Span::styled(
                "  No commands found.",
                self.theme
                    .list_item
                    .add_modifier(ratatui::style::Modifier::DIM),
            )]);
            buf.set_line(layout[2].x, layout[2].y + 1, &no_results, layout[2].width);
        } else {
            let list = List::new(items);
            list.render(layout[2], buf);
        }
    }
}

pub struct ThemeSelectionModal<'a> {
    pub theme: &'a Theme,
    pub state: &'a crate::app::state::ThemeSelectionState,
}

impl Widget for ThemeSelectionModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(40, 30, area);
        if modal_area.width == 0 || modal_area.height == 0 {
            return;
        }

        draw_drop_shadow(buf, modal_area, area);
        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(" SELECT THEME ", self.theme.header_active),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_focus);

        let items: Vec<ListItem> = self
            .state
            .themes
            .iter()
            .enumerate()
            .map(|(i, palette)| {
                let style = if i == self.state.selected_index {
                    self.theme.list_selected
                } else {
                    self.theme.list_item
                };

                let prefix = if i == self.state.selected_index {
                    "> "
                } else {
                    "  "
                };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(palette.label(), style),
                ]))
            })
            .collect();

        let list = List::new(items).block(block);
        list.render(modal_area, buf);
    }
}

pub struct EvologModal<'a> {
    pub theme: &'a Theme,
    pub state: &'a crate::app::state::EvologState,
}

impl Widget for EvologModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(80, 80, area);
        if modal_area.width == 0 || modal_area.height == 0 {
            return;
        }

        draw_drop_shadow(buf, modal_area, area);
        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(" EVOLUTION LOG ", self.theme.header_active),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_focus);

        let inner_area = block.inner(modal_area);
        block.render(modal_area, buf);

        let lines: Vec<ListItem> = self
            .state
            .content
            .iter()
            .map(|l: &String| ListItem::new(l.as_str()).style(self.theme.list_item))
            .collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(self.state.scroll as usize));

        let list = List::new(lines);
        ratatui::widgets::StatefulWidget::render(list, inner_area, buf, &mut list_state);

        // Render scroll hint/footer
        let hint_area = Rect::new(
            modal_area.x,
            modal_area.y + modal_area.height - 1,
            modal_area.width,
            1,
        );
        let hint = Line::from(vec![
            Span::raw(" Press "),
            Span::styled("Esc", self.theme.footer_segment_key),
            Span::raw(" to close | "),
            Span::styled("j/k", self.theme.footer_segment_key),
            Span::raw(" to scroll "),
        ]);
        let hint_paragraph = Paragraph::new(hint).alignment(ratatui::layout::Alignment::Center);
        hint_paragraph.render(hint_area, buf);
    }
}

pub struct OperationLogModal<'a> {
    pub theme: &'a Theme,
    pub state: &'a crate::app::state::OperationLogState,
}

impl Widget for OperationLogModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(80, 80, area);
        if modal_area.width == 0 || modal_area.height == 0 {
            return;
        }

        draw_drop_shadow(buf, modal_area, area);
        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(" OPERATION LOG ", self.theme.header_active),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_focus);

        let inner_area = block.inner(modal_area);
        block.render(modal_area, buf);

        let lines: Vec<ListItem> = self
            .state
            .content
            .iter()
            .map(|l: &String| ListItem::new(l.as_str()).style(self.theme.list_item))
            .collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(self.state.scroll as usize));

        let list = List::new(lines);
        ratatui::widgets::StatefulWidget::render(list, inner_area, buf, &mut list_state);

        // Render scroll hint/footer
        let hint_area = Rect::new(
            modal_area.x,
            modal_area.y + modal_area.height - 1,
            modal_area.width,
            1,
        );
        let hint = Line::from(vec![
            Span::raw(" Press "),
            Span::styled("Esc", self.theme.footer_segment_key),
            Span::raw(" to close | "),
            Span::styled("j/k", self.theme.footer_segment_key),
            Span::raw(" to scroll "),
        ]);
        let hint_paragraph = Paragraph::new(hint).alignment(ratatui::layout::Alignment::Center);
        hint_paragraph.render(hint_area, buf);
    }
}

pub struct ErrorModal<'a> {
    pub theme: &'a Theme,
    pub error: &'a ErrorState,
}

impl Widget for ErrorModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(60, 20, area);
        if modal_area.width == 0 || modal_area.height == 0 {
            return;
        }

        draw_drop_shadow(buf, modal_area, area);
        Clear.render(modal_area, buf);

        let (title_text, title_style, icon) = match self.error.severity {
            ErrorSeverity::Info => (" INFO ", self.theme.header_item, "󰋼"),
            ErrorSeverity::Warning => (" WARNING ", self.theme.header_warn, "󱈸"),
            ErrorSeverity::Error => (" ERROR ", self.theme.status_error, "󰅚"),
            ErrorSeverity::Critical => (" CRITICAL ", self.theme.status_error, "󰀦"),
        };

        let block = Block::default()
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(title_text, title_style),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(title_style);

        let timestamp = self.error.timestamp.format("%H:%M:%S").to_string();

        let mut text_lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(format!("{icon} "), title_style),
                Span::styled(&self.error.message, self.theme.footer_segment_val),
            ]),
            Line::from(vec![Span::styled(
                format!("Occurred at: {timestamp}"),
                self.theme.list_item,
            )]),
            Line::from(""),
        ];

        if !self.error.suggestions.is_empty() {
            text_lines.push(Line::from(Span::styled(
                "Suggestions:",
                self.theme.header_item,
            )));
            for suggestion in &self.error.suggestions {
                text_lines.push(Line::from(vec![
                    Span::styled("  • ", self.theme.header_item),
                    Span::styled(suggestion, self.theme.footer_segment_key),
                ]));
            }
            text_lines.push(Line::from(""));
        }

        text_lines.push(Line::from(vec![
            Span::raw(" Press "),
            Span::styled("Esc", self.theme.footer_segment_key),
            Span::raw(" to acknowledge "),
        ]));

        let paragraph = Paragraph::new(text_lines)
            .alignment(ratatui::layout::Alignment::Center)
            .block(block);

        paragraph.render(modal_area, buf);
    }
}

pub struct ModalManager<'a> {
    pub theme: &'a Theme,
    pub app_state: &'a crate::app::state::AppState<'a>,
}

impl Widget for ModalManager<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use crate::app::state::AppMode;

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
                let modal_area = centered_rect_fixed_height(80, 5, area);
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
                    let modal_area = centered_rect(80, 80, area);
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

                    let recent_items: Vec<ListItem> = self
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
                            ListItem::new(format!("{prefix}{f}")).style(style)
                        })
                        .collect();

                    let recent_list = List::new(recent_items).block(
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

                    let preset_items: Vec<ListItem> = self
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
                            ListItem::new(format!("{prefix}{f}")).style(style)
                        })
                        .collect();

                    let preset_list = List::new(preset_items).block(
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

pub fn dim_area(buf: &mut Buffer, area: Rect) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.set_style(cell.style().add_modifier(ratatui::style::Modifier::DIM));
        }
    }
}

// Helper functions extracted from ui.rs

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

fn centered_rect_fixed_height(percent_x: u16, height: u16, r: Rect) -> Rect {
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

fn draw_drop_shadow(buf: &mut Buffer, area: Rect, terminal_area: Rect) {
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

fn render_revset_categories(
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
