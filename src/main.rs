// src/main.rs

use slate_core::*;
use slattery::*;
use slate_sfile::SFileExtension;

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process;
use std::io::{self, Write, BufRead};

#[derive(Parser)]
#[command(name = "slate")]
#[command(version = "0.1.0")]
#[command(about = "SlateScript Programming Language")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a SlateScript file
    Run {
        /// Path to the .st file
        file: PathBuf,
        
        /// Explicitly load extensions from specific dynamic libraries
        #[arg(short, long)]
        extensions: Option<Vec<PathBuf>>,
    },

    /// Slattery UI Framework commands
    Slattery {
        #[command(subcommand)]
        command: SlatteryCommands,
    },
}

#[derive(Subcommand)]
enum SlatteryCommands {
    /// Create a new Slattery application
    New {
        name: String,
    },
    /// Run a Slattery application (loads Slattery dynamically)
    Run {
        file: PathBuf,
    },
    /// Build a Slattery application into an executable
    Build {
        name: String,
        #[arg(short, long, default_value = "build")]
        output: String,
        #[arg(short, long)]
        release: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, extensions: _extensions } => {
            if !file.exists() {
                eprintln!("[ERROR] File not found: {:?}", file);
                process::exit(1);
            }
            
            match std::fs::read_to_string(&file) {
                Ok(source) => {
                    let has_ui = has_ui_syntax(&source);
                    
                    if has_ui {
                        let mut ui_framework = slattery::ui_integration::UiFramework::new();
                        
                        match ui_framework.parse_and_render(&source, Some(&file)) {
                            Ok(_) => {
                                println!("[OK] UI application completed successfully!");
                            }
                            Err(e) => {
                                eprintln!("[ERROR] Slattery UI error: {}", e);
                                process::exit(1);
                            }
                        }
                    } else {
                        if let Err(e) = slate_core::run_file(&file) {
                            eprintln!("[ERROR] {}", e);
                            process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] Failed to read file: {}", e);
                    process::exit(1);
                }
            }
        }
        Commands::Slattery { command } => {
            if let Err(e) = handle_slattery_command(command) {
                eprintln!("[ERROR] {}", e);
                process::exit(1);
            }
        }
    }
}

fn has_ui_syntax(source: &str) -> bool {
    if source.contains("import from \"slattery\"") {
        return true;
    }
    
    let ui_keywords = [
        "Window", "Column", "Row", "Text", "Button", "Input",
        "Child:", "Parent:", "on_click", "on_tap", "on_change", "on_input",
        "Identity:", "Rewrite", "render", "make App ="
    ];
    
    for keyword in ui_keywords {
        if source.contains(keyword) {
            return true;
        }
    }
    
    false
}

fn handle_slattery_command(command: SlatteryCommands) -> Result<(), String> {
    match command {
        SlatteryCommands::New { name } => {
            println!("Creating new Slattery application: {}", name);
            slattery::install::create_app_environment(&name)
                .map_err(|e| format!("Failed to create app: {}", e))?;
            println!("[OK] Application '{}' created!", name);
            Ok(())
        }
        SlatteryCommands::Run { file } => {
            let source = std::fs::read_to_string(&file)
                .map_err(|e| format!("Failed to read file: {}", e))?;
            
            slattery::ui_integration::UiFramework::new()
                .parse_and_render(&source, Some(&file))
                .map_err(|e| format!("Runtime error: {}", e))
        }
        SlatteryCommands::Build { name, output, release } => {
            let config = slattery::build::BuildConfig::new(&name, &output, release)?;
            slattery::build::build_app(&config)
        }
    }
}