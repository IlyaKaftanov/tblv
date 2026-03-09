mod cli;

use clap::Parser;
use cli::Cli;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    println!("File: {:?}, Head: {}", cli.file, cli.head);
    Ok(())
}
