use super::action::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyConfig {
    pub profile: String,
    pub custom: Option<HashMap<String, String>>,
}

impl KeyConfig {
    pub fn load() -> Self {
        if let Some(mut config_path) = home::home_dir() {
            config_path.push(".config");
            config_path.push("judo");
            config_path.push("config.toml");

            if config_path.exists() {
                if let Ok(content) = std::fs::read_to_string(config_path) {
                    if let Ok(config) = toml::from_str(&content) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            profile: "vim".to_string(),
            custom: None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct KeyMap {
    pub global: HashMap<KeyEvent, Action>,
    pub diff_mode: HashMap<KeyEvent, Action>,
}

impl KeyMap {
    pub fn from_config(config: &KeyConfig) -> Self {
        let mut map = Self::vim_profile();

        if let Some(custom) = &config.custom {
            for (key_str, action_str) in custom {
                if let (Some(key), Some(action)) = (parse_key(key_str), parse_action(action_str)) {
                    map.global.insert(key, action);
                }
            }
        }

        map
    }

    fn vim_profile() -> Self {
        let mut global = HashMap::new();
        let mut diff_mode = HashMap::new();

        global.insert(key_char('q'), Action::Quit);
        global.insert(key_code(KeyCode::Enter), Action::ToggleDiffs);
        global.insert(key_code(KeyCode::Tab), Action::FocusDiff);
        global.insert(key_char('l'), Action::FocusDiff);
        global.insert(key_char('j'), Action::SelectNext);
        global.insert(key_code(KeyCode::Down), Action::SelectNext);
        global.insert(key_char('k'), Action::SelectPrev);
        global.insert(key_code(KeyCode::Up), Action::SelectPrev);
        global.insert(key_char('s'), Action::SnapshotWorkingCopy);
        global.insert(key_char('S'), Action::EnterSquashMode);
        global.insert(
            key_char('e'),
            Action::EditRevision(crate::domain::models::CommitId("".to_string())),
        );
        global.insert(
            key_char('n'),
            Action::NewRevision(crate::domain::models::CommitId("".to_string())),
        );
        global.insert(
            key_char('a'),
            Action::AbandonRevision(crate::domain::models::CommitId("".to_string())),
        );
        global.insert(key_char('b'), Action::SetBookmarkIntent);
        global.insert(key_char('B'), Action::DeleteBookmark("".to_string()));
        global.insert(key_char('d'), Action::DescribeRevisionIntent);
        global.insert(key_char('m'), Action::FilterMine);
        global.insert(key_char('t'), Action::FilterTrunk);
        global.insert(key_char('c'), Action::FilterConflicts);
        global.insert(key_char('u'), Action::Undo);
        global.insert(key_char('U'), Action::Redo);
        global.insert(key_char('f'), Action::Fetch);
        global.insert(key_char('/'), Action::EnterFilterMode);
        global.insert(key_char('p'), Action::PushIntent);
        global.insert(key_char('?'), Action::ToggleHelp);
        global.insert(key_code(KeyCode::PageDown), Action::ScrollDiffDown(10));
        global.insert(key_code(KeyCode::PageUp), Action::ScrollDiffUp(10));
        global.insert(key_char('['), Action::PrevHunk);
        global.insert(key_char(']'), Action::NextHunk);
        global.insert(key_code(KeyCode::Esc), Action::CancelMode);

        diff_mode.insert(key_char('h'), Action::FocusGraph);
        diff_mode.insert(key_code(KeyCode::Tab), Action::FocusGraph);
        diff_mode.insert(key_char('j'), Action::SelectNextFile);
        diff_mode.insert(key_code(KeyCode::Down), Action::SelectNextFile);
        diff_mode.insert(key_char('k'), Action::SelectPrevFile);
        diff_mode.insert(key_code(KeyCode::Up), Action::SelectPrevFile);
        diff_mode.insert(key_code(KeyCode::PageDown), Action::ScrollDiffDown(10));
        diff_mode.insert(key_code(KeyCode::PageUp), Action::ScrollDiffUp(10));
        diff_mode.insert(key_char('['), Action::PrevHunk);
        diff_mode.insert(key_char(']'), Action::NextHunk);
        diff_mode.insert(key_code(KeyCode::Esc), Action::CancelMode);

        Self { global, diff_mode }
    }

    pub fn get_action(&self, event: KeyEvent, mode: super::state::AppMode) -> Option<Action> {
        if mode == super::state::AppMode::Diff {
            if let Some(action) = self.diff_mode.get(&event) {
                return Some(action.clone());
            }
        }
        self.global.get(&event).cloned()
    }
}

fn key_code(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn key_char(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty())
}

fn parse_key(s: &str) -> Option<KeyEvent> {
    match s.to_lowercase().as_str() {
        "enter" => Some(key_code(KeyCode::Enter)),
        "tab" => Some(key_code(KeyCode::Tab)),
        "esc" => Some(key_code(KeyCode::Esc)),
        "up" => Some(key_code(KeyCode::Up)),
        "down" => Some(key_code(KeyCode::Down)),
        "left" => Some(key_code(KeyCode::Left)),
        "right" => Some(key_code(KeyCode::Right)),
        "pgup" | "pageup" => Some(key_code(KeyCode::PageUp)),
        "pgdn" | "pagedown" => Some(key_code(KeyCode::PageDown)),
        s if s.len() == 1 => Some(key_char(s.chars().next().unwrap())),
        _ => None,
    }
}

fn parse_action(s: &str) -> Option<Action> {
    match s.to_lowercase().as_str() {
        "quit" => Some(Action::Quit),
        "togglediffs" => Some(Action::ToggleDiffs),
        "focusdiff" => Some(Action::FocusDiff),
        "focusgraph" => Some(Action::FocusGraph),
        "selectnext" => Some(Action::SelectNext),
        "selectprev" => Some(Action::SelectPrev),
        "selectnextfile" => Some(Action::SelectNextFile),
        "selectprevfile" => Some(Action::SelectPrevFile),
        "snapshot" => Some(Action::SnapshotWorkingCopy),
        "edit" => Some(Action::EditRevision(crate::domain::models::CommitId(
            "".to_string(),
        ))),
        "new" => Some(Action::NewRevision(crate::domain::models::CommitId(
            "".to_string(),
        ))),
        "describe" => Some(Action::DescribeRevisionIntent),
        "abandon" => Some(Action::AbandonRevision(crate::domain::models::CommitId(
            "".to_string(),
        ))),
        "setbookmark" => Some(Action::SetBookmarkIntent),
        "deletebookmark" => Some(Action::DeleteBookmark("".to_string())),
        "undo" => Some(Action::Undo),
        "redo" => Some(Action::Redo),
        "fetch" => Some(Action::Fetch),
        "push" => Some(Action::PushIntent),
        "filter" => Some(Action::EnterFilterMode),
        "help" => Some(Action::ToggleHelp),
        "nexthunk" => Some(Action::NextHunk),
        "prevhunk" => Some(Action::PrevHunk),
        "cancel" => Some(Action::CancelMode),
        _ => None,
    }
}
