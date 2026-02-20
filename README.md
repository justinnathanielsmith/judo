# Judo

A TUI for [Jujutsu](https://github.com/martinvonz/jj), built with Rust and Ratatui.

## Features

- **Revision Graph**: Visualize your commit history with an interactive graph.
- **Diff View**: Inspect changes between revisions with hunk navigation.
- **Snapshotting**: Quickly create snapshots of your current working copy.
- **Operations**:
  - **Edit**: Move the working copy to a specific revision.
  - **New**: Create a new child revision.
  - **Describe**: Modify revision descriptions directly from the TUI.
  - **Abandon**: Discard revisions you no longer need.
  - **Undo/Redo**: Seamlessly navigate through your operation history.

## Installation

Ensure you have [Rust](https://www.rust-lang.org/tools/install) installed.

```bash
# Clone the repository
git clone https://github.com/yourusername/judo.git
cd judo

# Run the TUI
cargo run
```

## Usage

### Navigation

| Key           | Action                   |
| ------------- | ------------------------ |
| `j` / `↓`     | Select next revision     |
| `k` / `↑`     | Select previous revision |
| `PgDn`        | Scroll diff down         |
| `PgUp`        | Scroll diff up           |
| `[`           | Jump to previous hunk    |
| `]`           | Jump to next hunk        |
| `q`           | Quit the application     |

### Commands

| Key | Action                                     |
| --- | ------------------------------------------ |
| `s` | Snapshot working copy                      |
| `e` | Edit selected revision                     |
| `n` | Create new child from selected revision    |
| `d` | Describe selected revision (opens input)   |
| `a` | Abandon selected revision                  |
| `u` | Undo last operation                        |
| `U` | Redo last operation                        |
| `Esc` | Cancel current mode / Clear errors       |

## Architecture

Judo implements The Elm Architecture (TEA) pattern for a robust and predictable UI state:

- **Model**: `AppState` (Project state, selection, UI mode)
- **View**: Renders UI based on `AppState` (Graph, Diff, Status)
- **Update**: `reducer` handles `Action`s to modify `AppState`, and may return `Command`s for side effects.
- **Commands**: Handled asynchronously via `VcsFacade` (currently implemented for Jujutsu).

Powered by:
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI
- [jj-lib](https://github.com/martinvonz/jj) - Jujutsu library
- [tokio](https://tokio.rs/) - Async runtime
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal manipulation
