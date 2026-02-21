use crate::app::ui;
use crate::domain::models::{FileStatus, RepoStatus};
use crate::theme::{glyphs, Theme};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Cell, Row, StatefulWidget, Table, TableState},
};

pub struct RevisionGraph<'a> {
    pub repo: &'a RepoStatus,
    pub theme: &'a Theme,
    pub show_diffs: bool,
    pub selected_file_index: Option<usize>,
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
        let now_secs = chrono::Utc::now().timestamp();
        let mut rows: Vec<Row> = Vec::new();

        for (i, row) in self.repo.graph.iter().enumerate() {
            let is_selected = state.selected() == Some(i);
            let row_height = ui::calculate_row_height(row, is_selected, self.show_diffs);

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
                let mut sorted_parents = parent_cols.clone();
                sorted_parents.sort();

                let min_p = sorted_parents.first().cloned().unwrap_or(row.visual.column);
                let max_p = sorted_parents.last().cloned().unwrap_or(row.visual.column);
                let range_min = min_p.min(row.visual.column);
                let range_max = max_p.max(row.visual.column);

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
            let change_id_short = row.change_id.get(0..8).unwrap_or(&row.change_id);
            let commit_id_short = row.commit_id.0.get(0..8).unwrap_or(&row.commit_id.0);

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
                Span::styled(format!("{} ", type_glyph), change_id_style),
                Span::styled(change_id_short.to_string(), change_id_style),
                Span::raw(" "),
                Span::styled(row.author.clone(), self.theme.author),
                Span::raw(" "),
                Span::styled(row.timestamp.clone(), self.theme.timestamp),
                Span::raw(" "),
            ];

            // Add bookmarks if any
            for bookmark in &row.bookmarks {
                line_1_details.push(Span::styled(bookmark.clone(), self.theme.bookmark));
                line_1_details.push(Span::raw(" "));
            }

            line_1_details.push(Span::styled(
                commit_id_short.to_string(),
                self.theme.commit_id_dim,
            ));
            detail_lines.push(Line::from(line_1_details));

            // Line 2: Description
            let description = row.description.lines().next().unwrap_or("");
            if description.is_empty() {
                detail_lines.push(Line::from(Span::styled(
                    "(no description set)",
                    self.theme.timestamp,
                )));
            } else {
                detail_lines.push(Line::from(Span::raw(description.to_string())));
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
                    detail_lines.push(Line::from(Span::styled(
                        format!("{}{}", prefix, file.path),
                        style,
                    )));
                }
            }

            rows.push(
                Row::new(vec![Cell::from(graph_lines), Cell::from(detail_lines)])
                    .height(row_height)
                    .style(Style::default()),
            );
        }

        let table = Table::new(rows, [Constraint::Length(16), Constraint::Min(0)])
            .row_highlight_style(self.theme.highlight)
            .highlight_symbol(" ");

        StatefulWidget::render(table, area, buf, state);
    }
}
