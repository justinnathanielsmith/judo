use crate::app::{action::Action, command::Command, reducer, state::AppState, ui};
use crate::components::revision_graph::calculate_row_height;
use crate::domain::vcs::VcsFacade;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, MouseButton, MouseEventKind};
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
                map_event_to_action(event, &app_state, terminal.size()?)
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
                        // 1. Suspend TUI
                        crossterm::terminal::disable_raw_mode()?;
                        crossterm::execute!(
                            std::io::stdout(),
                            crossterm::terminal::LeaveAlternateScreen,
                            crossterm::cursor::Show
                        )?;

                        // 2. Run external tool
                        // We'll use 'jj resolve' which uses the configured tool
                        // SECURITY: Validate path to prevent path traversal
                        use std::path::Component;
                        if std::path::Path::new(&path)
                            .components()
                            .any(|c| matches!(c, Component::ParentDir))
                        {
                            crossterm::terminal::enable_raw_mode()?;
                            crossterm::execute!(
                                std::io::stdout(),
                                crossterm::terminal::EnterAlternateScreen,
                                crossterm::cursor::Hide
                            )?;
                            terminal.clear()?;
                            let _ = action_tx
                                .send(Action::OperationCompleted(Err(format!(
                                    "Invalid path: {path}"
                                ))))
                                .await;
                            continue;
                        }

                        let mut child = std::process::Command::new("jj")
                            .arg("resolve")
                            .arg(&path)
                            .spawn()?;

                        let status = child.wait()?;

                        // 3. Resume TUI
                        crossterm::terminal::enable_raw_mode()?;
                        crossterm::execute!(
                            std::io::stdout(),
                            crossterm::terminal::EnterAlternateScreen,
                            crossterm::cursor::Hide
                        )?;
                        terminal.clear()?;

                        // 4. Trigger refresh
                        let _ = action_tx
                            .send(Action::OperationCompleted(if status.success() {
                                Ok(format!("Resolved {path}"))
                            } else {
                                Err(format!("Resolve failed for {path}"))
                            }))
                            .await;
                    }
                    Command::Split(commit_id) => {
                        // 1. Suspend TUI
                        crossterm::terminal::disable_raw_mode()?;
                        crossterm::execute!(
                            std::io::stdout(),
                            crossterm::terminal::LeaveAlternateScreen,
                            crossterm::cursor::Show
                        )?;

                        let mut child = std::process::Command::new("jj")
                            .arg("split")
                            .arg("-r")
                            .arg(&commit_id.0)
                            .spawn()?;

                        let status = child.wait()?;

                        // 3. Resume TUI
                        crossterm::terminal::enable_raw_mode()?;
                        crossterm::execute!(
                            std::io::stdout(),
                            crossterm::terminal::EnterAlternateScreen,
                            crossterm::cursor::Hide
                        )?;
                        terminal.clear()?;

                        // 4. Trigger refresh
                        let _ = action_tx
                            .send(Action::OperationCompleted(if status.success() {
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

fn resolve_clicked_row(
    app_state: &AppState<'_>,
    clicked_row: usize,
) -> Option<(usize, Option<usize>)> {
    let offset = app_state.log.list_state.offset();
    let repo = app_state.repo.as_ref()?;
    
    let mut current_y = 0;
    for i in offset..repo.graph.len() {
        let row = &repo.graph[i];
        let is_selected = app_state.log.list_state.selected() == Some(i);
        let row_height = calculate_row_height(row, is_selected, app_state.show_diffs) as usize;

        if clicked_row >= current_y && clicked_row < current_y + row_height {
            let file_idx = if is_selected && app_state.show_diffs && clicked_row >= current_y + 2 {
                Some(clicked_row - (current_y + 2))
            } else {
                None
            };
            return Some((i, file_idx));
        }
        current_y += row_height;
        // Optimization: if we're past the possible clicked area we could break
        // but we don't have the graph_area.height here, so we just iterate.
    }
    None
}

pub fn map_event_to_action(
    event: Event,
    app_state: &AppState<'_>,
    terminal_size: ratatui::layout::Size,
) -> Option<Action> {
    match app_state.mode {
        crate::app::state::AppMode::Input 
        | crate::app::state::AppMode::BookmarkInput 
        | crate::app::state::AppMode::CommitInput => {
            match event {
                Event::Key(key) => match key.code {
                    KeyCode::Esc => Some(Action::CancelMode),
                    KeyCode::Enter => {
                        if app_state.mode == crate::app::state::AppMode::CommitInput {
                            if let Some(input) = &app_state.input {
                                return Some(Action::CommitWorkingCopy(
                                    input.text_area.lines().join("\n"),
                                ));
                            }
                            return None;
                        }
                        
                        if let (Some(repo), Some(idx), Some(input)) = (
                            &app_state.repo,
                            app_state.log.list_state.selected(),
                            &app_state.input,
                        ) {
                            if let Some(row) = repo.graph.get(idx) {
                                if app_state.mode == crate::app::state::AppMode::BookmarkInput {
                                    let name = input.text_area.lines().join("").trim().to_string();
                                    if name.is_empty() {
                                        return None;
                                    }
                                    Some(Action::SetBookmark(row.commit_id.clone(), name))
                                } else {
                                    Some(Action::DescribeRevision(
                                        row.commit_id.clone(),
                                        input.text_area.lines().join("\n"),
                                    ))
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    _ => Some(Action::TextAreaInput(key)),
                },
                _ => None,
            }
        }
        crate::app::state::AppMode::FilterInput => match event {
            Event::Key(key) => {
                if let Some(action) = app_state.keymap.get_action(key, app_state) {
                    return Some(action);
                }
                Some(Action::TextAreaInput(key))
            }
            _ => None,
        },
        crate::app::state::AppMode::ContextMenu => {
            match event {
                Event::Key(key) => match key.code {
                    KeyCode::Esc => Some(Action::CloseContextMenu),
                    KeyCode::Char('j') | KeyCode::Down => Some(Action::SelectContextMenuNext),
                    KeyCode::Char('k') | KeyCode::Up => Some(Action::SelectContextMenuPrev),
                    KeyCode::Enter => app_state
                        .context_menu
                        .as_ref()
                        .map(|menu| Action::SelectContextMenuAction(menu.selected_index)),
                    _ => None,
                },
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if let Some(menu) = &app_state.context_menu {
                            let area = ratatui::layout::Rect::new(
                                0,
                                0,
                                terminal_size.width,
                                terminal_size.height,
                            );
                            let menu_area = menu.calculate_rect(area);

                            if mouse.column >= menu_area.x
                                && mouse.column < menu_area.x + menu_area.width
                                && mouse.row >= menu_area.y
                                && mouse.row < menu_area.y + menu_area.height
                            {
                                // Adjust for borders: top/left border is at y/x, so content starts at y+1
                                let clicked_idx =
                                    (mouse.row.saturating_sub(menu_area.y + 1)) as usize;
                                Some(Action::SelectContextMenuAction(clicked_idx))
                            } else {
                                Some(Action::CloseContextMenu)
                            }
                        } else {
                            Some(Action::CloseContextMenu)
                        }
                    }
                    _ => None,
                },
                _ => None,
            }
        }
        crate::app::state::AppMode::NoRepo => match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
                KeyCode::Char('i') | KeyCode::Enter => Some(Action::InitRepo),
                _ => None,
            },
            _ => None,
        },
        crate::app::state::AppMode::Loading => None,
        crate::app::state::AppMode::Help => match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q' | '?') => Some(Action::ToggleHelp),
                _ => None,
            },
            _ => None,
        },
        crate::app::state::AppMode::Diff => {
            match event {
                Event::Key(key) => {
                    if let Some(action) = app_state.keymap.get_action(key, app_state) {
                        return Some(action);
                    }
                    match key.code {
                        KeyCode::Char('m') | KeyCode::Enter => {
                            if let (Some(repo), Some(idx)) =
                                (&app_state.repo, app_state.log.list_state.selected())
                            {
                                if let Some(row) = repo.graph.get(idx) {
                                    if let Some(file_idx) = app_state.log.selected_file_index {
                                        if let Some(file) = row.changed_files.get(file_idx) {
                                            if file.status
                                                == crate::domain::models::FileStatus::Conflicted
                                            {
                                                return Some(Action::ResolveConflict(
                                                    file.path.clone(),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            None
                        }
                        _ => None,
                    }
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => Some(Action::ScrollDiffUp(1)),
                    MouseEventKind::ScrollDown => Some(Action::ScrollDiffDown(1)),
                    MouseEventKind::Down(MouseButton::Left) => {
                        let now = Instant::now();
                        let is_double_click = app_state
                            .last_click_time
                            .is_some_and(|t| now.duration_since(t).as_millis() < 500)
                            && app_state.last_click_pos == Some((mouse.column, mouse.row));

                        let area = ratatui::layout::Rect::new(
                            0,
                            0,
                            terminal_size.width,
                            terminal_size.height,
                        );
                        let layout = ui::get_layout(area, app_state);

                        // Revision Graph Area
                        let graph_area = layout.body[0];
                        if mouse.column > graph_area.x
                            && mouse.column < graph_area.x + graph_area.width - 1
                            && mouse.row > graph_area.y
                            && mouse.row < graph_area.y + graph_area.height - 1
                        {
                            if is_double_click {
                                Some(Action::ToggleDiffs)
                            } else {
                                let clicked_row = (mouse.row - (graph_area.y + 1)) as usize;
                                if let Some((i, file_idx)) = resolve_clicked_row(app_state, clicked_row) {
                                    if let Some(idx) = file_idx {
                                        Some(Action::SelectFile(idx))
                                    } else {
                                        Some(Action::SelectIndex(i))
                                    }
                                } else {
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
                _ => None,
            }
        }
        _ => match event {
            Event::Resize(w, h) => Some(Action::Resize(w, h)),
            Event::Key(key) => {
                if let Some(action) = app_state.keymap.get_action(key, app_state) {
                    return Some(action);
                }
                None
            }
            Event::Mouse(mouse) => {
                let area =
                    ratatui::layout::Rect::new(0, 0, terminal_size.width, terminal_size.height);
                let layout = ui::get_layout(area, app_state);

                let graph_area = layout.body[0];
                let diff_area = layout.body[1];

                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        if app_state.show_diffs
                            && mouse.column >= diff_area.x
                            && mouse.column < diff_area.x + diff_area.width
                        {
                            Some(Action::ScrollDiffUp(1))
                        } else {
                            Some(Action::SelectPrev)
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if app_state.show_diffs
                            && mouse.column >= diff_area.x
                            && mouse.column < diff_area.x + diff_area.width
                        {
                            Some(Action::ScrollDiffDown(1))
                        } else {
                            Some(Action::SelectNext)
                        }
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        let now = Instant::now();
                        let is_double_click = app_state
                            .last_click_time
                            .is_some_and(|t| now.duration_since(t).as_millis() < 500)
                            && app_state.last_click_pos == Some((mouse.column, mouse.row));

                        // Double click anywhere toggles the diff panel
                        if is_double_click {
                            Some(Action::ToggleDiffs)
                        } else if mouse.column > graph_area.x
                            && mouse.column < graph_area.x + graph_area.width - 1
                            && mouse.row > graph_area.y
                            && mouse.row < graph_area.y + graph_area.height - 1
                        {
                            let clicked_row = (mouse.row - (graph_area.y + 1)) as usize;
                            if let Some((i, file_idx)) = resolve_clicked_row(app_state, clicked_row) {
                                if let Some(idx) = file_idx {
                                    Some(Action::SelectFile(idx))
                                } else {
                                    Some(Action::SelectIndex(i))
                                }
                            } else {
                                None
                            }
                        } else if app_state.show_diffs
                            && mouse.column >= diff_area.x
                            && mouse.column < diff_area.x + diff_area.width
                        {
                            Some(Action::FocusDiff)
                        } else {
                            None
                        }
                    }
                    MouseEventKind::Down(MouseButton::Right) => {
                        if mouse.column > graph_area.x
                            && mouse.column < graph_area.x + graph_area.width - 1
                            && mouse.row > graph_area.y
                            && mouse.row < graph_area.y + graph_area.height - 1
                        {
                            let clicked_row = (mouse.row - (graph_area.y + 1)) as usize;
                            if let Some((i, _)) = resolve_clicked_row(app_state, clicked_row) {
                                if let Some(repo) = &app_state.repo {
                                    if let Some(row) = repo.graph.get(i) {
                                        return Some(Action::OpenContextMenu(
                                            Some(row.commit_id.clone()),
                                            (mouse.column, mouse.row),
                                        ));
                                    }
                                }
                            }
                            None
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        },
    }
}

use std::future::Future;

async fn run_operation<F, Fut>(
    tx: mpsc::Sender<Action>,
    start_msg: String,
    success_msg: &'static str,
    action: F,
) where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send,
{
    let _ = tx.send(Action::OperationStarted(start_msg)).await;
    match action().await {
        Ok(()) => {
            let _ = tx
                .send(Action::OperationCompleted(Ok(success_msg.to_string())))
                .await;
        }
        Err(e) => {
            let _ = tx
                .send(Action::OperationCompleted(Err(format!("Error: {e}"))))
                .await;
        }
    }
}

pub(crate) fn handle_command(
    command: Command,
    adapter: Arc<dyn VcsFacade>,
    tx: mpsc::Sender<Action>,
) -> Result<()> {
    match command {
        Command::LoadRepoBackground(limit, revset) => {
            tokio::spawn(async move {
                match adapter.get_operation_log(None, limit, revset).await {
                    Ok(repo) => {
                        let _ = tx
                            .send(Action::RepoReloadedBackground(Box::new(repo)))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::ErrorOccurred(format!(
                                "Background sync failed: {e}"
                            )))
                            .await;
                    }
                }
            });
        }
        Command::LoadRepo(heads, limit, revset) => {
            let is_batch = heads.is_some();
            tokio::spawn(async move {
                match adapter.get_operation_log(heads, limit, revset).await {
                    Ok(repo) => {
                        if is_batch {
                            let _ = tx.send(Action::GraphBatchLoaded(Box::new(repo))).await;
                        } else {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::ErrorOccurred(format!("Failed to load repo: {e}")))
                            .await;
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
                            .send(Action::DiffLoaded(commit_id_clone, format!("Error: {e}")))
                            .await;
                    }
                }
            });
        }
        Command::DescribeRevision(commit_id, message) => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    format!("Describing {commit_id}..."),
                    "Described",
                    move || async move { adapter.describe_revision(&commit_id.0, &message).await },
                )
                .await;
            });
        }
        Command::Commit(message) => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    "Committing...".to_string(),
                    "Committed",
                    move || async move { adapter.commit(&message).await },
                )
                .await;
            });
        }
        Command::Snapshot => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted("Snapshotting...".to_string()))
                    .await;
                match adapter.snapshot().await {
                    Ok(msg) => {
                        let _ = tx.send(Action::OperationCompleted(Ok(msg))).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {e}"))))
                            .await;
                    }
                }
            });
        }
        Command::Edit(commit_id) => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    format!("Editing {commit_id}..."),
                    "Edit successful",
                    move || async move { adapter.edit(&commit_id).await },
                )
                .await;
            });
        }
        Command::Squash(commit_ids) => {
            tokio::spawn(async move {
                let msg = if commit_ids.len() == 1 {
                    format!("Squashing {}...", commit_ids[0])
                } else {
                    format!("Squashing {} revisions...", commit_ids.len())
                };
                run_operation(tx, msg, "Squash successful", move || async move {
                    adapter.squash(&commit_ids).await
                })
                .await;
            });
        }
        Command::New(commit_id) => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    format!("Creating child of {commit_id}..."),
                    "New revision created",
                    move || async move { adapter.new_child(&commit_id).await },
                )
                .await;
            });
        }
        Command::Absorb => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    "Absorbing changes...".to_string(),
                    "Absorb successful",
                    move || async move { adapter.absorb().await },
                )
                .await;
            });
        }
        Command::Duplicate(commit_ids) => {
            tokio::spawn(async move {
                let msg = if commit_ids.len() == 1 {
                    format!("Duplicating {}...", commit_ids[0])
                } else {
                    format!("Duplicating {} revisions...", commit_ids.len())
                };
                run_operation(tx, msg, "Revision(s) duplicated", move || async move {
                    adapter.duplicate(&commit_ids).await
                })
                .await;
            });
        }
        Command::Rebase(commit_ids, destination) => {
            tokio::spawn(async move {
                let msg = if commit_ids.len() == 1 {
                    format!("Rebasing {} onto {}...", commit_ids[0], destination)
                } else {
                    format!("Rebasing {} revisions onto {}...", commit_ids.len(), destination)
                };
                run_operation(tx, msg, "Rebase successful", move || async move {
                    adapter.rebase(&commit_ids, &destination).await
                })
                .await;
            });
        }
        Command::Parallelize(commit_ids) => {
            tokio::spawn(async move {
                let msg = if commit_ids.len() == 1 {
                    format!("Parallelizing {}...", commit_ids[0])
                } else {
                    format!("Parallelizing {} revisions...", commit_ids.len())
                };
                run_operation(
                    tx,
                    msg,
                    "Revision(s) parallelized",
                    move || async move { adapter.parallelize(&commit_ids).await },
                )
                .await;
            });
        }
        Command::Abandon(commit_ids) => {
            tokio::spawn(async move {
                let msg = if commit_ids.len() == 1 {
                    format!("Abandoning {}...", commit_ids[0])
                } else {
                    format!("Abandoning {} revisions...", commit_ids.len())
                };
                run_operation(tx, msg, "Revision(s) abandoned", move || async move {
                    adapter.abandon(&commit_ids).await
                })
                .await;
            });
        }
        Command::Revert(commit_ids) => {
            tokio::spawn(async move {
                let msg = if commit_ids.len() == 1 {
                    format!("Reverting {}...", commit_ids[0])
                } else {
                    format!("Reverting {} revisions...", commit_ids.len())
                };
                run_operation(tx, msg, "Revision(s) reverted", move || async move {
                    adapter.revert(&commit_ids).await
                })
                .await;
            });
        }

        Command::Split(_commit_id) => {
            // Handled directly in run_loop because it requires suspending TUI
        }
        Command::SetBookmark(commit_id, name) => {
            let name_clone = name.clone();
            tokio::spawn(async move {
                run_operation(
                    tx,
                    format!("Setting bookmark {name_clone}..."),
                    "Bookmark set", // Note: The previous success message sent back the bookmark name, but we use a static str now.
                    move || async move { adapter.set_bookmark(&commit_id, &name_clone).await },
                )
                .await;
            });
        }
        Command::DeleteBookmark(name) => {
            let name_clone = name.clone();
            tokio::spawn(async move {
                run_operation(
                    tx,
                    format!("Deleting bookmark {name_clone}..."),
                    "Bookmark deleted", // Note: Previous sent back the bookmark name
                    move || async move { adapter.delete_bookmark(&name_clone).await },
                )
                .await;
            });
        }
        Command::Undo => {
            tokio::spawn(async move {
                run_operation(tx, "Undoing...".to_string(), "Undo successful", move || async move {
                    adapter.undo().await
                })
                .await;
            });
        }
        Command::Redo => {
            tokio::spawn(async move {
                run_operation(tx, "Redoing...".to_string(), "Redo successful", move || async move {
                    adapter.redo().await
                })
                .await;
            });
        }
        Command::Fetch => {
            tokio::spawn(async move {
                run_operation(tx, "Fetching...".to_string(), "Fetch successful", move || async move {
                    adapter.fetch().await
                })
                .await;
            });
        }
        Command::Push(bookmark) => {
            let bookmark_clone = bookmark.clone();
            tokio::spawn(async move {
                let msg = if let Some(ref b) = bookmark_clone {
                    format!("Pushing {b}...")
                } else {
                    "Pushing...".to_string()
                };
                run_operation(tx, msg, "Push successful", move || async move {
                    adapter.push(bookmark_clone).await
                })
                .await;
            });
        }
        Command::ResolveConflict(_) => {
            // Handled specially in run_loop to allow TUI suspension
        }
        Command::InitRepo => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    "Initializing repository...".to_string(),
                    "Repository initialized",
                    move || async move { adapter.init_repo().await },
                )
                .await;
            });
        }
        Command::Evolog(commit_id) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Fetching evolog for {}...",
                        commit_id.0
                    )))
                    .await;
                match adapter.evolog(&commit_id).await {
                    Ok(content) => {
                        let _ = tx.send(Action::OpenEvolog(content)).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {e}"))))
                            .await;
                    }
                }
            });
        }
        Command::OperationLog => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted("Fetching operation log...".to_string()))
                    .await;
                match adapter.operation_log().await {
                    Ok(content) => {
                        let _ = tx.send(Action::OpenOperationLog(content)).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {e}"))))
                            .await;
                    }
                }
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::action::Action;
    use crate::app::command::Command;
    use crate::app::state::AppState;
    use crate::domain::models::CommitId;
    use crate::domain::vcs::MockVcsFacade;
    use crossterm::event::{Event, KeyCode, KeyModifiers};
    use rand::{Rng, SeedableRng};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_handle_command_error_propagation() {
        let mut mock = MockVcsFacade::new();
        let commit_id = CommitId("test-commit".to_string());
        let commit_id_clone = commit_id.clone();

        // Simulate a failure in get_commit_diff
        mock.expect_get_commit_diff()
            .with(mockall::predicate::eq(commit_id_clone))
            .returning(|_| Err(anyhow::anyhow!("VCS Error")));

        let adapter = Arc::new(mock);
        let (tx, mut rx) = mpsc::channel(1);

        handle_command(Command::LoadDiff(commit_id), adapter, tx).unwrap();

        // We expect a DiffLoaded action with an error message in it
        let action = rx.recv().await.unwrap();
        if let Action::DiffLoaded(_, diff) = action {
            assert!(diff.contains("Error: VCS Error"));
        } else {
            panic!("Expected Action::DiffLoaded, got {action:?}");
        }
    }

    #[tokio::test]
    async fn test_handle_command_success() {
        let mut mock = MockVcsFacade::new();
        let commit_id = CommitId("test-commit".to_string());
        let commit_id_clone = commit_id.clone();

        // Simulate a success
        mock.expect_get_commit_diff()
            .with(mockall::predicate::eq(commit_id_clone))
            .returning(|_| Ok("Diff Content".to_string()));

        let adapter = Arc::new(mock);
        let (tx, mut rx) = mpsc::channel(1);

        handle_command(Command::LoadDiff(commit_id), adapter, tx).unwrap();

        let action = rx.recv().await.unwrap();
        if let Action::DiffLoaded(_, diff) = action {
            assert_eq!(diff, "Diff Content");
        } else {
            panic!("Expected Action::DiffLoaded, got {action:?}");
        }
    }

    #[tokio::test]
    async fn test_full_command_error_to_state() {
        let mut mock = MockVcsFacade::new();
        mock.expect_snapshot()
            .returning(|| Err(anyhow::anyhow!("Snapshot failed")));

        let adapter = Arc::new(mock);
        let (tx, mut rx) = mpsc::channel(2);
        let mut state = crate::app::state::AppState::default();

        handle_command(Command::Snapshot, adapter, tx).unwrap();

        // 1. First action: OperationStarted
        let action1 = rx.recv().await.unwrap();
        crate::app::reducer::update(&mut state, action1);
        assert_eq!(state.mode, crate::app::state::AppMode::Loading);
        assert!(state
            .active_tasks
            .iter()
            .any(|t| t.contains("Snapshotting")));

        // 2. Second action: OperationCompleted(Err)
        let action2 = rx.recv().await.unwrap();
        crate::app::reducer::update(&mut state, action2);

        // Mode should reset to NoRepo (since no repo in state) and error should be set
        assert_eq!(state.mode, crate::app::state::AppMode::NoRepo);
        assert!(state.last_error.is_some());
        assert!(state
            .last_error
            .unwrap()
            .message
            .contains("Error: Snapshot failed"));
    }

    #[tokio::test]
    async fn test_keystroke_fuzzing() {
        let mut mock = MockVcsFacade::new();
        // Setup mock to return some data to avoid crashes in UI
        mock.expect_workspace_root()
            .returning(|| std::path::PathBuf::from("/tmp"));
        mock.expect_operation_log()
            .returning(|| Ok("op log content".to_string()));
        mock.expect_get_operation_log().returning(|_, _, _| {
            Ok(crate::domain::models::RepoStatus {
                repo_name: "test-repo".to_string(),
                operation_id: "test".to_string(),
                workspace_id: "default".to_string(),
                working_copy_id: crate::domain::models::CommitId("wc".to_string()),
                graph: vec![crate::domain::models::GraphRow {
                    timestamp_secs: 0,
                    commit_id: crate::domain::models::CommitId("wc".to_string()),
                    commit_id_short: "wc".to_string(),
                    change_id: "wc".to_string(),
                    change_id_short: "wc".to_string(),
                    description: "desc".to_string(),
                    author: "author".to_string(),
                    timestamp: "time".to_string(),
                    is_working_copy: true,
                    is_immutable: false,
                    parents: vec![],
                    bookmarks: vec![],
                    changed_files: vec![crate::domain::models::FileChange {
                        path: "file.txt".to_string(),
                        status: crate::domain::models::FileStatus::Modified,
                    }],
                    visual: crate::domain::models::GraphRowVisual::default(),
                }],
            })
        });
        mock.expect_get_commit_diff()
            .returning(|_| Ok("diff content".to_string()));
        mock.expect_snapshot()
            .returning(|| Ok("snapshot".to_string()));
        mock.expect_new_child().returning(|_| Ok(()));
        mock.expect_edit().returning(|_| Ok(()));
        mock.expect_squash().returning(|_| Ok(()));
        mock.expect_abandon().returning(|_| Ok(()));
        mock.expect_absorb().returning(|| Ok(()));
        mock.expect_duplicate().returning(|_| Ok(()));
        mock.expect_set_bookmark().returning(|_, _| Ok(()));
        mock.expect_delete_bookmark().returning(|_| Ok(()));
        mock.expect_undo().returning(|| Ok(()));
        mock.expect_redo().returning(|| Ok(()));
        mock.expect_fetch().returning(|| Ok(()));
        mock.expect_push().returning(|_| Ok(()));
        mock.expect_describe_revision().returning(|_, _| Ok(()));
        mock.expect_evolog().returning(|_| Ok("evolog".to_string()));
        mock.expect_rebase().returning(|_, _| Ok(()));
        mock.expect_parallelize().returning(|_| Ok(()));
        mock.expect_revert().returning(|_| Ok(()));

        let adapter = Arc::new(mock);
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let app_state = AppState::default();

        let (event_tx, event_rx) = mpsc::channel(100);

        // Spawn a task to feed random events
        let fuzzer_handle = tokio::spawn(async move {
            let mut rng = rand::rngs::StdRng::seed_from_u64(42);
            for _ in 0..10000 {
                let event = match rng.gen_range(0..100) {
                    0..=5 => {
                        let w = rng.gen_range(10..200);
                        let h = rng.gen_range(10..100);
                        Event::Resize(w, h)
                    }
                    6..=15 => generate_random_mouse(&mut rng, ratatui::layout::Size::new(80, 24)),
                    _ => generate_random_key(&mut rng),
                };
                if event_tx.send(Ok(event)).await.is_err() {
                    break;
                }
                // Yield to allow the loop to process events
                if rng.gen_bool(0.1) {
                    tokio::task::yield_now().await;
                }
            }
            // Send Quit
            let _ = event_tx
                .send(Ok(Event::Key(crossterm::event::KeyEvent::new(
                    KeyCode::Char('q'),
                    KeyModifiers::NONE,
                ))))
                .await;
        });

        // Run the real loop (with a test backend)
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            run_loop_with_events(&mut terminal, app_state, adapter, event_rx),
        )
        .await;

        match result {
            Ok(res) => res.unwrap(),
            Err(_) => panic!("Fuzzer timed out - possible deadlock or too slow"),
        }

        fuzzer_handle.await.unwrap();
    }

    fn generate_random_key<R: Rng>(rng: &mut R) -> Event {
        use crossterm::event::KeyEvent;
        let code = match rng.gen_range(0..20) {
            0 => KeyCode::Esc,
            1 => KeyCode::Enter,
            2 => KeyCode::Left,
            3 => KeyCode::Right,
            4 => KeyCode::Up,
            5 => KeyCode::Down,
            6 => KeyCode::Home,
            7 => KeyCode::End,
            8 => KeyCode::PageUp,
            9 => KeyCode::PageDown,
            10 => KeyCode::Tab,
            11 => KeyCode::BackTab,
            12 => KeyCode::Delete,
            13 => KeyCode::Backspace,
            _ => {
                let c = rng.gen_range(b' '..=b'~') as char;
                KeyCode::Char(c)
            }
        };

        let mut modifiers = KeyModifiers::empty();
        if rng.gen_bool(0.1) {
            modifiers.insert(KeyModifiers::CONTROL);
        }
        if rng.gen_bool(0.1) {
            modifiers.insert(KeyModifiers::ALT);
        }
        if rng.gen_bool(0.1) {
            modifiers.insert(KeyModifiers::SHIFT);
        }

        Event::Key(KeyEvent::new(code, modifiers))
    }

    fn generate_random_mouse<R: Rng>(rng: &mut R, size: ratatui::layout::Size) -> Event {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        let kind = match rng.gen_range(0..5) {
            0 => MouseEventKind::Down(MouseButton::Left),
            1 => MouseEventKind::Down(MouseButton::Right),
            2 => MouseEventKind::ScrollUp,
            3 => MouseEventKind::ScrollDown,
            _ => MouseEventKind::Moved,
        };

        let column = rng.gen_range(0..size.width);
        let row = rng.gen_range(0..size.height);

        Event::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: crossterm::event::KeyModifiers::empty(),
        })
    }
}
