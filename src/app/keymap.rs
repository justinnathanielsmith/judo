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
    #[must_use]
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
    #[must_use]
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
            Action::EditRevision(crate::domain::models::CommitId(String::new())),
        );
        global.insert(
            key_char('n'),
            Action::NewRevision(crate::domain::models::CommitId(String::new())),
        );
        global.insert(
            key_char('a'),
            Action::AbandonRevision(crate::domain::models::CommitId(String::new())),
        );
        global.insert(key_char('b'), Action::SetBookmarkIntent);
        global.insert(key_char('B'), Action::DeleteBookmarkIntent);
        global.insert(key_char('d'), Action::DescribeRevisionIntent);
        global.insert(key_char('m'), Action::FilterMine);
        global.insert(
            key_char('x'),
            Action::ToggleSelection(crate::domain::models::CommitId(String::new())),
        );
        global.insert(key_char('t'), Action::FilterTrunk);
        global.insert(key_char('c'), Action::FilterConflicts);
        global.insert(key_char('u'), Action::Undo);
        global.insert(key_char('U'), Action::Redo);
        global.insert(key_char('f'), Action::Fetch);
        global.insert(key_char('/'), Action::EnterFilterMode);
        global.insert(key_char('p'), Action::PushIntent);
        global.insert(key_char('?'), Action::ToggleHelp);
        global.insert(key_char('T'), Action::EnterThemeSelection);
        global.insert(key_char('r'), Action::RebaseRevisionIntent);
        global.insert(
            key_char('v'),
            Action::EvologRevision(crate::domain::models::CommitId(String::new())),
        );
        global.insert(key_code(KeyCode::PageDown), Action::ScrollDiffDown(10));
        global.insert(key_code(KeyCode::PageUp), Action::ScrollDiffUp(10));
        global.insert(key_char('['), Action::PrevHunk);
        global.insert(key_char(']'), Action::NextHunk);
        global.insert(key_char(':'), Action::EnterCommandMode);
        global.insert(key_char('C'), Action::ClearFilter);
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

    pub fn get_action(
        &self,
        event: KeyEvent,
        state: &super::state::AppState<'_>,
    ) -> Option<Action> {
        let mode = state.mode;
        if mode == super::state::AppMode::Diff {
            if let Some(action) = self.diff_mode.get(&event) {
                return Some(action.clone());
            }
        } else if mode == super::state::AppMode::CommandPalette {
            return match event.code {
                KeyCode::Esc => Some(Action::CancelMode),
                KeyCode::Enter => Some(Action::CommandPaletteSelect),
                KeyCode::Down => Some(Action::CommandPaletteNext),
                KeyCode::Up => Some(Action::CommandPalettePrev),
                KeyCode::Char('n') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Action::CommandPaletteNext)
                }
                KeyCode::Char('p') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Action::CommandPalettePrev)
                }
                _ => Some(Action::TextAreaInput(event)),
            };
        } else if mode == super::state::AppMode::ThemeSelection {
            return match event.code {
                KeyCode::Esc => Some(Action::CancelMode),
                KeyCode::Char('j') | KeyCode::Down => Some(Action::SelectNext),
                KeyCode::Char('k') | KeyCode::Up => Some(Action::SelectPrev),
                KeyCode::Enter => Some(Action::CommandPaletteSelect),
                _ => None,
            };
        } else if mode == super::state::AppMode::FilterInput {
            return match event.code {
                KeyCode::Esc => Some(Action::CancelMode),
                KeyCode::Enter => {
                    let text = if let Some(input) = &state.input {
                        input.text_area.lines().join("\n")
                    } else {
                        String::new()
                    };
                    Some(Action::ApplyFilter(text))
                }
                KeyCode::Tab => Some(Action::ToggleFilterSource),
                KeyCode::Down => Some(Action::FilterNext),
                KeyCode::Up => Some(Action::FilterPrev),
                KeyCode::Char('n') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Action::FilterNext)
                }
                KeyCode::Char('p') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Action::FilterPrev)
                }
                _ => Some(Action::TextAreaInput(event)),
            };
        } else if mode == super::state::AppMode::RebaseInput {
            return match event.code {
                KeyCode::Esc => Some(Action::CancelMode),
                KeyCode::Enter => {
                    let text = if let Some(input) = &state.input {
                        input.text_area.lines().join("\n")
                    } else {
                        String::new()
                    };
                    if text.trim().is_empty() {
                        return Some(Action::CancelMode);
                    }
                    let ids = state.get_selected_commit_ids();
                    Some(Action::RebaseRevision(ids, text))
                }
                _ => Some(Action::TextAreaInput(event)),
            };
        } else if mode == super::state::AppMode::RebaseSelect {
            return match event.code {
                KeyCode::Esc => Some(Action::CancelMode),
                KeyCode::Enter => {
                    let text = if let Some(input) = &state.input {
                        input.text_area.lines().join("\n")
                    } else {
                        String::new()
                    };
                    if text.trim().is_empty() {
                        return Some(Action::CancelMode);
                    }
                    let ids = state.rebase_sources.clone();
                    Some(Action::RebaseRevision(ids, text))
                }
                _ => Some(Action::TextAreaInput(event)),
            };
        } else if mode == super::state::AppMode::Evolog {
            return match event.code {
                KeyCode::Esc | KeyCode::Char('q') => Some(Action::CloseEvolog),
                KeyCode::Char('j') | KeyCode::Down => Some(Action::ScrollEvologDown(1)),
                KeyCode::Char('k') | KeyCode::Up => Some(Action::ScrollEvologUp(1)),
                KeyCode::PageDown => Some(Action::ScrollEvologDown(10)),
                KeyCode::PageUp => Some(Action::ScrollEvologUp(10)),
                _ => None,
            };
        } else if mode == super::state::AppMode::OperationLog {
            return match event.code {
                KeyCode::Esc | KeyCode::Char('q') => Some(Action::CloseOperationLog),
                KeyCode::Char('j') | KeyCode::Down => Some(Action::ScrollOperationLogDown(1)),
                KeyCode::Char('k') | KeyCode::Up => Some(Action::ScrollOperationLogUp(1)),
                KeyCode::PageDown => Some(Action::ScrollOperationLogDown(10)),
                KeyCode::PageUp => Some(Action::ScrollOperationLogUp(10)),
                _ => None,
            };
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
            String::new(),
        ))),
        "new" => Some(Action::NewRevision(crate::domain::models::CommitId(
            String::new(),
        ))),
        "describe" => Some(Action::DescribeRevisionIntent),
        "commit" => Some(Action::CommitWorkingCopyIntent),
        "abandon" => Some(Action::AbandonRevision(crate::domain::models::CommitId(
            String::new(),
        ))),
        "setbookmark" => Some(Action::SetBookmarkIntent),
        "deletebookmark" => Some(Action::DeleteBookmarkIntent),
        "undo" => Some(Action::Undo),
        "redo" => Some(Action::Redo),
        "fetch" => Some(Action::Fetch),
        "push" => Some(Action::PushIntent),
        "filter" => Some(Action::EnterFilterMode),
        "help" => Some(Action::ToggleHelp),
        "nexthunk" => Some(Action::NextHunk),
        "prevhunk" => Some(Action::PrevHunk),
        "cancel" => Some(Action::CancelMode),
        "filterempty" => Some(Action::FilterEmpty),
        "filterdivergent" => Some(Action::FilterDivergent),
        "filtermerges" => Some(Action::FilterMerges),
        "filtertags" => Some(Action::FilterTags),
        "filterremotebookmarks" => Some(Action::FilterRemoteBookmarks),
        "filterworking" => Some(Action::FilterWorking),
        "clearfilter" => Some(Action::ClearFilter),
        "split" => Some(Action::SplitRevision(crate::domain::models::CommitId(
            String::new(),
        ))),
        "rebase" => Some(Action::RebaseRevisionIntent),
        "evolog" => Some(Action::EvologRevision(crate::domain::models::CommitId(
            String::new(),
        ))),
        "oplog" | "operationlog" => Some(Action::OperationLog),
        _ => None,
    }
}
