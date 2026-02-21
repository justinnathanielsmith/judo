use super::action::Action;
use super::keymap::{KeyConfig, KeyMap};
use crate::domain::models::{CommitId, RepoStatus};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{TableState, Widget};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Instant;
use tui_textarea::{CursorMove, TextArea};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Panel {
    Graph,
    Diff,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AppMode {
    Normal,        // Navigating the log
    Command,       // Typing a command like ":q" or ":new"
    SquashSelect,  // Selecting a target to squash into
    BookmarkInput, // Inputting a bookmark name
    Input,         // A generic text input modal (e.g., for commit messages)
    Loading,       // Blocking interaction (optional, often better handled with a flag)
    Diff,          // Focusing the diff window for scrolling
    ContextMenu,   // Right-click menu for actions
    FilterInput,   // Inputting a revset filter
    Help,          // Showing the help overlay
    NoRepo,        // No repository found, showing welcome screen
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenuState {
    pub commit_id: CommitId,
    pub x: u16,
    pub y: u16,
    pub selected_index: usize,
    pub actions: Vec<(String, Action)>,
}

impl ContextMenuState {
    pub fn calculate_rect(&self, terminal_area: Rect) -> Rect {
        let longest_action = self
            .actions
            .iter()
            .map(|(name, _)| name.len())
            .max()
            .unwrap_or(0) as u16;
        let menu_width = (longest_action + 6).min(terminal_area.width);
        let menu_height = (self.actions.len() as u16 + 2).min(terminal_area.height);

        let mut x = self.x;
        let mut y = self.y;

        if x + menu_width > terminal_area.width {
            x = terminal_area.width.saturating_sub(menu_width);
        }

        if y + menu_height > terminal_area.height {
            y = y.saturating_sub(menu_height);
        }

        Rect::new(x, y, menu_width, menu_height)
    }
}

#[derive(Default)]
pub struct AppTextArea<'a>(pub TextArea<'a>);

impl<'a> Clone for AppTextArea<'a> {
    fn clone(&self) -> Self {
        let mut area = TextArea::new(self.0.lines().iter().map(|s| s.to_string()).collect());
        let (row, col) = self.0.cursor();
        area.move_cursor(CursorMove::Jump(row as u16, col as u16));
        Self(area)
    }
}

impl<'a> std::fmt::Debug for AppTextArea<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppTextArea")
            .field("lines", &self.0.lines())
            .field("cursor", &self.0.cursor())
            .finish()
    }
}

impl<'a> PartialEq for AppTextArea<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.0.lines() == other.0.lines() && self.0.cursor() == other.0.cursor()
    }
}

impl<'a> Deref for AppTextArea<'a> {
    type Target = TextArea<'a>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for AppTextArea<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> Widget for &AppTextArea<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Widget::render(&self.0, area, buf);
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct HeaderState {
    pub op_id: String,
    pub wc_info: String,
    pub stats: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppState<'a> {
    // --- Connectivity & Status ---
    pub should_quit: bool,
    pub mode: AppMode,
    pub last_error: Option<String>,
    pub status_message: Option<String>, // "Snapshot created."
    pub status_clear_time: Option<Instant>,
    pub workspace_id: String,
    pub active_tasks: Vec<String>,

    // --- JJ Data (The "Source of Truth") ---
    // We wrap this in Option because we might start before the repo is loaded.
    pub repo: Option<RepoStatus>,
    pub revset: Option<String>,
    pub is_loading_more: bool,
    pub has_more: bool,

    // --- UI State (Selection, Scroll) ---
    // We keep this separate so it persists even if `repo` data refreshes.
    pub log_list_state: TableState,
    pub selected_file_index: Option<usize>,

    // --- Derived/Cached Data ---
    // Data fetched lazily based on selection (the "Debounced" content)
    pub current_diff: Option<String>,
    pub is_loading_diff: bool,
    pub diff_scroll: u16,
    pub diff_cache: HashMap<CommitId, String>,
    pub show_diffs: bool,
    pub header_state: HeaderState,
    pub spinner: String,

    // --- Input Handling ---
    pub text_area: AppTextArea<'a>,

    // --- Click Tracking ---
    pub last_click_time: Option<Instant>,
    pub last_click_pos: Option<(u16, u16)>,

    // --- Context Menu ---
    pub context_menu: Option<ContextMenuState>,

    // --- Animation ---
    pub frame_count: u64,
    pub hunk_highlight_time: Option<Instant>,

    // --- Layout ---
    pub diff_ratio: u16,
    pub focused_panel: Panel,

    // --- Config ---
    pub keymap: Arc<KeyMap>,
}

impl<'a> AppState<'a> {
    pub fn new(config: KeyConfig) -> Self {
        Self {
            keymap: Arc::new(KeyMap::from_config(&config)),
            ..Default::default()
        }
    }
}

impl<'a> Default for AppState<'a> {
    fn default() -> Self {
        Self {
            should_quit: false,
            mode: AppMode::Normal,
            last_error: None,
            status_message: None,
            status_clear_time: None,
            workspace_id: "".to_string(),
            active_tasks: Vec::new(),
            repo: None,
            revset: None,
            is_loading_more: false,
            has_more: true,
            log_list_state: TableState::default(),
            selected_file_index: None,
            current_diff: None,
            is_loading_diff: false,
            diff_scroll: 0,
            diff_cache: HashMap::new(),
            show_diffs: false,
            header_state: HeaderState::default(),
            spinner: "⠋".to_string(),
            text_area: AppTextArea::default(),
            last_click_time: None,
            last_click_pos: None,
            context_menu: None,
            frame_count: 0,
            hunk_highlight_time: None,
            diff_ratio: 50,
            focused_panel: Panel::Graph,
            keymap: Arc::new(KeyMap::from_config(&KeyConfig::default())),
        }
    }
}
