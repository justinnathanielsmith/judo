use crate::app::ui;
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

            // Graph Line 1: Node symbol and existing pipes
            let mut line_1_graph = Vec::new();
            for (lane_idx, is_active) in row.visual.active_lanes.iter().enumerate() {
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
                } else if *is_active {
                    line_1_graph.push(Span::styled("│", lane_style));
                } else {
                    line_1_graph.push(Span::raw(" "));
                }
            }
            // Add spacing after graph for the "flow" look
            line_1_graph.push(Span::raw("  "));
            graph_lines.push(Line::from(line_1_graph));

            // Subsequent Graph Lines: Connector pipes
            for _ in 1..row_height {
                let mut connector_line = Vec::new();
                for (lane_idx, is_active) in row.visual.connector_lanes.iter().enumerate() {
                    let lane_style =
                        self.theme.graph_lanes[lane_idx % self.theme.graph_lanes.len()];
                    if *is_active {
                        connector_line.push(Span::styled("│", lane_style));
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
            .highlight_symbol(">> ");

        StatefulWidget::render(table, area, buf, state);
    }
}
