pub mod app;
pub mod data;
pub mod model;
pub mod mts_client;
pub mod ui;

use std::{io, time::Duration};

use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::CrosstermBackend, Terminal};

use crate::{app::App, data::DataProvider};

fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let provider = data::build_provider_from_env();
    let mut app = App::new(vec![()], provider.as_ref());

    setup_terminal()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let run_result = run_app(&mut terminal, &mut app, provider.as_ref());

    restore_terminal()?;
    run_result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    provider: &dyn DataProvider,
) -> Result<()> {
    while !app.quit {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(app.tick_rate)? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key, provider);
            }
        }

        app.tick(provider);
        std::thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}

fn setup_terminal() -> Result<()> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    Ok(())
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
