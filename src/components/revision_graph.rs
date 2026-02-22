use crate::app::state::{AppMode, Panel};
use crate::domain::models::{FileStatus, GraphRow, RepoStatus};
use crate::theme::{glyphs, Theme};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Paragraph, Row, StatefulWidget, Table, TableState, Widget,
    },
};

pub fn calculate_row_height(row: &GraphRow, is_selected: bool, show_diffs: bool) -> u16 {
    let num_files = if is_selected && show_diffs {
        row.changed_files.len()
    } else {
        0
    };
    2 + num_files as u16
}

pub struct RevisionGraph<'a> {
    pub repo: &'a RepoStatus,
    pub theme: &'a Theme,
    pub show_diffs: bool,
    pub selected_file_index: Option<usize>,
    pub selected_ids: &'a std::collections::HashSet<crate::domain::models::CommitId>,
    pub now_secs: i64,
}

/// Returns a copy of `style` with its `Color::Rgb` foreground dimmed by `factor` (0.0–1.0).
/// Non-Rgb fg colors are left unchanged. Used to indicate commit age on connector lines.
fn age_dimmed_style(style: Style, factor: f32) -> Style {
    if let Some(Color::Rgb(r, g, b)) = style.fg {
        style.fg(Color::Rgb(
            (r as f32 * factor) as u8,
            (g as f32 * factor) as u8,
            (b as f32 * factor) as u8,
        ))
    } else {
        style
    }
}

/// Maps commit age in days to a brightness factor for connector lines.
/// Recent commits are full-brightness; older commits fade progressively.
fn brightness_for_age(age_days: f32) -> f32 {
    if age_days < 7.0 {
        1.0
    } else if age_days < 30.0 {
        0.70
    } else if age_days < 180.0 {
        0.45
    } else {
        0.25
    }
}

impl<'a> StatefulWidget for RevisionGraph<'a> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TableState) {
        let now_secs = self.now_secs;
        let mut rows: Vec<Row> = Vec::new();

        for (i, row) in self.repo.graph.iter().enumerate() {
            let is_selected = state.selected() == Some(i);
            let row_height = calculate_row_height(row, is_selected, self.show_diffs);

            // Compute age-based brightness for this commit's connector lines.
            let age_days = (now_secs - row.timestamp_secs).max(0) as f32 / 86_400.0;
            let brightness = brightness_for_age(age_days);

            // Prepare Graph Column
            let mut graph_lines = Vec::new();
            let max_lanes = row
                .visual
                .active_lanes
                .len()
                .max(row.visual.connector_lanes.len());

            // Graph Line 1: Node symbol and existing pipes
            let mut line_1_graph = Vec::new();
            for lane_idx in 0..max_lanes {
                let lane_style = self.theme.graph_lanes[lane_idx % self.theme.graph_lanes.len()];
                if lane_idx == row.visual.column {
                    let (symbol, style) = if row.is_working_copy {
                        ("◉", self.theme.graph_node_wc)
                    } else if row.is_immutable {
                        ("◆", self.theme.graph_node_immutable)
                    } else {
                        ("○", self.theme.graph_node_mutable)
                    };
                    // Node symbols always full brightness
                    line_1_graph.push(Span::styled(symbol, style));
                } else if row
                    .visual
                    .active_lanes
                    .get(lane_idx)
                    .cloned()
                    .unwrap_or(false)
                {
                    // Connector pipes: age-dimmed
                    line_1_graph.push(Span::styled("│", age_dimmed_style(lane_style, brightness)));
                } else {
                    line_1_graph.push(Span::raw(" "));
                }
            }
            // Add spacing after graph for the "flow" look
            line_1_graph.push(Span::raw("  "));
            graph_lines.push(Line::from(line_1_graph));

            // Subsequent Graph Lines: Connector pipes and branching/merging
            for h in 1..row_height {
                let mut connector_line = Vec::new();

                let parent_cols = &row.visual.parent_columns;
                let range_min = row.visual.parent_min;
                let range_max = row.visual.parent_max;

                for lane_idx in 0..max_lanes {
                    let lane_style =
                        self.theme.graph_lanes[lane_idx % self.theme.graph_lanes.len()];
                    // All symbols in connector rows are age-dimmed
                    let dim_style = age_dimmed_style(lane_style, brightness);

                    let is_active_above = row
                        .visual
                        .active_lanes
                        .get(lane_idx)
                        .cloned()
                        .unwrap_or(false);
                    let is_active_below = row
                        .visual
                        .connector_lanes
                        .get(lane_idx)
                        .cloned()
                        .unwrap_or(false);

                    let mut symbol = if is_active_below { "│" } else { " " };

                    if h == 1 {
                        if lane_idx == row.visual.column {
                            if parent_cols.len() > 1 {
                                let has_left = parent_cols.iter().any(|p| *p < row.visual.column);
                                let has_right = parent_cols.iter().any(|p| *p > row.visual.column);
                                let has_down = parent_cols.contains(&row.visual.column);

                                symbol = match (has_left, has_right, has_down) {
                                    (true, true, true) => "┼",
                                    (true, true, false) => "┬",
                                    (true, false, true) => "┤",
                                    (false, true, true) => "├",
                                    (true, false, false) => "╮",
                                    (false, true, false) => "╭",
                                    (false, false, true) => "│",
                                    (false, false, false) => " ",
                                };
                            } else if parent_cols.len() == 1 && parent_cols[0] != row.visual.column
                            {
                                symbol = if parent_cols[0] < row.visual.column {
                                    "╮"
                                } else {
                                    "╭"
                                };
                            } else if parent_cols.is_empty() {
                                symbol = " "; // Root
                            } else {
                                symbol = "│"; // Single parent same lane
                            }
                        } else if parent_cols.contains(&lane_idx) {
                            if is_active_above {
                                symbol = if lane_idx < row.visual.column {
                                    "┤"
                                } else {
                                    "├"
                                };
                            } else {
                                symbol = if lane_idx < row.visual.column {
                                    "╭"
                                } else {
                                    "╮"
                                };
                            }
                        } else if lane_idx > range_min && lane_idx < range_max {
                            symbol = if is_active_above { "┼" } else { "─" };
                        }
                    }

                    connector_line.push(Span::styled(symbol, dim_style));
                }
                connector_line.push(Span::raw("  "));
                graph_lines.push(Line::from(connector_line));
            }

            // Prepare Details Column
            let mut detail_lines = Vec::new();

            // Line 1: ChangeId Author Timestamp CommitId

            let change_id_style = if row.is_working_copy {
                self.theme.change_id_wc
            } else if row.is_immutable {
                self.theme.change_id_immutable
            } else {
                self.theme.change_id_mutable
            };

            // Working copy shows branch icon, all commits show commit icon
            let type_glyph = if row.is_working_copy {
                glyphs::BRANCH
            } else {
                glyphs::COMMIT
            };

            let mut line_1_details = vec![
                Span::styled(type_glyph, change_id_style),
                Span::styled(" ", change_id_style),
                Span::styled(&row.change_id_short, change_id_style),
                Span::raw(" "),
                Span::styled(&row.author, self.theme.author),
                Span::raw(" "),
                Span::styled(&row.timestamp, self.theme.timestamp),
                Span::raw(" "),
            ];

            // Add bookmarks if any
            for bookmark in &row.bookmarks {
                line_1_details.push(Span::styled(bookmark.clone(), self.theme.bookmark));
                line_1_details.push(Span::raw(" "));
            }

            line_1_details.push(Span::styled(&row.commit_id_short, self.theme.commit_id_dim));
            detail_lines.push(Line::from(line_1_details));

            // Line 2: Description
            let description = row.description.lines().next().unwrap_or("");
            if description.is_empty() {
                detail_lines.push(Line::from(Span::styled(
                    "(no description set)",
                    self.theme.timestamp,
                )));
            } else {
                detail_lines.push(Line::from(Span::raw(description)));
            }

            // Line 3+: Files
            if is_selected && self.show_diffs {
                for (file_idx, file) in row.changed_files.iter().enumerate() {
                    let is_file_selected = self.selected_file_index == Some(file_idx);
                    let (prefix, mut style) = match file.status {
                        FileStatus::Added => ("+ ", self.theme.diff_add),
                        FileStatus::Modified => ("~ ", self.theme.diff_modify),
                        FileStatus::Deleted => ("- ", self.theme.diff_remove),
                        FileStatus::Conflicted => ("! ", self.theme.diff_conflict),
                    };
                    if is_file_selected {
                        style = self.theme.list_selected;
                    }
                    detail_lines.push(Line::from(vec![
                        Span::styled(prefix, style),
                        Span::styled(&file.path, style),
                    ]));
                }
            }

            let mut row_style = Style::default();
            if self.selected_ids.contains(&row.commit_id) {
                row_style = self.theme.highlight;
            }

            rows.push(
                Row::new(vec![Cell::from(graph_lines), Cell::from(detail_lines)])
                    .height(row_height)
                    .style(row_style),
            );
        }

        let table = Table::new(rows, [Constraint::Length(16), Constraint::Min(0)])
            .row_highlight_style(self.theme.highlight)
            .highlight_symbol(" ");

        StatefulWidget::render(table, area, buf, state);
    }
}

/// Panel wrapper for the revision graph that owns the Block, borders, focus styling,
/// empty/loading states. Used by `ui.rs` in place of the previously inlined logic.
pub struct RevisionGraphPanel<'a> {
    pub repo: Option<&'a crate::domain::models::RepoStatus>,
    pub theme: &'a Theme,
    pub show_diffs: bool,
    pub selected_file_index: Option<usize>,
    pub spinner: &'a str,
    pub focused_panel: Panel,
    pub mode: AppMode,
    pub revset: Option<&'a str>,
    pub selected_ids: &'a std::collections::HashSet<crate::domain::models::CommitId>,
}

impl<'a> StatefulWidget for RevisionGraphPanel<'a> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TableState) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let is_graph_focused = self.focused_panel == Panel::Graph;
        let is_body_active = self.mode == AppMode::Normal || self.mode == AppMode::Diff;

        let (border_style, title_style, borders, border_type) =
            if is_graph_focused && is_body_active {
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
                    Borders::RIGHT,
                    BorderType::Plain,
                )
            } else {
                (
                    self.theme.commit_id_dim,
                    self.theme.header_item,
                    Borders::RIGHT,
                    BorderType::Rounded,
                )
            };

        let title_spans = if is_graph_focused && is_body_active {
            vec![
                Span::styled(format!(" {} ", glyphs::FOCUS), self.theme.border_focus),
                Span::styled("REVISION GRAPH", title_style),
                Span::raw(" "),
            ]
        } else {
            vec![
                Span::raw(" "),
                Span::styled("REVISION GRAPH", title_style),
                Span::raw(" "),
            ]
        };

        let block = Block::default()
            .title(Line::from(title_spans))
            .title_bottom(Line::from(vec![
                Span::raw(" "),
                Span::styled("j/k", self.theme.footer_segment_key),
                Span::raw(": navigate "),
                Span::styled("d", self.theme.footer_segment_key),
                Span::raw(": describe "),
            ]))
            .borders(borders)
            .border_type(border_type)
            .border_style(border_style);

        let inner = block.inner(area);

        if let Some(repo) = self.repo {
            if repo.graph.is_empty() {
                let message = if let Some(revset) = self.revset {
                    if revset == "conflicts()" {
                        " 🎉 No Conflicts Found ".to_string()
                    } else {
                        format!(" No results for: {} ", revset)
                    }
                } else {
                    " Repository is empty ".to_string()
                };

                if inner.width > 0 && inner.height > 0 {
                    let lines = vec![
                        Line::from(""),
                        Line::from(Span::styled(message, self.theme.status_info)),
                        Line::from(""),
                    ];
                    let centered_area = Rect {
                        x: inner.x,
                        y: (inner.y + inner.height / 2).saturating_sub(1),
                        width: inner.width,
                        height: 3.min(inner.height),
                    };
                    if centered_area.width > 0 && centered_area.height > 0 {
                        Paragraph::new(lines)
                            .alignment(Alignment::Center)
                            .render(centered_area, buf);
                    }
                }
            } else if inner.width > 0 && inner.height > 0 {
                let graph = RevisionGraph {
                    repo,
                    theme: self.theme,
                    show_diffs: self.show_diffs,
                    selected_file_index: self.selected_file_index,
                    selected_ids: self.selected_ids,
                    now_secs: chrono::Utc::now().timestamp(),
                };
                StatefulWidget::render(graph, inner, buf, state);
            }
        } else if inner.width > 0 && inner.height > 0 {
            // Loading state
            let logo_ascii = [
                r"   _ _   _ ___   ___ ",
                r"  | | | | |   \ / _ \",
                r" _| | |_| | |) | (_) |",
                r"|___|_____|___/ \___/ ",
            ];
            let mut lines: Vec<Line> = logo_ascii
                .iter()
                .map(|l| Line::from(Span::styled(*l, self.theme.header_logo)))
                .collect();
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(self.spinner.to_string(), self.theme.header_logo),
                Span::raw(" Loading Jujutsu Repository... "),
            ]));

            let logo_height: u16 = 6;
            let centered_area = Rect {
                x: inner.x,
                y: (inner.y + inner.height / 2).saturating_sub(logo_height / 2),
                width: inner.width,
                height: logo_height.min(inner.height),
            };
            if centered_area.width > 0 && centered_area.height > 0 {
                Paragraph::new(lines)
                    .alignment(Alignment::Center)
                    .render(centered_area, buf);
            }
        }

        block.render(area, buf);
    }
}
