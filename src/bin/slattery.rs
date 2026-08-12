//! Slattery CLI - Standalone Slattery UI Framework CLI
//! 
//! This is a standalone CLI for creating and managing Slattery applications.

use clap::{Parser, Subcommand};
use std::process;

#[derive(Parser)]
#[command(name = "slattery")]
#[command(about = "Slattery UI Framework CLI")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Slattery application
    New {
        /// Name of the application to create
        name: String,
    },
    /// Run a Slattery application
    Run {
        /// Path to the main.st file to run
        file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => {
            println!("Creating new Slattery application: {}", name);
            if let Err(e) = slate::slattery::install::create_app_environment(&name) {
                eprintln!("[ERROR] Failed to create application: {}", e);
                process::exit(1);
            }
            println!("[OK] Application '{}' created successfully!", name);
            println!("[File] Run your app with: slattery run {}/src/main.st", name);
        }
        Commands::Run { file } => {
            if let Err(e) = slate::run_file(std::path::Path::new(&file)) {
                eprintln!("[ERROR] Failed to run application: {}", e);
                process::exit(1);
            }
        }
    }
}
