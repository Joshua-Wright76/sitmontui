pub mod app;
pub mod data;
pub mod market_ticker;
pub mod model;
pub mod mts_client;
pub mod ui;

use std::{io, sync::Arc, time::Duration};

use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::CrosstermBackend, Terminal};

use crate::{app::App, data::DataProvider, market_ticker::MarketTicker};

fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let provider = data::build_provider_from_env();
    let mut app = App::new(vec![()], provider.as_ref());

    let market_ticker = Arc::clone(&app.market_ticker);
    std::thread::spawn(move || {
        let mut ticker = MarketTicker::new();
        loop {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            match ticker.fetch_quotes(now_ms) {
                Ok(()) => {
                    if let Ok(mut guard) = market_ticker.lock() {
                        *guard = ticker.clone();
                    }
                }
                Err(e) => {
                    eprintln!("Market ticker error: {}", e);
                }
            }
            std::thread::sleep(Duration::from_secs(300));
        }
    });

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
        let ticker_data = {
            let mut ticker = app.market_ticker.lock().unwrap();
            ticker.maybe_scroll(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            );
            ticker.clone()
        };
        terminal.draw(|f| ui::draw(f, app, &ticker_data))?;

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
