use clap::{Parser, Subcommand};
use std::process;

mod commands;
mod handlers;
mod helpers;
mod options;

use commands::daily::DailyArgs;
use commands::init::InitError;
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
    /// Run the interactive setup wizard to create or update ~/.config/take-note/config.toml
    Init,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Weekly(args) => {
            if let Err(e) = commands::weekly::run(args) {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        }
        Commands::Daily(args) => {
            if let Err(e) = commands::daily::run(args) {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        }
        Commands::Init => match commands::init::run() {
            Ok(()) => {}
            Err(InitError::Interrupted) => process::exit(130),
            Err(InitError::PreFlightFixed) => {
                eprintln!("pre-flight fixes applied — rerun `take-note init` to continue");
                process::exit(2);
            }
            Err(InitError::Other(msg)) => {
                eprintln!("Error: {msg}");
                process::exit(1);
            }
        },
    }
}
