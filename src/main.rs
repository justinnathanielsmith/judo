use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use judo::app::{r#loop::run_loop, state::AppState};
use judo::domain::vcs::VcsFacade;
use judo::infrastructure;

fn setup_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_panic_hook();

    // Initialize adapter to verify repo context
    // This happens BEFORE terminal setup so if it fails (e.g. corrupt config),
    // we don't leave the terminal in raw mode.
    let adapter = std::sync::Arc::new(infrastructure::jj_adapter::JjAdapter::new()?);
    let key_config = judo::app::keymap::KeyConfig::load();
    let mut app_state = AppState::new(key_config);

    if !adapter.is_valid().await {
        app_state.mode = judo::app::state::AppMode::NoRepo;
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let res = run_loop(&mut terminal, app_state, adapter).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}
