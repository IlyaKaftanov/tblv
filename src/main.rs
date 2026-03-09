mod app;
mod cli;
mod data;

use clap::Parser;
use cli::Cli;
use data::DataSource;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let mut ds = DataSource::open(&cli.file, cli.head)?;
    let df = ds.head()?;
    println!("Loaded {} rows x {} cols", df.height(), df.width());
    println!("Columns: {:?}", ds.column_names()?);

    Ok(())
}
