use crate::domain::models::{CommitId, RepoStatus};
use ratatui::widgets::TableState;
use std::collections::HashMap;
use tui_textarea::TextArea;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,       // Navigating the log
    Command,      // Typing a command like ":q" or ":new"
    SquashSelect, // Selecting a target to squash into
    BookmarkInput, // Inputting a bookmark name
    Input,        // A generic text input modal (e.g., for commit messages)
    Loading,      // Blocking interaction (optional, often better handled with a flag)
}

// Cannot derive Debug/Clone/PartialEq easily because TextArea doesn't support them all or is heavy
// So we wrap TextArea in a struct or just keep it in AppState and implement Debug manually if needed.
// For now, we will assume standard usage.
pub struct AppState<'a> {
    // --- Connectivity & Status ---
    pub should_quit: bool,
    pub mode: AppMode,
    pub last_error: Option<String>,
    pub status_message: Option<String>, // "Snapshot created."

    // --- JJ Data (The "Source of Truth") ---
    // We wrap this in Option because we might start before the repo is loaded.
    pub repo: Option<RepoStatus>,

    // --- UI State (Selection, Scroll) ---
    // We keep this separate so it persists even if `repo` data refreshes.
    pub log_list_state: TableState,

    // --- Derived/Cached Data ---
    // Data fetched lazily based on selection (the "Debounced" content)
    pub current_diff: Option<String>,
    pub is_loading_diff: bool,
    pub diff_scroll: u16,
    pub diff_cache: HashMap<CommitId, String>,

    // --- Input Handling ---
    pub text_area: TextArea<'a>,
}

impl<'a> Default for AppState<'a> {
    fn default() -> Self {
        Self {
            should_quit: false,
            mode: AppMode::Normal,
            last_error: None,
            status_message: None,
            repo: None,
            log_list_state: TableState::default(),
            current_diff: None,
            is_loading_diff: false,
            diff_scroll: 0,
            diff_cache: HashMap::new(),
            text_area: TextArea::default(),
        }
    }
}
