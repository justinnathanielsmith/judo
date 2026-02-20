use crate::domain::models::RepoStatus;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, StatefulWidget, Table, TableState},
};

pub struct RevisionGraph<'a> {
    pub repo: &'a RepoStatus,
}

impl<'a> StatefulWidget for RevisionGraph<'a> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TableState) {
        let mut lanes: Vec<Option<String>> = Vec::new();
        let mut rows: Vec<Row> = Vec::new();

        for (_i, row) in self.repo.graph.iter().enumerate() {
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

            // Prepare graph column representation
            let mut graph_col = String::new();
            for (lane_idx, lane_commit) in lanes.iter().enumerate() {
                if lane_idx == current_lane {
                    let symbol = if row.is_working_copy {
                        "@"
                    } else if row.is_immutable {
                        "o"
                    } else {
                        "*"
                    };
                    graph_col.push_str(symbol);
                } else if lane_commit.is_some() {
                    graph_col.push('|');
                } else {
                    graph_col.push(' ');
                }
            }

            // Update lanes for children (next rows)
            // Remove current commit from its lane
            lanes[current_lane] = None;
            // Add parents to lanes (if not already there)
            for parent in &row.parents {
                if !lanes.iter().any(|l| l.as_ref() == Some(&parent.0)) {
                    if let Some(pos) = lanes.iter().position(|l| l.is_none()) {
                        lanes[pos] = Some(parent.0.clone());
                    } else {
                        lanes.push(Some(parent.0.clone()));
                    }
                }
            }

            let is_wc = row.is_working_copy;
            let style = if is_wc {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if row.is_immutable {
                Style::default().fg(Color::Blue)
            } else {
                Style::default().fg(Color::Magenta)
            };

            let change_id_style = Style::default().fg(Color::Cyan);

            rows.push(
                Row::new(vec![
                    Cell::from(graph_col).style(Style::default().fg(Color::White)),
                    Cell::from(
                        row.change_id
                            .get(0..8)
                            .unwrap_or(&row.change_id)
                            .to_string(),
                    )
                    .style(change_id_style),
                    Cell::from(row.description.lines().next().unwrap_or("").to_string()),
                    Cell::from(row.author.clone()),
                ])
                .style(style),
            );
        }

        let table = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Min(20),
                Constraint::Length(15),
            ],
        )
        .block(
            Block::default()
                .title("Revision Graph")
                .borders(Borders::ALL),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 40)) // Subtle highlight
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

        StatefulWidget::render(table, area, buf, state);
    }
}
