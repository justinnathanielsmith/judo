use crate::app::{action::Action, command::Command, reducer, state::AppState, ui};
use crate::domain::vcs::VcsFacade;
use crate::theme::Theme;

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
    let theme = Theme::default();

    // Repository Watcher
    let (notify_tx, mut notify_rx) = mpsc::channel(1);
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if res.is_ok() {
            let _ = notify_tx.blocking_send(());
        }
    })?;

    let repo_path = adapter.workspace_root();
    let op_heads_path = repo_path.join(".jj").join("repo").join("op_heads");
    if op_heads_path.exists() {
        watcher.watch(&op_heads_path, RecursiveMode::NonRecursive)?;
    }

    // Initial Load
    handle_command(
        Command::LoadRepo(None, 100, None),
        adapter.clone(),
        action_tx.clone(),
    )
    .await?;

    loop {
        // --- 1. Render ---
        terminal.draw(|f| {
            ui::draw(f, &mut app_state, &theme);
        })?;

        // --- 2. Event Handling (TEA Runtime) ---
        let action = tokio::select! {
            _ = interval.tick() => Some(Action::Tick),

            // External Changes
            Some(_) = notify_rx.recv() => Some(Action::ExternalChangeDetected),

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
                    // SECURITY: Validate path to prevent path traversal
                    if path.contains("..") {
                        crossterm::terminal::enable_raw_mode()?;
                        crossterm::execute!(
                            std::io::stdout(),
                            crossterm::terminal::EnterAlternateScreen,
                            crossterm::cursor::Hide
                        )?;
                        terminal.clear()?;
                        let _ = action_tx
                            .send(Action::OperationCompleted(Err(format!(
                                "Invalid path: {}",
                                path
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
                            Ok(format!("Resolved {}", path))
                        } else {
                            Err(format!("Resolve failed for {}", path))
                        }))
                        .await;
                } else {
                    handle_command(cmd, adapter.clone(), action_tx.clone()).await?;
                }
            }
        }
    }

    Ok(())
}

pub fn map_event_to_action(
    event: Event,
    app_state: &AppState<'_>,
    terminal_size: ratatui::layout::Size,
) -> Option<Action> {
    match app_state.mode {
        crate::app::state::AppMode::Input | crate::app::state::AppMode::BookmarkInput => {
            match event {
                Event::Key(key) => match key.code {
                    KeyCode::Esc => Some(Action::CancelMode),
                    KeyCode::Enter => {
                        if let (Some(repo), Some(idx)) =
                            (&app_state.repo, app_state.log_list_state.selected())
                        {
                            if let Some(row) = repo.graph.get(idx) {
                                if app_state.mode == crate::app::state::AppMode::BookmarkInput {
                                    Some(Action::SetBookmark(
                                        row.commit_id.clone(),
                                        app_state.text_area.lines().join(""),
                                    ))
                                } else {
                                    Some(Action::DescribeRevision(
                                        row.commit_id.clone(),
                                        app_state.text_area.lines().join("\n"),
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
        crate::app::state::AppMode::FilterInput => {
            match event {
                Event::Key(key) => match key.code {
                    KeyCode::Esc => Some(Action::CancelMode),
                    KeyCode::Enter => {
                        Some(Action::ApplyFilter(app_state.text_area.lines().join("")))
                    }
                    _ => Some(Action::TextAreaInput(key)),
                },
                _ => None,
            }
        }
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
        crate::app::state::AppMode::Loading => None,
        crate::app::state::AppMode::Diff => {
            match event {
                Event::Key(key) => match key.code {
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
                        if let (Some(repo), Some(idx)) =
                            (&app_state.repo, app_state.log_list_state.selected())
                        {
                            if let Some(row) = repo.graph.get(idx) {
                                if let Some(file_idx) = app_state.selected_file_index {
                                    if let Some(file) = row.changed_files.get(file_idx) {
                                        if file.status == crate::domain::models::FileStatus::Conflicted
                                        {
                                            Some(Action::ResolveConflict(file.path.clone()))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => Some(Action::ScrollDiffUp(1)),
                    MouseEventKind::ScrollDown => Some(Action::ScrollDiffDown(1)),
                    MouseEventKind::Down(MouseButton::Left) => {
                        let now = Instant::now();
                        let is_double_click = app_state.last_click_time.is_some_and(|t| {
                            now.duration_since(t).as_millis() < 500
                        }) && app_state.last_click_pos == Some((mouse.column, mouse.row));

                        let area = ratatui::layout::Rect::new(
                            0,
                            0,
                            terminal_size.width,
                            terminal_size.height,
                        );
                        let layout = ui::get_layout(area, app_state.show_diffs);

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
                                let offset = app_state.log_list_state.offset();
                                let mut result = None;
                                if let Some(repo) = &app_state.repo {
                                    let mut current_y = 0;
                                    for i in offset..repo.graph.len() {
                                        let row = &repo.graph[i];
                                        let is_selected =
                                            app_state.log_list_state.selected() == Some(i);
                                        let row_height = 2 + if is_selected && app_state.show_diffs {
                                            row.changed_files.len()
                                        } else {
                                            0
                                        };

                                        if clicked_row >= current_y
                                            && clicked_row < current_y + row_height
                                        {
                                            if is_selected
                                                && app_state.show_diffs
                                                && clicked_row >= current_y + 2
                                            {
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
                },
                _ => None,
            }
        }
        _ => match event {
            Event::Resize(w, h) => Some(Action::Resize(w, h)),
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Enter => Some(Action::ToggleDiffs),
                KeyCode::Tab | KeyCode::Char('l') => Some(Action::FocusDiff),
                KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNext),
                KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrev),
                KeyCode::Char('s') => Some(Action::SnapshotWorkingCopy),
                KeyCode::Char('S') => {
                    if let (Some(repo), Some(idx)) =
                        (&app_state.repo, app_state.log_list_state.selected())
                    {
                        repo.graph
                            .get(idx)
                            .map(|row| Action::SquashRevision(row.commit_id.clone()))
                    } else {
                        None
                    }
                }
                KeyCode::Char('e') => {
                    if let (Some(repo), Some(idx)) =
                        (&app_state.repo, app_state.log_list_state.selected())
                    {
                        repo.graph
                            .get(idx)
                            .map(|row| Action::EditRevision(row.commit_id.clone()))
                    } else {
                        None
                    }
                }
                KeyCode::Char('n') => {
                    if let (Some(repo), Some(idx)) =
                        (&app_state.repo, app_state.log_list_state.selected())
                    {
                        repo.graph
                            .get(idx)
                            .map(|row| Action::NewRevision(row.commit_id.clone()))
                    } else {
                        None
                    }
                }
                KeyCode::Char('a') => {
                    if let (Some(repo), Some(idx)) =
                        (&app_state.repo, app_state.log_list_state.selected())
                    {
                        repo.graph
                            .get(idx)
                            .map(|row| Action::AbandonRevision(row.commit_id.clone()))
                    } else {
                        None
                    }
                }
                KeyCode::Char('b') => Some(Action::SetBookmarkIntent),
                KeyCode::Char('B') => {
                    if let (Some(repo), Some(idx)) =
                        (&app_state.repo, app_state.log_list_state.selected())
                    {
                        if let Some(row) = repo.graph.get(idx) {
                            // For now, delete the first bookmark if exists
                            row.bookmarks
                                .first()
                                .map(|bookmark| Action::DeleteBookmark(bookmark.clone()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
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
            },
            Event::Mouse(mouse) => {
                let area = ratatui::layout::Rect::new(0, 0, terminal_size.width, terminal_size.height);
                let layout = ui::get_layout(area, app_state.show_diffs);

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
                        let is_double_click = app_state.last_click_time.is_some_and(|t| {
                            now.duration_since(t).as_millis() < 500
                        }) && app_state.last_click_pos == Some((mouse.column, mouse.row));

                        // Double click anywhere toggles the diff panel
                        if is_double_click {
                            Some(Action::ToggleDiffs)
                        } else if mouse.column > graph_area.x
                            && mouse.column < graph_area.x + graph_area.width - 1
                            && mouse.row > graph_area.y
                            && mouse.row < graph_area.y + graph_area.height - 1
                        {
                            let clicked_row = (mouse.row - (graph_area.y + 1)) as usize;
                            let offset = app_state.log_list_state.offset();
                            let mut result = None;
                            if let Some(repo) = &app_state.repo {
                                let mut current_y = 0;
                                for i in offset..repo.graph.len() {
                                    let row = &repo.graph[i];
                                    let is_selected =
                                        app_state.log_list_state.selected() == Some(i);
                                    let row_height = ui::calculate_row_height(
                                        row,
                                        is_selected,
                                        app_state.show_diffs,
                                    ) as usize;

                                    if clicked_row >= current_y
                                        && clicked_row < current_y + row_height
                                    {
                                        if is_selected
                                            && app_state.show_diffs
                                            && clicked_row >= current_y + 2
                                        {
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
                            let offset = app_state.log_list_state.offset();
                            let mut result = None;
                            if let Some(repo) = &app_state.repo {
                                let mut current_y = 0;
                                for i in offset..repo.graph.len() {
                                    let row = &repo.graph[i];
                                    let is_selected =
                                        app_state.log_list_state.selected() == Some(i);
                                    let row_height = ui::calculate_row_height(
                                        row,
                                        is_selected,
                                        app_state.show_diffs,
                                    ) as usize;

                                    if clicked_row >= current_y
                                        && clicked_row < current_y + row_height
                                    {
                                        result = Some(Action::OpenContextMenu(
                                            row.commit_id.clone(),
                                            (mouse.column, mouse.row),
                                        ));
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
        },
    }
}

pub(crate) async fn handle_command(
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
                            .send(Action::OperationCompleted(Ok(
                                "Fetch successful".to_string()
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
                            .send(Action::OperationCompleted(
                                Ok("Push successful".to_string()),
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
        Command::ResolveConflict(_) => {
            // Handled specially in run_loop to allow TUI suspension
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

        handle_command(Command::LoadDiff(commit_id), adapter, tx).await.unwrap();

        // We expect a DiffLoaded action with an error message in it
        let action = rx.recv().await.unwrap();
        if let Action::DiffLoaded(_, diff) = action {
            assert!(diff.contains("Error: VCS Error"));
        } else {
            panic!("Expected Action::DiffLoaded, got {:?}", action);
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
            .returning(|_| {
                Ok("Diff Content".to_string())
            });

        let adapter = Arc::new(mock);
        let (tx, mut rx) = mpsc::channel(1);

        handle_command(Command::LoadDiff(commit_id), adapter, tx).await.unwrap();

        let action = rx.recv().await.unwrap();
        if let Action::DiffLoaded(_, diff) = action {
            assert_eq!(diff, "Diff Content");
        } else {
            panic!("Expected Action::DiffLoaded, got {:?}", action);
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

        handle_command(Command::Snapshot, adapter, tx).await.unwrap();

        // 1. First action: OperationStarted
        let action1 = rx.recv().await.unwrap();
        crate::app::reducer::update(&mut state, action1);
        assert_eq!(state.mode, crate::app::state::AppMode::Loading);
        assert!(state.active_tasks.iter().any(|t| t.contains("Snapshotting")));

        // 2. Second action: OperationCompleted(Err)
        let action2 = rx.recv().await.unwrap();
        crate::app::reducer::update(&mut state, action2);

        // Mode should reset and error should be set
        assert_eq!(state.mode, crate::app::state::AppMode::Normal);
        assert!(state.last_error.is_some());
        assert!(state.last_error.unwrap().contains("Error: Snapshot failed"));
    }

    #[tokio::test]
    async fn test_keystroke_fuzzing() {
        let mut mock = MockVcsFacade::new();
        // Setup mock to return some data to avoid crashes in UI
        mock.expect_workspace_root()
            .returning(|| std::path::PathBuf::from("/tmp"));
        mock.expect_get_operation_log()
            .returning(|_, _, _| {
                Ok(crate::domain::models::RepoStatus {
                    operation_id: "test".to_string(),
                    workspace_id: "default".to_string(),
                    working_copy_id: crate::domain::models::CommitId("wc".to_string()),
                    graph: vec![crate::domain::models::GraphRow {
                        commit_id: crate::domain::models::CommitId("wc".to_string()),
                        change_id: "wc".to_string(),
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
