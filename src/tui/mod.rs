use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, terminal};
use ratatui::prelude::*;

mod input;
mod state;
mod ui;

pub fn run_tui(db_path: PathBuf) -> Result<()> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let mut app = state::App::new(db_path)?;

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut state::App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw_ui(frame, app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if input::handle_key(app, key)? {
                    return Ok(());
                }
            }
        }
    }
}
