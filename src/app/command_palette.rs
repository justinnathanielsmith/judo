use super::action::Action;

#[derive(Debug, Clone)]
pub struct CommandDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub action: Action,
}

#[must_use]
pub fn get_commands() -> Vec<CommandDefinition> {
    vec![
        CommandDefinition {
            name: "Snapshot",
            description: "Snapshot the working copy",
            action: Action::SnapshotWorkingCopy,
        },
        CommandDefinition {
            name: "Describe",
            description: "Update the revision description",
            action: Action::DescribeRevisionIntent,
        },
        CommandDefinition {
            name: "New Child",
            description: "Create a new child revision",
            action: Action::NewRevision(crate::domain::models::CommitId(String::new())),
        },
        CommandDefinition {
            name: "Edit",
            description: "Edit the selected revision",
            action: Action::EditRevision(crate::domain::models::CommitId(String::new())),
        },
        CommandDefinition {
            name: "Abandon",
            description: "Abandon the selected revision",
            action: Action::AbandonRevision(crate::domain::models::CommitId(String::new())),
        },
        CommandDefinition {
            name: "Squash",
            description: "Squash revision into parent",
            action: Action::SquashRevision(crate::domain::models::CommitId(String::new())),
        },
        CommandDefinition {
            name: "Set Bookmark",
            description: "Set a bookmark on the selected revision",
            action: Action::SetBookmarkIntent,
        },
        CommandDefinition {
            name: "Delete Bookmark",
            description: "Delete a bookmark",
            action: Action::DeleteBookmarkIntent,
        },
        CommandDefinition {
            name: "Undo",
            description: "Undo the last operation",
            action: Action::Undo,
        },
        CommandDefinition {
            name: "Redo",
            description: "Redo the last operation",
            action: Action::Redo,
        },
        CommandDefinition {
            name: "Fetch",
            description: "Fetch from the remote",
            action: Action::Fetch,
        },
        CommandDefinition {
            name: "Push",
            description: "Push to the remote",
            action: Action::PushIntent,
        },
        CommandDefinition {
            name: "Filter: Mine",
            description: "Show only your revisions",
            action: Action::FilterMine,
        },
        CommandDefinition {
            name: "Filter: Trunk",
            description: "Show revisions in the trunk",
            action: Action::FilterTrunk,
        },
        CommandDefinition {
            name: "Filter: Conflicts",
            description: "Show revisions with conflicts",
            action: Action::FilterConflicts,
        },
        CommandDefinition {
            name: "Filter: Custom",
            description: "Enter a custom revset filter",
            action: Action::EnterFilterMode,
        },
        CommandDefinition {
            name: "Toggle Diffs",
            description: "Toggle the diff panel",
            action: Action::ToggleDiffs,
        },
        CommandDefinition {
            name: "Help",
            description: "Show the help overlay",
            action: Action::ToggleHelp,
        },
        CommandDefinition {
            name: "Quit",
            description: "Quit Judo",
            action: Action::Quit,
        },
    ]
}

#[must_use]
pub fn search_commands(query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..get_commands().len()).collect();
    }

    let query_lower = query.to_lowercase();
    let commands = get_commands();
    let mut results = Vec::new();

    // First pass: exact substring match in name (higher priority)
    for (i, cmd) in commands.iter().enumerate() {
        if cmd.name.to_lowercase().contains(&query_lower) {
            results.push(i);
        }
    }

    // Second pass: exact substring match in description (lower priority)
    for (i, cmd) in commands.iter().enumerate() {
        if !results.contains(&i) && cmd.description.to_lowercase().contains(&query_lower) {
            results.push(i);
        }
    }

    results
}
