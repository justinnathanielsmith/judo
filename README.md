# Judo

A TUI for [Jujutsu](https://github.com/martinvonz/jj), built with Rust and Ratatui.

## Features

- **Revision Graph**: Visualize your commit history.
- **Diff View**: Inspect changes between revisions.
- **Snapshotting**: Create snapshots of your current state.
- **Describe**: Modify revision descriptions (WIP).

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

| Key       | Action                   |
| --------- | ------------------------ |
| `j` / `↓` | Select next revision     |
| `k` / `↑` | Select previous revision |
| `q`       | Quit the application     |

## Architecture

Judo implements The Elm Architecture (TEA) pattern:
- **Model**: `AppState` (Project state, selection, UI mode)
- **View**: Renders UI based on `AppState` (Graph, Diff, Status)
- **Update**: `reducer` handles `Action`s to modify `AppState`

Powered by:
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI
- [jj-lib](https://github.com/martinvonz/jj) - Jujutsu library
- [tokio](https://tokio.rs/) - Async runtime
