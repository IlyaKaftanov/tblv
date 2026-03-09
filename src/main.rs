mod app;
mod cli;
mod data;
mod event;
mod ui;

use clap::Parser;
use cli::Cli;
use crossterm::event as ct_event;
use data::DataSource;

use crate::app::{App, View};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let mut ds = DataSource::open(&cli.file, cli.head)?;
    let df = ds.head()?;
    let columns = ds.column_names()?;
    let dtypes = ds.column_dtypes()?;

    let mut app = App::new(df, columns, dtypes);

    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, &mut app, &ds);
    ratatui::restore();

    result.map_err(|e| color_eyre::eyre::eyre!(e))?;
    Ok(())
}

fn run_app(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    ds: &DataSource,
) -> std::io::Result<()> {
    loop {
        // Compute stats if loading (blocking for MVP).
        if app.loading {
            let result = match app.view {
                View::Describe => ds.describe_column(&app.stats_column),
                View::Uniques => ds.value_counts(&app.stats_column, 100),
                _ => unreachable!(),
            };
            match result {
                Ok(df) => app.stats_result = Some(df),
                Err(_) => app.view = View::Table,
            }
            app.loading = false;
        }

        terminal.draw(|frame| ui::render(frame, app))?;

        if app.should_quit {
            break Ok(());
        }

        let ev = ct_event::read().map_err(std::io::Error::other)?;
        event::handle_event(app, ev);
    }
}
