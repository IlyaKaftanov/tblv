mod app;
mod cli;
mod data;
mod event;
mod geo;
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
    app.total_file_rows = ds.total_row_count().ok();

    if !ds.skipped_columns.is_empty() {
        let mut msg = String::from("Skipped columns (unsupported types):\n\n");
        for sc in &ds.skipped_columns {
            msg.push_str(&format!("  {} ({})\n", sc.name, sc.type_name));
        }
        msg.push_str("\nPress any key to continue");
        app.notification = Some(msg);
    }

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
            match app.view {
                View::Describe => match ds.describe_column(&app.stats_column) {
                    Ok(df) => app.stats_result = Some(df),
                    Err(_) => app.view = View::Table,
                },
                View::Uniques => match ds.value_counts(&app.stats_column, 100) {
                    Ok(df) => app.stats_result = Some(df),
                    Err(_) => app.view = View::Table,
                },
                View::FilterMenu => match ds.unique_values(&app.stats_column, 200) {
                    Ok(values) => {
                        let active = app
                            .active_filter_for_col(&app.stats_column)
                            .cloned()
                            .unwrap_or_default();
                        app.filter_items = values
                            .into_iter()
                            .map(|v| {
                                let selected = !active.is_empty() && active.contains(&v);
                                (v, selected)
                            })
                            .collect();
                    }
                    Err(_) => {
                        app.view = View::Table;
                    }
                },
                _ => {}
            }
            app.loading = false;
        }

        // Refresh data when sort/filter state changes.
        if app.needs_refresh {
            let sort_col_name = app.sort_col.map(|i| app.columns[i].as_str());
            if let Ok(df) = ds.query(&app.filters, sort_col_name, app.sort_desc) {
                app.data = df;
            }
            app.needs_refresh = false;
            app.cursor_row = 0;
            app.row_offset = 0;
        }

        terminal.draw(|frame| ui::render(frame, app))?;

        if app.should_quit {
            break Ok(());
        }

        let ev = ct_event::read().map_err(std::io::Error::other)?;
        event::handle_event(app, ev);
    }
}
