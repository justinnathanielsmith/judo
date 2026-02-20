use crate::domain::models::{FileStatus, RepoStatus};
use crate::theme::Theme;
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
}

impl<'a> StatefulWidget for RevisionGraph<'a> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TableState) {
        let mut lanes: Vec<Option<String>> = Vec::new();
        let mut rows: Vec<Row> = Vec::new();

        for (i, row) in self.repo.graph.iter().enumerate() {
            let is_selected = state.selected() == Some(i);

            // Find or assign a lane for this commit
            let commit_id_hex = &row.commit_id.0;
            let current_lane = lanes
                .iter()
                .position(|l| l.as_ref() == Some(commit_id_hex))
                .unwrap_or_else(|| {
                    // If not found, find an empty spot or push a new one
                    if let Some(pos) = lanes.iter().position(|l| l.is_none()) {
                        lanes[pos] = Some(commit_id_hex.clone());
                        pos
                    } else {
                        lanes.push(Some(commit_id_hex.clone()));
                        lanes.len() - 1
                    }
                });

            let show_files = is_selected && self.show_diffs;
            let num_files = if show_files {
                row.changed_files.len()
            } else {
                0
            };
            let row_height = 2 + num_files as u16;

            // Prepare Graph Column
            let mut graph_lines = Vec::new();

            // Graph Line 1: Node symbol and existing pipes
            let mut line_1_graph = Vec::new();
            for (lane_idx, lane_commit) in lanes.iter().enumerate() {
                if lane_idx == current_lane {
                    let (symbol, style) = if row.is_working_copy {
                        ("@", self.theme.graph_node_wc)
                    } else if row.is_immutable {
                        ("◆", self.theme.graph_node_immutable)
                    } else {
                        ("○", self.theme.graph_node_mutable)
                    };
                    line_1_graph.push(Span::styled(symbol, style));
                } else if lane_commit.is_some() {
                    line_1_graph.push(Span::styled("│", self.theme.graph_line));
                } else {
                    line_1_graph.push(Span::raw(" "));
                }
            }
            // Add spacing after graph for the "flow" look
            line_1_graph.push(Span::raw("  "));
            graph_lines.push(Line::from(line_1_graph));

            // Update lanes for parents
            lanes[current_lane] = None;
            for parent in &row.parents {
                if !lanes.iter().any(|l| l.as_ref() == Some(&parent.0)) {
                    if let Some(pos) = lanes.iter().position(|l| l.is_none()) {
                        lanes[pos] = Some(parent.0.clone());
                    } else {
                        lanes.push(Some(parent.0.clone()));
                    }
                }
            }

            // Subsequent Graph Lines: Connector pipes
            for _ in 1..row_height {
                let mut connector_line = Vec::new();
                for lane_commit in lanes.iter() {
                    if lane_commit.is_some() {
                        connector_line.push(Span::styled("│", self.theme.graph_line));
                    } else {
                        connector_line.push(Span::raw(" "));
                    }
                }
                connector_line.push(Span::raw("  "));
                graph_lines.push(Line::from(connector_line));
            }

            // Prepare Details Column
            let mut detail_lines = Vec::new();

            // Line 1: ChangeId Author Timestamp CommitId
            let change_id_short = row.change_id.get(0..8).unwrap_or(&row.change_id);
            let commit_id_short = row.commit_id.0.get(0..8).unwrap_or(&row.commit_id.0);

            let change_id_style = if row.is_immutable {
                self.theme.change_id_immutable
            } else {
                self.theme.change_id_mutable
            };

            let mut line_1_details = vec![
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
            if show_files {
                for file in &row.changed_files {
                    let style = match file.status {
                        FileStatus::Added => self.theme.diff_add,
                        FileStatus::Modified => self.theme.diff_modify,
                        FileStatus::Deleted => self.theme.diff_remove,
                    };
                    detail_lines.push(Line::from(Span::styled(format!("{}", file.path), style)));
                }
            }

            rows.push(
                Row::new(vec![Cell::from(graph_lines), Cell::from(detail_lines)])
                    .height(row_height)
                    .style(Style::default()),
            );
        }

        let table = Table::new(rows, [Constraint::Length(12), Constraint::Min(0)])
            .row_highlight_style(self.theme.highlight)
            .highlight_symbol(">> ");

        StatefulWidget::render(table, area, buf, state);
    }
}
