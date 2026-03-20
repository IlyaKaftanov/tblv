use clap::Parser;
use std::path::PathBuf;

/// tblv — Terminal Table Viewer for CSV and Parquet files
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    /// Path to CSV or Parquet file
    pub file: PathBuf,

    /// Number of rows to display
    #[arg(short = 'n', long = "head", default_value_t = 5000)]
    pub head: u32,
}
