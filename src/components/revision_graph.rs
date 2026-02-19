use crate::domain::models::RepoStatus;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, StatefulWidget, Table, TableState, Widget},
};

pub struct RevisionGraph<'a> {
    pub repo: &'a RepoStatus,
}

impl<'a> StatefulWidget for RevisionGraph<'a> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let rows: Vec<Row> = self
            .repo
            .graph
            .iter()
            .map(|row| {
                let is_wc = row.commit_id == self.repo.working_copy_id;
                let style = if is_wc {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(
                        row.change_id
                            .get(0..8)
                            .unwrap_or(&row.change_id)
                            .to_string(),
                    ),
                    Cell::from(row.description.clone()),
                    Cell::from(row.author.clone()),
                ])
                .style(style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Min(20),
                Constraint::Length(15),
            ],
        )
        .block(Block::default().title("Graph").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

        StatefulWidget::render(table, area, buf, state);
    }
}
