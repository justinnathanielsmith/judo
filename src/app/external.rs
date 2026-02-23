use anyhow::Result;
use ratatui::{backend::Backend, Terminal};
use std::process::Command;

pub fn run_external_command<B: Backend>(
    terminal: &mut Terminal<B>,
    program: &str,
    args: &[&str],
) -> Result<bool> {
    // 1. Suspend TUI
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;

    // 2. Run external tool
    let mut child = Command::new(program).args(args).spawn()?;
    let status = child.wait()?;

    // 3. Resume TUI
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide
    )?;

    // Clear the terminal to remove any leftover output from the external command
    terminal.clear()?;

    Ok(status.success())
}
