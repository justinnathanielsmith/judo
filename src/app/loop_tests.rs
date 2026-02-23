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
                has_conflict: false,
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
