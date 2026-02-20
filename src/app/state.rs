use super::action::Action;
use crate::domain::models::{CommitId, RepoStatus};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{TableState, Widget};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use tui_textarea::{CursorMove, TextArea};
use std::time::Instant;
use tui_textarea::TextArea;

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
}

#[derive(Debug, Clone)]
pub struct ContextMenuState {
    pub commit_id: CommitId,
    pub x: u16,
    pub y: u16,
    pub selected_index: usize,
    pub actions: Vec<(String, Action)>,
}

pub struct AppTextArea<'a>(pub TextArea<'a>);

impl<'a> Default for AppTextArea<'a> {
    fn default() -> Self {
        Self(TextArea::default())
    }
}

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

#[derive(Debug, Clone, PartialEq)]
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
    pub show_diffs: bool,

    // --- Input Handling ---
    pub text_area: AppTextArea<'a>,

    // --- Click Tracking ---
    pub last_click_time: Option<Instant>,
    pub last_click_pos: Option<(u16, u16)>,

    // --- Context Menu ---
    pub context_menu: Option<ContextMenuState>,

    // --- Animation ---
    pub frame_count: u64,
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
            text_area: AppTextArea::default(),
            show_diffs: false,
            last_click_time: None,
            last_click_pos: None,
            context_menu: None,
            frame_count: 0,
        }
    }
}
