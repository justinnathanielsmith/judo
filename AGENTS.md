# AGENTS.md

This file guides AI agents working on the Judo codebase. Follow these instructions to ensure consistency, stability, and high performance.

## Mission
Build a high-performance, idiomatic Rust TUI for [Jujutsu (jj)](https://github.com/martinvonz/jj). The goal is to provide a fast, intuitive interface for version control operations.

## Tech Stack
- **Language**: Rust (2021 edition)
- **TUI Framework**: [ratatui](https://github.com/ratatui-org/ratatui) (latest stable)
- **Async Runtime**: [tokio](https://tokio.rs/)
- **VCS Backend**: `jj-lib` (Jujutsu library)
- **Error Handling**: `anyhow` for app-level errors.

## Architecture: The Elm Architecture (TEA)
We strictly follow The Elm Architecture pattern to manage state and side effects.

### 1. Model (`AppState`)
- Located in `src/app/state.rs`.
- Validates the **Single Source of Truth** for the entire application.
- Contains UI state (selection, mode) and Domain state (RepoStatus).

### 2. Update (`reducer`)
- Located in `src/app/reducer.rs`.
- Handles `Action`s to modify `AppState`.
- **Pure Functions**: Reducers should be pure where possible.
- **Side Effects**: commands (like Git operations) should return an `Action` to be processed next, rather than mutating state directly inside an async block without an action.

### 3. View (`ui`)
- Located in `src/app/ui.rs` or component modules.
- **Declarative**: Renders the UI based *solely* on the current `AppState`.
- No business logic in views.

## Coding Standards

### Rust Idioms
- **Error Handling**: Use `Result<T, anyhow::Error>` for fallible functions in the app layer. Unwrap only when you are 100% sure it will not panic (and document why).
- **Async**: Use `tokio::task::spawn_blocking` for any heavy computation or blocking I/O (especially `jj-lib` calls) to keep the UI thread responsive.
- **Clippy**: Ensure code is clippy-clean (`cargo clippy`).

### File Structure
- `src/domain/`: Core business logic and VCS abstractions.
- `src/infrastructure/`: Adapters for external systems (e.g., `jj-lib`).
- `src/app/`: Application-specific logic (state, reducer, loop).
- `src/components/`: Reusable UI widgets.

## Workflow

1. **Understand First**: Read `AppState` and related `Action`s before modifying code.
2. **Atomic Changes**: Keep PRs/Changing focused on one feature or fix.
3. **Verify**:
    - Run `cargo check` to catch compilation errors.
    - Run `cargo test` to ensure no regressions.
