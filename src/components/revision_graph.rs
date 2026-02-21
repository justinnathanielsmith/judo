use crate::app::ui;
use crate::domain::models::{FileStatus, RepoStatus};
use crate::theme::{glyphs, Theme};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Cell, Row, StatefulWidget, Table, TableState},
};

pub struct RevisionGraph<'a> {
    pub repo: &'a RepoStatus,
    pub theme: &'a Theme,
    pub show_diffs: bool,
    pub selected_file_index: Option<usize>,
}

impl<'a> StatefulWidget for RevisionGraph<'a> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TableState) {
        let mut rows: Vec<Row> = Vec::new();

        for (i, row) in self.repo.graph.iter().enumerate() {
            let is_selected = state.selected() == Some(i);
            let row_height = ui::calculate_row_height(row, is_selected, self.show_diffs);

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
                    line_1_graph.push(Span::styled(symbol, style));
                } else if row
                    .visual
                    .active_lanes
                    .get(lane_idx)
                    .cloned()
                    .unwrap_or(false)
                {
                    line_1_graph.push(Span::styled("│", lane_style));
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

                    connector_line.push(Span::styled(symbol, lane_style));
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
