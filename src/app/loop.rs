use crate::app::{action::Action, reducer, state::AppState};
use crate::components::diff_view::DiffView;
use crate::components::revision_graph::RevisionGraph;
use crate::domain::vcs::VcsFacade;
use crate::infrastructure::jj_adapter::JjAdapter;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

pub async fn run_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app_state: AppState<'_>,
) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::channel(100);
    let mut interval = interval(Duration::from_millis(250));

    // Initialize adapter
    // let _adapter = JjAdapter::new(); // In real app, maybe shared via Arc

    // Initial fetch
    let tx = action_tx.clone();
    // Move adapter creation to the async block or Arc it.
    // Since JjAdapter is not Clone/Send/Sync by default (depends on fields),
    // we'll try initializing it inside the task for this MVP.
    tokio::spawn(async move {
        // We use a fresh adapter here for the background task
        match JjAdapter::new() {
            Ok(adapter) => {
                match adapter.get_operation_log().await {
                    Ok(repo) => {
                        let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                    }
                    Err(e) => {
                        // Send error action? For now just log/ignore or print
                        eprintln!("Failed to load repo log: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to init adapter in bg: {}", e);
            }
        }
    });

    loop {
        // --- 1. Render ---
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(f.area());

            // Left: Revision Graph
            if let Some(repo) = &app_state.repo {
                let graph = RevisionGraph { repo };
                f.render_stateful_widget(graph, chunks[0], &mut app_state.log_list_state);
            } else {
                let loading = Paragraph::new("Loading repo...")
                    .block(Block::default().title("Graph").borders(Borders::ALL));
                f.render_widget(loading, chunks[0]);
            }

            // Right: Diff View
            let diff_view = DiffView {
                diff_content: app_state.current_diff.as_deref(),
            };
            let right_block = Block::default().title("Diff").borders(Borders::ALL);
            let inner_area = right_block.inner(chunks[1]);
            f.render_widget(right_block, chunks[1]);
            f.render_widget(diff_view, inner_area);

            // Status Bar (very simple overlay for now)
            if let Some(msg) = &app_state.status_message {
                let status = Paragraph::new(msg.as_str());
                let area = Layout::default()
                    .constraints([Constraint::Min(0), Constraint::Length(1)])
                    .split(f.area())[1];
                f.render_widget(status, area);
            }
        })?;

        // --- 2. Event Handling (TEA Runtime) ---
        let action = tokio::select! {
            _ = interval.tick() => Some(Action::Tick),

            // User Input
            event = async { event::read().unwrap() } => {
                match event {
                    Event::Key(key) => {
                        match key.code {
                            KeyCode::Char('q') => Some(Action::Quit),
                            KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNext),
                            KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrev),
                            // ... other mappings
                            _ => None,
                        }
                    },
                    _ => None,
                }
            },

            // Async Results
            Some(a) = action_rx.recv() => Some(a),
        };

        // --- 3. Update (Reducer) ---
        if let Some(action) = action {
            if let Action::Quit = action {
                break;
            }

            // Side-effect detectors
            let prev_selection = app_state.log_list_state.selected();

            // Run reducer
            reducer::update(&mut app_state, action.clone());

            // Post-reducer side effects (Runtime logic)
            if app_state.should_quit {
                break;
            }

            // Example: Selection changed -> fetch diff
            if app_state.log_list_state.selected() != prev_selection && app_state.is_loading_diff {
                if let (Some(repo), Some(idx)) =
                    (&app_state.repo, app_state.log_list_state.selected())
                {
                    if let Some(row) = repo.graph.get(idx) {
                        let commit_id = row.commit_id.clone();
                        let tx = action_tx.clone();
                        tokio::spawn(async move {
                            match JjAdapter::new() {
                                Ok(adapter) => match adapter.get_commit_diff(&commit_id).await {
                                    Ok(diff) => {
                                        let _ = tx.send(Action::DiffLoaded(diff)).await;
                                    }
                                    Err(e) => {
                                        let _ = tx
                                            .send(Action::DiffLoaded(format!("Error: {}", e)))
                                            .await;
                                    }
                                },
                                Err(e) => {
                                    let _ = tx
                                        .send(Action::DiffLoaded(format!("Adapter Error: {}", e)))
                                        .await;
                                }
                            }
                        });
                    }
                }
            }
        }
    }

    Ok(())
}
