use super::action::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyConfig {
    pub profile: String,
    pub custom: Option<HashMap<String, String>>,
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            profile: "vim".to_string(),
            custom: None,
        }
    }
}

pub struct KeyMap {
    // Maps Mode -> (Key -> Action)
    // For simplicity, let's start with a global map and override by mode if needed.
    pub global: HashMap<KeyEvent, Action>,
    pub diff_mode: HashMap<KeyEvent, Action>,
}

impl KeyMap {
    pub fn from_config(_config: &KeyConfig) -> Self {
        // In a real app, we would load from a file or use the profile.
        // For now, let's implement the default "vim" profile.
        let mut global = HashMap::new();
        let mut diff_mode = HashMap::new();

        // --- Global / Normal Mode ---
        global.insert(key('q'), Action::Quit);
        global.insert(key(KeyCode::Enter), Action::ToggleDiffs);
        global.insert(key(KeyCode::Tab), Action::FocusDiff);
        global.insert(key('l'), Action::FocusDiff);
        global.insert(key('j'), Action::SelectNext);
        global.insert(key(KeyCode::Down), Action::SelectNext);
        global.insert(key('k'), Action::SelectPrev);
        global.insert(key(KeyCode::Up), Action::SelectPrev);
        global.insert(key('s'), Action::SnapshotWorkingCopy);
        global.insert(key('S'), Action::EnterSquashMode);
        global.insert(key('e'), Action::EditRevision(crate::domain::models::CommitId("".to_string()))); // Placeholder
        global.insert(key('n'), Action::NewRevision(crate::domain::models::CommitId("".to_string()))); // Placeholder
        global.insert(key('a'), Action::AbandonRevision(crate::domain::models::CommitId("".to_string()))); // Placeholder
        global.insert(key('b'), Action::SetBookmarkIntent);
        global.insert(key('B'), Action::DeleteBookmark("".to_string())); // Placeholder
        global.insert(key('d'), Action::DescribeRevisionIntent);
        global.insert(key('m'), Action::FilterMine);
        global.insert(key('t'), Action::FilterTrunk);
        global.insert(key('c'), Action::FilterConflicts);
        global.insert(key('u'), Action::Undo);
        global.insert(key('U'), Action::Redo);
        global.insert(key('f'), Action::Fetch);
        global.insert(key('/'), Action::EnterFilterMode);
        global.insert(key('p'), Action::PushIntent);
        global.insert(key('?'), Action::ToggleHelp);
        global.insert(key(KeyCode::PageDown), Action::ScrollDiffDown(10));
        global.insert(key(KeyCode::PageUp), Action::ScrollDiffUp(10));
        global.insert(key('['), Action::PrevHunk);
        global.insert(key(']'), Action::NextHunk);
        global.insert(key(KeyCode::Esc), Action::CancelMode);

        // --- Diff Mode Overrides ---
        diff_mode.insert(key('h'), Action::FocusGraph);
        diff_mode.insert(key(KeyCode::Tab), Action::FocusGraph);
        diff_mode.insert(key('j'), Action::SelectNextFile);
        diff_mode.insert(key(KeyCode::Down), Action::SelectNextFile);
        diff_mode.insert(key('k'), Action::SelectPrevFile);
        diff_mode.insert(key(KeyCode::Up), Action::SelectPrevFile);

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

fn key(code: impl Into<KeyCode>) -> KeyEvent {
    KeyEvent::new(code.into(), KeyModifiers::empty())
}
