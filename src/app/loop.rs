use crate::app::{
    action::Action, command::Command, input::map_event_to_action, reducer, state::AppState, ui,
};
use crate::domain::vcs::VcsFacade;

use anyhow::Result;
use crossterm::event::{self, Event, MouseButton, MouseEventKind};
use notify::{RecursiveMode, Watcher};
use ratatui::{backend::Backend, Terminal};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;

const TICK_RATE: Duration = Duration::from_millis(250);

pub async fn run_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: AppState<'_>,
    adapter: Arc<dyn VcsFacade>,
) -> Result<()> {
    // User input channel
    let (event_tx, event_rx) = mpsc::channel(100);
    tokio::task::spawn_blocking(move || loop {
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
    });

    run_loop_with_events(terminal, app_state, adapter, event_rx).await
}

pub async fn run_loop_with_events<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app_state: AppState<'_>,
    adapter: Arc<dyn VcsFacade>,
    mut event_rx: mpsc::Receiver<Result<Event, std::io::Error>>,
) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::channel(100);
    let mut interval = interval(TICK_RATE);

    // Repository Watcher
    let (notify_tx, mut notify_rx) = mpsc::channel(1);
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if res.is_ok() {
            let _ = notify_tx.try_send(());
        }
    })?;

    let repo_path = adapter.workspace_root();
    let op_heads_path = repo_path.join(".jj").join("repo").join("op_heads");
    if op_heads_path.exists() {
        watcher.watch(&op_heads_path, RecursiveMode::NonRecursive)?;
    }

    let action_tx_clone = action_tx.clone();
    tokio::spawn(async move {
        let mut pending = false;
        let debounce_duration = Duration::from_millis(500);

        loop {
            if pending {
                tokio::select! {
                    Some(()) = notify_rx.recv() => {}
                    () = tokio::time::sleep(debounce_duration) => {
                        let _ = action_tx_clone.send(Action::ExternalChangeDetected).await;
                        pending = false;
                    }
                }
            } else if notify_rx.recv().await.is_some() {
                pending = true;
            } else {
                break;
            }
        }
    });

    // Initial Load
    if app_state.mode != crate::app::state::AppMode::NoRepo {
        handle_command(
            Command::LoadRepo(None, 100, None),
            adapter.clone(),
            action_tx.clone(),
        )?;
    }

    loop {
        // --- 1. Render ---
        terminal.draw(|f| {
            ui::draw(f, &mut app_state);
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
                let action = map_event_to_action(event.clone(), &app_state, terminal.size()?);
                if let Event::Mouse(mouse) = event {
                    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                        app_state.last_click_time = Some(Instant::now());
                        app_state.last_click_pos = Some((mouse.column, mouse.row));
                    }
                }
                action
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
                match cmd {
                    Command::ResolveConflict(path) => {
                        // SECURITY: Validate path to prevent path traversal
                        use std::path::Component;
                        if std::path::Path::new(&path)
                            .components()
                            .any(|c| matches!(c, Component::ParentDir))
                        {
                            let _ = action_tx
                                .send(Action::OperationCompleted(Err(format!(
                                    "Invalid path: {path}"
                                ))))
                                .await;
                            continue;
                        }

                        let success = crate::app::external::run_external_command(
                            terminal,
                            "jj",
                            &["resolve", &path],
                        )?;

                        // Trigger refresh
                        let _ = action_tx
                            .send(Action::OperationCompleted(if success {
                                Ok(format!("Resolved {path}"))
                            } else {
                                Err(format!("Resolve failed for {path}"))
                            }))
                            .await;
                    }
                    Command::Split(commit_id) => {
                        let success = crate::app::external::run_external_command(
                            terminal,
                            "jj",
                            &["split", "-r", &commit_id.0],
                        )?;

                        // Trigger refresh
                        let _ = action_tx
                            .send(Action::OperationCompleted(if success {
                                Ok(format!("Split revision {}", commit_id.0))
                            } else {
                                Err(format!("Split failed for revision {}", commit_id.0))
                            }))
                            .await;
                    }
                    other_cmd => {
                        handle_command(other_cmd, adapter.clone(), action_tx.clone())?;
                    }
                }
            }
        }
    }

    Ok(())
}

pub(crate) fn handle_command(
    command: Command,
    adapter: Arc<dyn VcsFacade>,
    tx: mpsc::Sender<Action>,
) -> Result<()> {
    crate::app::features::vcs::handle_command(command, adapter, tx)
}

#[cfg(test)]
#[path = "loop_tests.rs"]
mod tests;
