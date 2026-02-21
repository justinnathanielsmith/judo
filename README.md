# Judo

**Judo** is a high-performance, terminal-based user interface (TUI) for [Jujutsu (jj)](https://github.com/martinvonz/jj), a Git-compatible version control system. Built with Rust and [Ratatui](https://github.com/ratatui-org/ratatui), Judo provides a fast, intuitive, and interactive way to manage your version control operations.

## Features

- **Interactive Revision Graph**: Visualize your commit history with a navigable graph.
- **Integrated Diff View**: Inspect changes between revisions with hunk-level navigation and conflict resolution.
- **Snapshotting**: Effortlessly create snapshots of your working copy.
- **VCS Operations**:
  - **Edit & New**: Seamlessly move your working copy or create new child revisions.
  - **Describe**: Modify revision descriptions directly within the TUI.
  - **Abandon**: Discard unnecessary revisions.
  - **Undo/Redo**: Navigate through your operation history with ease.
  - **Bookmarks**: Manage bookmarks (set/delete) on any revision.
  - **Fetch & Push**: Synchronize with remote repositories.
- **Filtering**: Quickly filter the revision graph using `jj` revsets (e.g., `mine()`, `trunk()`, `conflicts()`).
- **Conflict Resolution**: Launch external merge tools to resolve conflicts directly from the TUI.
- **Real-time Monitoring**: Automatically refreshes the UI when changes are detected in the repository.

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Jujutsu (jj)](https://github.com/martinvonz/jj) installed and configured on your system.

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/judo.git
cd judo

# Build and install
cargo install --path .
```

Alternatively, you can run it directly:
```bash
cargo run
```

## Usage

Launch `judo` from within any Jujutsu-initialized repository.

### Keybindings

#### Navigation & Focus
| Key | Action |
| --- | --- |
| `j` / `↓` | Select next revision |
| `k` / `↑` | Select previous revision |
| `Enter` | Toggle the diff panel for the selected revision |
| `Tab` / `l` | Focus the diff panel |
| `h` | Focus the revision graph (when diff is focused) |
| `Esc` | Cancel current mode / Clear error messages |
| `q` | Quit Judo |

#### Operations
| Key | Action |
| --- | --- |
| `s` | Snapshot the current working copy |
| `e` | Edit the selected revision |
| `n` | Create a new child from the selected revision |
| `d` | Describe the selected revision (opens input) |
| `a` | Abandon the selected revision |
| `S` | Squash the selected revision into its parent |
| `b` | Set a bookmark on the selected revision |
| `B` | Delete the first bookmark on the selected revision |
| `u` | Undo the last operation |
| `U` | Redo the last operation |
| `f` | Fetch from the remote |
| `p` | Push to the remote |

#### Filtering
| Key | Action |
| --- | --- |
| `/` | Enter a custom revset filter |
| `m` | Quick filter: `mine()` |
| `t` | Quick filter: `trunk()` |
| `c` | Quick filter: `conflicts()` |

#### Diff View (when focused)
| Key | Action |
| --- | --- |
| `PgDn` / `PgUp` | Scroll through the diff |
| `[` / `]` | Jump to the previous/next hunk |
| `j` / `k` | Select the next/previous changed file |
| `m` / `Enter` | Resolve conflict (if the selected file has conflicts) |

## Architecture

Judo is built using **The Elm Architecture (TEA)** pattern, ensuring a robust and predictable state management system:

- **Model (`AppState`)**: The single source of truth for the application state.
- **Update (`reducer`)**: Pure functions that handle `Action`s to transition the `Model`.
- **View (`ui`)**: A declarative UI layer that renders based on the current `Model`.
- **Commands**: Asynchronous side effects (like VCS calls) that dispatch `Action`s back to the reducer.

For more details on the implementation and coding standards, see [AGENTS.md](./AGENTS.md).

## Contributing

We welcome contributions! To get started:

1. **Prerequisites**: Ensure you have Rust and Jujutsu installed.
2. **Setup**: Clone the repo and run `cargo build`.
3. **Tests**: Run `cargo test` to verify your changes.
4. **Code Quality**: Ensure your code is formatted (`cargo fmt`) and clean of linting issues (`cargo clippy`).

## Powered by

- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI
- [jj-lib](https://github.com/martinvonz/jj) - Jujutsu core library
- [tokio](https://tokio.rs/) - Async runtime
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal manipulation
