use clap::{Parser, Subcommand};
use std::process;

mod commands;
mod handlers;
mod helpers;
mod options;

use commands::daily::DailyArgs;
use commands::weekly::WeeklyArgs;
// Configuration is loaded per-command inside the subcommand modules.

/// A CLI for creating and managing weekly and daily markdown notes
#[derive(Parser)]
#[command(name = "take-note")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = env!("CARGO_PKG_DESCRIPTION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Open a file for the given week's note, creating it first if it doesn't exist
    Weekly(WeeklyArgs),
    /// Open a file for the given day's note, creating it first if it doesn't exist
    Daily(DailyArgs),
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Weekly(args) => commands::weekly::run(args),
        Commands::Daily(args) => commands::daily::run(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
