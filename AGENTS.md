# AGENTS.md

This file guides AI agents working on the Judo codebase. Follow these instructions to ensure consistency, stability, and high performance.

## Mission

Build a high-performance, idiomatic Rust TUI for [Jujutsu (jj)](https://github.com/martinvonz/jj). The goal is to provide a fast, intuitive interface for version control operations.

## Tech Stack

* **Language**: Rust (2021 edition)
* **TUI Framework**: [ratatui](https://github.com/ratatui-org/ratatui) (latest stable)
* **Async Runtime**: [tokio](https://tokio.rs/)
* **VCS Backend**: `jj-lib` (Jujutsu library)
* **Error Handling**: `anyhow` for app-level errors

## Architecture: The Elm Architecture (TEA)

We strictly follow The Elm Architecture pattern to manage state and side effects.

### 1. Model (`AppState`)

* Located in `src/app/state.rs`.
* Must remain the **Single Source of Truth** for the entire application.
* Contains UI state (selection, mode) and Domain state (RepoStatus).

### 2. Update (`reducer`)

* Located in `src/app/reducer.rs`.
* Handles `Action`s to modify `AppState`.
* **Pure Functions**: Reducers must be pure; logic should only calculate the next state from the current state and an action.
* **Side Effects**: Async operations or commands (like Git operations) must return an `Action` to be processed by the reducer rather than mutating state directly inside an async block.

### 3. View (`ui`)

* Located in `src/app/ui.rs` or component modules.
* **Declarative**: The UI must be rendered *solely* based on the current `AppState`.
* **Logic-Free**: No business or domain logic is permitted in the view layer.

## Coding Standards

### Rust Idioms & Performance

* **Error Handling**: Use `Result<T, anyhow::Error>` for all fallible functions in the app layer.
* **Panic Prevention**: Do not use `unwrap()` unless it is mathematically or logically impossible to fail. You must document the reasoning for any `unwrap()` used.
* **Responsive UI**: Use `tokio::task::spawn_blocking` for any heavy computation or blocking I/O, particularly for `jj-lib` calls, to keep the UI thread responsive.
* **Clippy**: Ensure code is clippy-clean (`cargo clippy`).

### File Structure

* `src/domain/`: Core business logic and VCS abstractions.
* `src/infrastructure/`: Adapters for external systems like `jj-lib`.
* `src/app/`: Application-specific logic (state, reducer, loop).
* `src/components/`: Reusable UI widgets.

## Workflow

1. **Understand First**: Read `AppState` and related `Action`s before modifying code.
2. **Atomic Changes**: Keep changes focused on exactly one feature or fix per task.
3. **Verify**:
* Run `cargo check` to catch compilation errors.
* Run `cargo test` to ensure no regressions.
* Verify that the `Esc` key correctly cancels current modes and clears errors in the UI.

## Lessons Learned

### Clippy & Formatting
* **Sequential Tools**: Always run `cargo fmt` after `cargo clippy --fix`. Clippy can introduce formatting artifacts (like trailing whitespace or long lines) that will fail strict CI formatting checks.
* **Async Cleanup**: Be mindful of Clippy's `unused_async` lint. Removing `async` from functions that spawn background tasks (rather than awaiting them) requires surgical updates to all call sites and tests to remove unnecessary `.await` calls.

### Version Control Workflow
* **Jujutsu (jj) Integration**: This project uses `jj` alongside Git.
  * Use `jj describe -m "message"` to update commit descriptions.
  * Use `jj bookmark set main -r @` to move the `main` branch to the current working copy.
  * Use `jj git push --branch main` to synchronize with the remote.
  * When tagging for releases, ensure the tag is placed on the immutable commit (after pushing or describing) and pushed to `origin` to trigger GitHub Actions.
