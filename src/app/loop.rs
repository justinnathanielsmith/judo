use crate::app::{action::Action, command::Command, reducer, state::AppState};
use crate::components::diff_view::DiffView;
use crate::components::revision_graph::RevisionGraph;
use crate::domain::vcs::VcsFacade;

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

use std::sync::Arc;

pub async fn run_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app_state: AppState<'_>,
    adapter: Arc<dyn VcsFacade>,
) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::channel(100);
    let mut interval = interval(Duration::from_millis(250));

    // User input channel
    let (event_tx, mut event_rx) = mpsc::channel(100);
    tokio::task::spawn_blocking(move || {
        loop {
            match event::read() {
                Ok(evt) => {
                    if event_tx.blocking_send(Ok(evt)).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = event_tx.blocking_send(Err(e));
                    break;
                }
            }
        }
    });

    // Initial fetch
    handle_command(Command::LoadRepo, adapter.clone(), action_tx.clone()).await?;

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
                scroll_offset: app_state.diff_scroll,
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

            // Error Bar
            if let Some(err) = &app_state.last_error {
                let error_bar = Paragraph::new(format!("Error: {}", err))
                    .style(ratatui::style::Style::default().fg(ratatui::style::Color::Red));
                let area = Layout::default()
                    .constraints([Constraint::Min(0), Constraint::Length(1)])
                    .split(f.area())[1];
                f.render_widget(error_bar, area);
            }

            // Input Modal
            if app_state.mode == crate::app::state::AppMode::Input {
                use ratatui::widgets::Clear;
                let area = centered_rect(60, 20, f.area());
                f.render_widget(Clear, area);
                f.render_widget(&app_state.text_area, area);
            }
        })?;

        // --- 2. Event Handling (TEA Runtime) ---
        let action = tokio::select! {
            _ = interval.tick() => Some(Action::Tick),

            // User Input
            Some(res) = event_rx.recv() => {
                let event = match res {
                    Ok(e) => e,
                    Err(e) => return Err(e.into()),
                };
                match app_state.mode {
                    crate::app::state::AppMode::Input => {
                        match event {
                            Event::Key(key) => {
                                match key.code {
                                    KeyCode::Esc => Some(Action::CancelMode),
                                    KeyCode::Enter => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            if let Some(row) = repo.graph.get(idx) {
                                                Some(Action::DescribeRevision(row.commit_id.clone(), app_state.text_area.lines().join("\n")))
                                            } else { None }
                                        } else { None }
                                    },
                                    _ => {
                                        app_state.text_area.input(key);
                                        None
                                    }
                                }
                            },
                            _ => None,
                        }
                    },
                    _ => {
                        match event {
                            Event::Key(key) => {
                                match key.code {
                                    KeyCode::Char('q') => Some(Action::Quit),
                                    KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNext),
                                    KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrev),
                                    KeyCode::Char('s') => Some(Action::SnapshotWorkingCopy),
                                    KeyCode::Char('e') => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            if let Some(row) = repo.graph.get(idx) {
                                                Some(Action::EditRevision(row.commit_id.clone()))
                                            } else { None }
                                        } else { None }
                                    },
                                    KeyCode::Char('n') => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            if let Some(row) = repo.graph.get(idx) {
                                                Some(Action::NewRevision(row.commit_id.clone()))
                                            } else { None }
                                        } else { None }
                                    },
                                    KeyCode::Char('a') => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            if let Some(row) = repo.graph.get(idx) {
                                                Some(Action::AbandonRevision(row.commit_id.clone()))
                                            } else { None }
                                        } else { None }
                                    },
                                    KeyCode::Char('d') => Some(Action::DescribeRevisionIntent),
                                    KeyCode::Char('u') => Some(Action::Undo),
                                    KeyCode::Char('U') => Some(Action::Redo),
                                    KeyCode::PageDown => Some(Action::ScrollDiffDown(10)),
                                    KeyCode::PageUp => Some(Action::ScrollDiffUp(10)),
                                    KeyCode::Char('[') => Some(Action::PrevHunk),
                                    KeyCode::Char(']') => Some(Action::NextHunk),
                                    _ => None,
                                }
                            },
                            _ => None,
                        }
                    }
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

            // Run reducer
            let command = reducer::update(&mut app_state, action.clone());

            // Post-reducer side effects (Runtime logic)
            if app_state.should_quit {
                break;
            }

            if let Some(cmd) = command {
                handle_command(cmd, adapter.clone(), action_tx.clone()).await?;
            }
        }
    }

    Ok(())
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

async fn handle_command(
    command: Command,
    adapter: Arc<dyn VcsFacade>,
    tx: mpsc::Sender<Action>,
) -> Result<()> {
    match command {
        Command::LoadRepo => {
            tokio::spawn(async move {
                match adapter.get_operation_log().await {
                    Ok(repo) => {
                        let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ErrorOccurred(format!("Failed to load repo: {}", e))).await;
                    }
                }
            });
        }
        Command::LoadDiff(commit_id) => {
            let commit_id_clone = commit_id.clone();
            tokio::spawn(async move {
                match adapter.get_commit_diff(&commit_id).await {
                    Ok(diff) => {
                        let _ = tx.send(Action::DiffLoaded(commit_id_clone, diff)).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::DiffLoaded(commit_id_clone, format!("Error: {}", e)))
                            .await;
                    }
                }
            });
        }
        Command::DescribeRevision(commit_id, message) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Describing {}...",
                        commit_id
                    )))
                    .await;
                match adapter.describe_revision(&commit_id.0, &message).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok("Described".to_string())))
                            .await;
                        // Reload repo after operation
                        if let Ok(repo) = adapter.get_operation_log().await {
                             let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Snapshot => {
            tokio::spawn(async move {
                let _ = tx.send(Action::OperationStarted("Snapshotting...".to_string())).await;
                match adapter.snapshot().await {
                    Ok(msg) => {
                        let _ = tx.send(Action::OperationCompleted(Ok(msg))).await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Action::OperationCompleted(Err(format!("Error: {}", e)))).await;
                    }
                }
            });
        }
        Command::Edit(commit_id) => {
            tokio::spawn(async move {
                let _ = tx.send(Action::OperationStarted(format!("Editing {}...", commit_id))).await;
                match adapter.edit(&commit_id).await {
                    Ok(_) => {
                        let _ = tx.send(Action::OperationCompleted(Ok("Edit successful".to_string()))).await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Action::OperationCompleted(Err(format!("Error: {}", e)))).await;
                    }
                }
            });
        }
        Command::Squash(commit_id) => {
            tokio::spawn(async move {
                let _ = tx.send(Action::OperationStarted(format!("Squashing {}...", commit_id))).await;
                match adapter.squash(&commit_id).await {
                    Ok(_) => {
                        let _ = tx.send(Action::OperationCompleted(Ok("Squash successful".to_string()))).await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Action::OperationCompleted(Err(format!("Error: {}", e)))).await;
                    }
                }
            });
        }
        Command::New(commit_id) => {
            tokio::spawn(async move {
                let _ = tx.send(Action::OperationStarted(format!("Creating child of {}...", commit_id))).await;
                match adapter.new_child(&commit_id).await {
                    Ok(_) => {
                        let _ = tx.send(Action::OperationCompleted(Ok("New revision created".to_string()))).await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Action::OperationCompleted(Err(format!("Error: {}", e)))).await;
                    }
                }
            });
        }
        Command::Abandon(commit_id) => {
            tokio::spawn(async move {
                let _ = tx.send(Action::OperationStarted(format!("Abandoning {}...", commit_id))).await;
                match adapter.abandon(&commit_id).await {
                    Ok(_) => {
                        let _ = tx.send(Action::OperationCompleted(Ok("Revision abandoned".to_string()))).await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Action::OperationCompleted(Err(format!("Error: {}", e)))).await;
                    }
                }
            });
        }
        Command::Undo => {
            tokio::spawn(async move {
                let _ = tx.send(Action::OperationStarted("Undoing...".to_string())).await;
                match adapter.undo().await {
                    Ok(_) => {
                        let _ = tx.send(Action::OperationCompleted(Ok("Undo successful".to_string()))).await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Action::OperationCompleted(Err(format!("Error: {}", e)))).await;
                    }
                }
            });
        }
        Command::Redo => {
            tokio::spawn(async move {
                let _ = tx.send(Action::OperationStarted("Redoing...".to_string())).await;
                match adapter.redo().await {
                    Ok(_) => {
                        let _ = tx.send(Action::OperationCompleted(Ok("Redo successful".to_string()))).await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Action::OperationCompleted(Err(format!("Error: {}", e)))).await;
                    }
                }
            });
        }
    }
    Ok(())
}


