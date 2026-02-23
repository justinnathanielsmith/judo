use crate::app::{
    action::{Action, UpdateResult},
    command::Command,
    state::{AppMode, AppState, AppTextArea},
};

pub fn update(state: &mut AppState, action: &Action) -> UpdateResult {
    match action {
        Action::EnterFilterMode => {
            state.mode = AppMode::FilterInput;
            state.is_selecting_presets = false;
            state.input = Some(crate::app::state::InputState {
                text_area: AppTextArea::default(),
            });
            if let Some(revset) = &state.revset {
                if let Some(input) = &mut state.input {
                    input.text_area.insert_str(revset);
                }
            }
            state.selected_filter_index = None;
            UpdateResult::Handled(None)
        }
        Action::ApplyFilter(filter) => {
            state.mode = AppMode::Normal;
            state.input = None;
            state.selected_filter_index = None;

            let filter_str = filter.trim().to_string();
            if filter_str.is_empty() {
                state.revset = None;
            } else {
                state.revset = Some(filter_str.clone());
                // Add to recent filters if not already there
                if let Some(pos) = state.recent_filters.iter().position(|f| f == &filter_str) {
                    state.recent_filters.remove(pos);
                }
                state.recent_filters.insert(0, filter_str);
                state.recent_filters.truncate(10);
                super::super::persistence::save_recent_filters(&state.recent_filters);
            }
            UpdateResult::Handled(Some(Command::LoadRepo(None, 100, state.revset.clone())))
        }
        Action::ClearFilter => {
            state.revset = None;
            state.selected_filter_index = None;
            UpdateResult::Handled(Some(Command::LoadRepo(None, 100, None)))
        }
        Action::FilterMine => UpdateResult::Handled(apply_quick_filter(state, "mine()")),
        Action::FilterTrunk => UpdateResult::Handled(apply_quick_filter(state, "trunk()")),
        Action::FilterConflicts => UpdateResult::Handled(apply_quick_filter(state, "conflicts()")),
        Action::FilterAll => UpdateResult::Handled(apply_quick_filter(state, "all()")),
        Action::FilterHeads => UpdateResult::Handled(apply_quick_filter(state, "heads(all())")),
        Action::FilterBookmarks => UpdateResult::Handled(apply_quick_filter(state, "bookmarks()")),
        Action::FilterImmutable => UpdateResult::Handled(apply_quick_filter(state, "immutable()")),
        Action::FilterMutable => UpdateResult::Handled(apply_quick_filter(state, "mutable()")),
        Action::FilterEmpty => UpdateResult::Handled(apply_quick_filter(state, "empty()")),
        Action::FilterDivergent => UpdateResult::Handled(apply_quick_filter(state, "divergent()")),
        Action::FilterMerges => UpdateResult::Handled(apply_quick_filter(state, "merges()")),
        Action::FilterTags => UpdateResult::Handled(apply_quick_filter(state, "tags()")),
        Action::FilterRemoteBookmarks => UpdateResult::Handled(apply_quick_filter(state, "remote_bookmarks()")),
        Action::FilterWorking => UpdateResult::Handled(apply_quick_filter(state, "working_copies()")),
        Action::FilterNext => {
            if state.mode == AppMode::FilterInput {
                let filters = if state.is_selecting_presets {
                    &state.preset_filters
                } else {
                    &state.recent_filters
                };

                if !filters.is_empty() {
                    let next = match state.selected_filter_index {
                        Some(i) => (i + 1) % filters.len(),
                        None => 0,
                    };
                    state.selected_filter_index = Some(next);
                    if let Some(input) = &mut state.input {
                        input.text_area = AppTextArea::default();
                        input.text_area.insert_str(&filters[next]);
                    }
                }
            }
            UpdateResult::Handled(None)
        }
        Action::FilterPrev => {
            if state.mode == AppMode::FilterInput {
                let filters = if state.is_selecting_presets {
                    &state.preset_filters
                } else {
                    &state.recent_filters
                };

                if !filters.is_empty() {
                    let next = match state.selected_filter_index {
                        Some(i) => {
                            if i == 0 {
                                filters.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => filters.len() - 1,
                    };
                    state.selected_filter_index = Some(next);
                    if let Some(input) = &mut state.input {
                        input.text_area = AppTextArea::default();
                        input.text_area.insert_str(&filters[next]);
                    }
                }
            }
            UpdateResult::Handled(None)
        }
        Action::ToggleFilterSource => {
            if state.mode == AppMode::FilterInput {
                state.is_selecting_presets = !state.is_selecting_presets;
                state.selected_filter_index = Some(0);
                let filters = if state.is_selecting_presets {
                    &state.preset_filters
                } else {
                    &state.recent_filters
                };
                if !filters.is_empty() {
                    if let Some(input) = &mut state.input {
                        input.text_area = AppTextArea::default();
                        input.text_area.insert_str(&filters[0]);
                    }
                }
            }
            UpdateResult::Handled(None)
        }
        _ => UpdateResult::NotHandled,
    }
}

fn apply_quick_filter(state: &mut AppState, filter: &str) -> Option<Command> {
    state.revset = Some(filter.to_string());
    state.mode = AppMode::Normal;
    Some(Command::LoadRepo(None, 100, state.revset.clone()))
}
