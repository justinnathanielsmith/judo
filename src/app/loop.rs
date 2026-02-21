use crate::app::{
    action::Action,
    command::Command,
    reducer,
    state::AppState,
    ui,
};
use crate::domain::vcs::VcsFacade;
use crate::theme::Theme;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, MouseButton, MouseEventKind};
use ratatui::{backend::Backend, Terminal};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;

const TICK_RATE: Duration = Duration::from_millis(250);

pub async fn run_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app_state: AppState<'_>,
    adapter: Arc<dyn VcsFacade>,
) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::channel(100);
    let mut interval = interval(TICK_RATE);
    let theme = Theme::default();

    // User input channel
    let (event_tx, mut event_rx) = mpsc::channel(100);
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

    // Initial Load
    handle_command(Command::LoadRepo(None, 100, None), adapter.clone(), action_tx.clone()).await?;


    loop {
        // --- 1. Render ---
        terminal.draw(|f| {
            ui::draw(f, &mut app_state, &theme);
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
                    crate::app::state::AppMode::Input | crate::app::state::AppMode::BookmarkInput => {
                        match event {
                            Event::Key(key) => {
                                match key.code {
                                    KeyCode::Esc => Some(Action::CancelMode),
                                    KeyCode::Enter => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            if let Some(row) = repo.graph.get(idx) {
                                                if app_state.mode == crate::app::state::AppMode::BookmarkInput {
                                                    Some(Action::SetBookmark(row.commit_id.clone(), app_state.text_area.lines().join("")))
                                                } else {
                                                    Some(Action::DescribeRevision(row.commit_id.clone(), app_state.text_area.lines().join("\n")))
                                                }
                                            } else { None }
                                        } else { None }
                                    },
                                    _ => {
                                        Some(Action::TextAreaInput(key))
                                    }
                                }
                            },
                            _ => None,
                        }
                    },
                    crate::app::state::AppMode::FilterInput => {
                        match event {
                            Event::Key(key) => {
                                match key.code {
                                    KeyCode::Esc => Some(Action::CancelMode),
                                    KeyCode::Enter => {
                                        Some(Action::ApplyFilter(app_state.text_area.lines().join("")))
                                    },
                                    _ => {
                                        Some(Action::TextAreaInput(key))
                                    }
                                }
                            },
                            _ => None,
                        }
                    },
                    crate::app::state::AppMode::ContextMenu => {
                        match event {
                            Event::Key(key) => {
                                match key.code {
                                    KeyCode::Esc => Some(Action::CloseContextMenu),
                                    KeyCode::Char('j') | KeyCode::Down => Some(Action::SelectContextMenuNext),
                                    KeyCode::Char('k') | KeyCode::Up => Some(Action::SelectContextMenuPrev),
                                    KeyCode::Enter => {
                                        app_state.context_menu.as_ref().map(|menu| Action::SelectContextMenuAction(menu.selected_index))
                                    },
                                    _ => None,
                                }
                            },
                            Event::Mouse(mouse) => {
                                match mouse.kind {
                                    MouseEventKind::Down(MouseButton::Left) => {
                                        if let Some(menu) = &app_state.context_menu {
                                            let size = terminal.size()?;
                                            let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                                            let menu_area = menu.calculate_rect(area);

                                            if mouse.column >= menu_area.x && mouse.column < menu_area.x + menu_area.width
                                                && mouse.row >= menu_area.y && mouse.row < menu_area.y + menu_area.height
                                            {
                                                // Adjust for borders: top/left border is at y/x, so content starts at y+1
                                                let clicked_idx = (mouse.row.saturating_sub(menu_area.y + 1)) as usize;
                                                Some(Action::SelectContextMenuAction(clicked_idx))
                                            } else {
                                                Some(Action::CloseContextMenu)
                                            }
                                        } else {
                                            Some(Action::CloseContextMenu)
                                        }
                                    },
                                    _ => None,
                                }
                            }
                            _ => None,
                        }
                    },
                    crate::app::state::AppMode::Diff => {
                        match event {
                            Event::Key(key) => {
                                match key.code {
                                    KeyCode::Esc => Some(Action::CancelMode),
                                    KeyCode::Char('q') => Some(Action::Quit),
                                    KeyCode::Char('h') | KeyCode::Tab => Some(Action::FocusGraph),
                                    KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNextFile),
                                    KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrevFile),
                                    KeyCode::PageDown => Some(Action::ScrollDiffDown(10)),
                                    KeyCode::PageUp => Some(Action::ScrollDiffUp(10)),
                                    KeyCode::Char('[') => Some(Action::PrevHunk),
                                    KeyCode::Char(']') => Some(Action::NextHunk),
                                    KeyCode::Char('m') | KeyCode::Enter => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            if let Some(row) = repo.graph.get(idx) {
                                                if let Some(file_idx) = app_state.selected_file_index {
                                                    if let Some(file) = row.changed_files.get(file_idx) {
                                                        if file.status == crate::domain::models::FileStatus::Conflicted {
                                                            Some(Action::ResolveConflict(file.path.clone()))
                                                        } else { None }
                                                    } else { None }
                                                } else { None }
                                            } else { None }
                                        } else { None }
                                    }
                                    _ => None,
                                }
                            }
                            Event::Mouse(mouse) => {
                                match mouse.kind {
                                    MouseEventKind::ScrollUp => Some(Action::ScrollDiffUp(1)),
                                    MouseEventKind::ScrollDown => Some(Action::ScrollDiffDown(1)),
                                    MouseEventKind::Down(MouseButton::Left) => {
                                        let now = Instant::now();
                                        let is_double_click = app_state.last_click_time.is_some_and(|t| now.duration_since(t).as_millis() < 500)
                                            && app_state.last_click_pos == Some((mouse.column, mouse.row));
                                        app_state.last_click_time = Some(now);
                                        app_state.last_click_pos = Some((mouse.column, mouse.row));

                                        let size = terminal.size()?;
                                        let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                                        let layout = ui::get_layout(area, app_state.show_diffs);

                                        // Revision Graph Area
                                        let graph_area = layout.body[0];
                                        if mouse.column > graph_area.x && mouse.column < graph_area.x + graph_area.width - 1
                                            && mouse.row > graph_area.y && mouse.row < graph_area.y + graph_area.height - 1
                                        {
                                            if is_double_click {
                                                Some(Action::ToggleDiffs)
                                            } else {
                                                let clicked_row = (mouse.row - (graph_area.y + 1)) as usize;
                                                let offset = app_state.log_list_state.offset();
                                                let mut result = None;
                                                if let Some(repo) = &app_state.repo {
                                                    let mut current_y = 0;
                                                    for i in offset..repo.graph.len() {
                                                        let row = &repo.graph[i];
                                                        let is_selected = app_state.log_list_state.selected() == Some(i);
                                                        let row_height = 2 + if is_selected && app_state.show_diffs { row.changed_files.len() } else { 0 };

                                                        if clicked_row >= current_y && clicked_row < current_y + row_height {
                                                            if is_selected && app_state.show_diffs && clicked_row >= current_y + 2 {
                                                                let file_idx = clicked_row - (current_y + 2);
                                                                result = Some(Action::SelectFile(file_idx));
                                                            } else {
                                                                result = Some(Action::SelectIndex(i));
                                                            }
                                                            break;
                                                        }
                                                        current_y += row_height;
                                                        if current_y > graph_area.height as usize {
                                                            break;
                                                        }
                                                    }
                                                }
                                                result
                                            }
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                }
                            }
                            _ => None,
                        }
                    }
                    _ => {
                        match event {
                            Event::Key(key) => {
                                match key.code {
                                    KeyCode::Char('q') => Some(Action::Quit),
                                    KeyCode::Enter => Some(Action::ToggleDiffs),
                                    KeyCode::Tab | KeyCode::Char('l') => Some(Action::FocusDiff),
                                    KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNext),
                                    KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrev),
                                    KeyCode::Char('s') => Some(Action::SnapshotWorkingCopy),
                                    KeyCode::Char('S') => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            repo.graph.get(idx).map(|row| Action::SquashRevision(row.commit_id.clone()))
                                        } else { None }
                                    },
                                    KeyCode::Char('e') => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            repo.graph.get(idx).map(|row| Action::EditRevision(row.commit_id.clone()))
                                        } else { None }
                                    },
                                    KeyCode::Char('n') => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            repo.graph.get(idx).map(|row| Action::NewRevision(row.commit_id.clone()))
                                        } else { None }
                                    },
                                    KeyCode::Char('a') => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            repo.graph.get(idx).map(|row| Action::AbandonRevision(row.commit_id.clone()))
                                        } else { None }
                                    },
                                    KeyCode::Char('b') => Some(Action::SetBookmarkIntent),
                                    KeyCode::Char('B') => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            if let Some(row) = repo.graph.get(idx) {
                                                // For now, delete the first bookmark if exists
                                                row.bookmarks.first().map(|bookmark| Action::DeleteBookmark(bookmark.clone()))
                                            } else { None }
                                        } else { None }
                                    },
                                    KeyCode::Char('d') => Some(Action::DescribeRevisionIntent),
                                    KeyCode::Char('m') => Some(Action::FilterMine),
                                    KeyCode::Char('t') => Some(Action::FilterTrunk),
                                    KeyCode::Char('c') => Some(Action::FilterConflicts),
                                    KeyCode::Char('u') => Some(Action::Undo),
                                    KeyCode::Char('U') => Some(Action::Redo),
                                    KeyCode::Char('f') => Some(Action::Fetch),
                                    KeyCode::Char('/') => Some(Action::EnterFilterMode),
                                    KeyCode::Char('p') => Some(Action::PushIntent),
                                    KeyCode::PageDown => Some(Action::ScrollDiffDown(10)),
                                    KeyCode::PageUp => Some(Action::ScrollDiffUp(10)),
                                    KeyCode::Char('[') => Some(Action::PrevHunk),
                                    KeyCode::Char(']') => Some(Action::NextHunk),
                                    KeyCode::Esc => Some(Action::CancelMode),
                                    _ => None,
                                }
                            },
                            Event::Mouse(mouse) => {
                                let size = terminal.size()?;
                                let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                                let layout = ui::get_layout(area, app_state.show_diffs);

                                let graph_area = layout.body[0];
                                let diff_area = layout.body[1];

                                match mouse.kind {
                                    MouseEventKind::ScrollUp => {
                                        if app_state.show_diffs && mouse.column >= diff_area.x && mouse.column < diff_area.x + diff_area.width {
                                            Some(Action::ScrollDiffUp(1))
                                        } else {
                                            Some(Action::SelectPrev)
                                        }
                                    }
                                    MouseEventKind::ScrollDown => {
                                        if app_state.show_diffs && mouse.column >= diff_area.x && mouse.column < diff_area.x + diff_area.width {
                                            Some(Action::ScrollDiffDown(1))
                                        } else {
                                            Some(Action::SelectNext)
                                        }
                                    }
                                    MouseEventKind::Down(MouseButton::Left) => {
                                        let now = Instant::now();
                                        let is_double_click = app_state.last_click_time.is_some_and(|t| now.duration_since(t).as_millis() < 500)
                                            && app_state.last_click_pos == Some((mouse.column, mouse.row));
                                        app_state.last_click_time = Some(now);
                                        app_state.last_click_pos = Some((mouse.column, mouse.row));

                                        // Double click anywhere toggles the diff panel
                                        if is_double_click {
                                            Some(Action::ToggleDiffs)
                                        } else if mouse.column > graph_area.x && mouse.column < graph_area.x + graph_area.width - 1
                                            && mouse.row > graph_area.y && mouse.row < graph_area.y + graph_area.height - 1
                                        {
                                            let clicked_row = (mouse.row - (graph_area.y + 1)) as usize;
                                            let offset = app_state.log_list_state.offset();
                                            let mut result = None;
                                            if let Some(repo) = &app_state.repo {
                                                let mut current_y = 0;
                                                for i in offset..repo.graph.len() {
                                                    let row = &repo.graph[i];
                                                    let is_selected = app_state.log_list_state.selected() == Some(i);
                                                    let row_height = ui::calculate_row_height(row, is_selected, app_state.show_diffs) as usize;

                                                    if clicked_row >= current_y && clicked_row < current_y + row_height {
                                                        if is_selected && app_state.show_diffs && clicked_row >= current_y + 2 {
                                                            let file_idx = clicked_row - (current_y + 2);
                                                            result = Some(Action::SelectFile(file_idx));
                                                        } else {
                                                            result = Some(Action::SelectIndex(i));
                                                        }
                                                        break;
                                                    }
                                                    current_y += row_height;
                                                    if current_y > graph_area.height as usize {
                                                        break;
                                                    }
                                                }
                                            }
                                            result
                                        } else if app_state.show_diffs && mouse.column >= diff_area.x && mouse.column < diff_area.x + diff_area.width {
                                            Some(Action::FocusDiff)
                                        } else {
                                            None
                                        }
                                    }
                                    MouseEventKind::Down(MouseButton::Right) => {
                                        if mouse.column > graph_area.x && mouse.column < graph_area.x + graph_area.width - 1
                                            && mouse.row > graph_area.y && mouse.row < graph_area.y + graph_area.height - 1
                                        {
                                            let clicked_row = (mouse.row - (graph_area.y + 1)) as usize;
                                            let offset = app_state.log_list_state.offset();
                                            let mut result = None;
                                            if let Some(repo) = &app_state.repo {
                                                let mut current_y = 0;
                                                for i in offset..repo.graph.len() {
                                                    let row = &repo.graph[i];
                                                    let is_selected = app_state.log_list_state.selected() == Some(i);
                                                    let row_height = ui::calculate_row_height(row, is_selected, app_state.show_diffs) as usize;

                                                    if clicked_row >= current_y && clicked_row < current_y + row_height {
                                                        // Selection happens on right click too
                                                        action_tx.try_send(Action::SelectIndex(i)).ok();
                                                        result = Some(Action::OpenContextMenu(row.commit_id.clone(), (mouse.column, mouse.row)));
                                                        break;
                                                    }
                                                    current_y += row_height;
                                                    if current_y > graph_area.height as usize {
                                                        break;
                                                    }
                                                }
                                            }
                                            result
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                }
                            }
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
                if let Command::ResolveConflict(path) = cmd {
                    // 1. Suspend TUI
                    crossterm::terminal::disable_raw_mode()?;
                    crossterm::execute!(
                        std::io::stdout(),
                        crossterm::terminal::LeaveAlternateScreen,
                        crossterm::cursor::Show
                    )?;

                    // 2. Run external tool
                    // We'll use 'jj resolve' which uses the configured tool
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
                    let _ = action_tx.send(Action::OperationCompleted(if status.success() {
                        Ok(format!("Resolved {}", path))
                    } else {
                        Err(format!("Resolve failed for {}", path))
                    })).await;
                } else {
                    handle_command(cmd, adapter.clone(), action_tx.clone()).await?;
                }
            }
        }
    }

    Ok(())
}

async fn handle_command(
    command: Command,
    adapter: Arc<dyn VcsFacade>,
    tx: mpsc::Sender<Action>,
) -> Result<()> {
    match command {
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
                            .send(Action::ErrorOccurred(format!("Failed to load repo: {}", e)))
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
                let _ = tx
                    .send(Action::OperationStarted("Snapshotting...".to_string()))
                    .await;
                match adapter.snapshot().await {
                    Ok(msg) => {
                        let _ = tx.send(Action::OperationCompleted(Ok(msg))).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Edit(commit_id) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Editing {}...",
                        commit_id
                    )))
                    .await;
                match adapter.edit(&commit_id).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(
                                Ok("Edit successful".to_string()),
                            ))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Squash(commit_id) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Squashing {}...",
                        commit_id
                    )))
                    .await;
                match adapter.squash(&commit_id).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok(
                                "Squash successful".to_string()
                            )))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::New(commit_id) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Creating child of {}...",
                        commit_id
                    )))
                    .await;
                match adapter.new_child(&commit_id).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok(
                                "New revision created".to_string()
                            )))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Abandon(commit_id) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Abandoning {}...",
                        commit_id
                    )))
                    .await;
                match adapter.abandon(&commit_id).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok(
                                "Revision abandoned".to_string()
                            )))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::SetBookmark(commit_id, name) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Setting bookmark {}...",
                        name
                    )))
                    .await;
                match adapter.set_bookmark(&commit_id, &name).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok(format!(
                                "Bookmark {} set",
                                name
                            ))))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::DeleteBookmark(name) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Deleting bookmark {}...",
                        name
                    )))
                    .await;
                match adapter.delete_bookmark(&name).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok(format!(
                                "Bookmark {} deleted",
                                name
                            ))))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Undo => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted("Undoing...".to_string()))
                    .await;
                match adapter.undo().await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(
                                Ok("Undo successful".to_string()),
                            ))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Redo => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted("Redoing...".to_string()))
                    .await;
                match adapter.redo().await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(
                                Ok("Redo successful".to_string()),
                            ))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Fetch => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted("Fetching...".to_string()))
                    .await;
                match adapter.fetch().await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok("Fetch successful".to_string())))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Push(bookmark) => {
            let bookmark_clone = bookmark.clone();
            tokio::spawn(async move {
                let msg = if let Some(ref b) = bookmark_clone {
                    format!("Pushing {}...", b)
                } else {
                    "Pushing...".to_string()
                };
                let _ = tx.send(Action::OperationStarted(msg)).await;
                match adapter.push(bookmark_clone).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok("Push successful".to_string())))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::ResolveConflict(_) => {
            // Handled specially in run_loop to allow TUI suspension
        }
    }
    Ok(())
}
