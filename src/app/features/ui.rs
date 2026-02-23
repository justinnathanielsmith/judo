use crate::app::{
    action::{Action, UpdateResult},
    state::{AppMode, AppState, CommandPaletteState, EvologState, OperationLogState},
};
use crate::theme::Theme;

pub fn update(state: &mut AppState, action: &Action) -> UpdateResult {
    match action {
        Action::EnterCommandMode => {
            state.mode = AppMode::CommandPalette;
            state.command_palette = Some(CommandPaletteState {
                matches: crate::app::command_palette::search_commands(""),
                ..Default::default()
            });
            UpdateResult::Handled(None)
        }
        Action::CancelMode => {
            state.mode = AppMode::Normal;
            state.input = None;
            state.command_palette = None;
            state.last_error = None;
            state.context_menu = None;
            state.evolog_state = None;
            state.operation_log_state = None;
            state.theme_selection = None;
            state.rebase_sources.clear();
            UpdateResult::Handled(None)
        }
        Action::ToggleHelp => {
            state.mode = if state.mode == AppMode::Help {
                AppMode::Normal
            } else {
                AppMode::Help
            };
            UpdateResult::Handled(None)
        }
        Action::EnterThemeSelection => {
            state.mode = AppMode::ThemeSelection;
            state.theme_selection = Some(crate::app::state::ThemeSelectionState::default());
            UpdateResult::Handled(None)
        }
        Action::SwitchTheme(palette) => {
            state.palette_type = *palette;
            state.theme = Theme::from_palette_type(*palette);
            state.mode = AppMode::Normal;
            UpdateResult::Handled(None)
        }
        Action::TextAreaInput(key) => {
            if let Some(input) = &mut state.input {
                input.text_area.input(*key);
            } else if state.mode == AppMode::CommandPalette {
                if let Some(cp) = &mut state.command_palette {
                    use crossterm::event::KeyCode;
                    match key.code {
                        KeyCode::Char(c) => {
                            cp.query.push(c);
                            cp.matches = crate::app::command_palette::search_commands(&cp.query);
                            cp.selected_index = 0;
                        }
                        KeyCode::Backspace => {
                            cp.query.pop();
                            cp.matches = crate::app::command_palette::search_commands(&cp.query);
                            cp.selected_index = 0;
                        }
                        _ => {}
                    }
                }
            }
            UpdateResult::Handled(None)
        }
        Action::CommandPaletteNext => {
            if let Some(cp) = &mut state.command_palette {
                if !cp.matches.is_empty() {
                    cp.selected_index = (cp.selected_index + 1) % cp.matches.len();
                }
            }
            UpdateResult::Handled(None)
        }
        Action::CommandPalettePrev => {
            if let Some(cp) = &mut state.command_palette {
                if !cp.matches.is_empty() {
                    if cp.selected_index == 0 {
                        cp.selected_index = cp.matches.len() - 1;
                    } else {
                        cp.selected_index -= 1;
                    }
                }
            }
            UpdateResult::Handled(None)
        }
        Action::SelectThemeNext => {
            if let Some(ts) = &mut state.theme_selection {
                ts.selected_index = (ts.selected_index + 1) % ts.themes.len();
            }
            UpdateResult::Handled(None)
        }
        Action::SelectThemePrev => {
            if let Some(ts) = &mut state.theme_selection {
                if ts.selected_index == 0 {
                    ts.selected_index = ts.themes.len() - 1;
                } else {
                    ts.selected_index -= 1;
                }
            }
            UpdateResult::Handled(None)
        }
        Action::CommandPaletteSelect => {
            if state.mode == AppMode::ThemeSelection {
                if let Some(ts) = &state.theme_selection {
                    if let Some(palette) = ts.themes.get(ts.selected_index) {
                        state.palette_type = *palette;
                        state.theme = Theme::from_palette_type(*palette);
                        state.mode = AppMode::Normal;
                        state.theme_selection = None;
                    }
                }
                return UpdateResult::Handled(None);
            }
            UpdateResult::NotHandled
        }
        Action::OpenEvolog(content) => {
            state.mode = AppMode::Evolog;
            state.evolog_state = Some(EvologState {
                content: content.lines().map(|s| s.to_string()).collect(),
                scroll: 0,
            });
            UpdateResult::Handled(None)
        }
        Action::CloseEvolog => {
            state.mode = AppMode::Normal;
            state.evolog_state = None;
            UpdateResult::Handled(None)
        }
        Action::ScrollEvologUp(n) => {
            if let Some(ev) = &mut state.evolog_state {
                ev.scroll = ev.scroll.saturating_sub(*n);
            }
            UpdateResult::Handled(None)
        }
        Action::ScrollEvologDown(n) => {
            if let Some(ev) = &mut state.evolog_state {
                let max_scroll = ev.content.len().saturating_sub(1) as u16;
                ev.scroll = ev.scroll.saturating_add(*n).min(max_scroll);
            }
            UpdateResult::Handled(None)
        }
        Action::OpenOperationLog(content) => {
            state.mode = AppMode::OperationLog;
            state.operation_log_state = Some(OperationLogState {
                content: content.lines().map(|s| s.to_string()).collect(),
                scroll: 0,
            });
            UpdateResult::Handled(None)
        }
        Action::CloseOperationLog => {
            state.mode = AppMode::Normal;
            state.operation_log_state = None;
            UpdateResult::Handled(None)
        }
        Action::ScrollOperationLogUp(n) => {
            if let Some(op) = &mut state.operation_log_state {
                op.scroll = op.scroll.saturating_sub(*n);
            }
            UpdateResult::Handled(None)
        }
        Action::ScrollOperationLogDown(n) => {
            if let Some(op) = &mut state.operation_log_state {
                let max_scroll = op.content.len().saturating_sub(1) as u16;
                op.scroll = op.scroll.saturating_add(*n).min(max_scroll);
            }
            UpdateResult::Handled(None)
        }
        Action::SelectContextMenuNext => {
            if let Some(menu) = &mut state.context_menu {
                menu.selected_index = (menu.selected_index + 1) % menu.actions.len();
            }
            UpdateResult::Handled(None)
        }
        Action::SelectContextMenuPrev => {
            if let Some(menu) = &mut state.context_menu {
                if menu.selected_index == 0 {
                    menu.selected_index = menu.actions.len() - 1;
                } else {
                    menu.selected_index -= 1;
                }
            }
            UpdateResult::Handled(None)
        }
        Action::CloseContextMenu => {
            state.context_menu = None;
            state.mode = AppMode::Normal;
            UpdateResult::Handled(None)
        }
        Action::OpenContextMenu(commit_id_opt, (x, y)) => {
            if let Some(commit_id) = commit_id_opt {
                state.context_menu = Some(crate::app::state::ContextMenuState {
                    commit_id: commit_id.clone(),
                    x: *x,
                    y: *y,
                    selected_index: 0,
                    actions: vec![
                        ("Edit".to_string(), Action::EditRevision(Some(commit_id.clone()))),
                        ("New Child".to_string(), Action::NewRevision(Some(commit_id.clone()))),
                        ("Abandon".to_string(), Action::AbandonRevision(Some(commit_id.clone()))),
                        ("Duplicate".to_string(), Action::DuplicateRevision),
                        ("Squash".to_string(), Action::EnterSquashMode),
                        ("Evolog".to_string(), Action::EvologRevision(Some(commit_id.clone()))),
                    ],
                });
                state.mode = AppMode::ContextMenu;
            }
            UpdateResult::Handled(None)
        }
        _ => UpdateResult::NotHandled,
    }
}
