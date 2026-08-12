// src/main.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;  // ← Added Path here

#[derive(Parser)]
#[command(name = "slate")]
#[command(version = "0.0.1")]
#[command(about = "SlateScript Programming Language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run { file: PathBuf },
    Version,
    Slit { 
        #[command(subcommand)] 
        command: SlitCommands 
    },
    Slattery { 
        #[command(subcommand)] 
        command: SlatteryCommands 
    },
}

#[derive(Subcommand)]
enum SlatteryCommands {
    /// Create a new Slattery application
    New { name: String },
    /// Build a Slattery application into an executable
    Build {
        name: String,
        #[arg(short, long, default_value = "build")]
        output: String,
        #[arg(short, long)]
        release: bool,
    },
    /// Run a Slattery application
    Run {
        file: PathBuf,
    },
    /// Clean build artifacts
    Clean {
        /// Name of the app to clean
        name: String,
    },
}

#[derive(Subcommand)]
enum SlitCommands {
    Install { package: String },
    Uninstall { package: String },
    List,
    Search { query: Option<String> },
    Init,
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Run { file } => {
            if file.extension().and_then(|ext| ext.to_str()) != Some("st") {
                eprintln!("[ERROR] File must have .st extension");
                std::process::exit(1);
            }
            
            if !file.exists() {
                eprintln!("[ERROR] File '{}' not found", file.display());
                std::process::exit(1);
            }
            
            if let Err(e) = slate::run_file(&file) {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        }
        
        Commands::Version => {
            println!("SlateScript v0.1.0");
        }
        
        Commands::Slit { command } => {
            if let Err(e) = handle_slit_command(command) {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        }
        
        Commands::Slattery { command } => {
            if let Err(e) = handle_slattery_command(command) {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn handle_slit_command(command: SlitCommands) -> Result<(), String> {
    match command {
        SlitCommands::Install { package } => {
            if package == "slattery" || package == "math-utils" {
                println!("+-------------------------------------------+");
                println!("| SLIT: Installing {}", package);
                println!("+-------------------------------------------+");
                println!("> Successfully installed {}", package);
                Ok(())
            } else {
                println!("Package '{}' not found in registry", package);
                Ok(())
            }
        }
        SlitCommands::Uninstall { package } => {
            println!("Package '{}' uninstalled", package);
            Ok(())
        }
        SlitCommands::List => {
            println!("+-------------------------------------------+");
            println!("| Installed Packages                        |");
            println!("+-------------------------------------------+");
            println!("  math-utils v0.1.0 - Basic math utilities");
            println!("  slattery  v1.0.0 - UI framework");
            println!("+-------------------------------------------+");
            Ok(())
        }
        SlitCommands::Search { query } => {
            println!("+-------------------------------------------+");
            match query {
                Some(q) => println!("| Search results for '{}':", q),
                None => println!("| Available packages:"),
            }
            println!("+-------------------------------------------+");
            println!("  math-utils - Basic math utilities");
            println!("  slattery  - UI framework");
            println!("+-------------------------------------------+");
            Ok(())
        }
        SlitCommands::Init => {
            std::fs::write("slate-packages.toml", "[packages]\n")
                .map_err(|e| format!("Failed to create package config: {}", e))?;
            println!("[OK] Created slate-packages.toml");
            Ok(())
        }
    }
}

fn handle_slattery_command(command: SlatteryCommands) -> Result<(), String> {
    match command {
        SlatteryCommands::New { name } => {
            println!("Creating new Slattery application: {}", name);
            slate::slattery::install::create_app_environment(&name)
                .map_err(|e| format!("Failed to create application: {}", e))?;
            println!("[OK] Application '{}' created successfully!", name);
            println!("[INFO] Next steps:");
            println!("  1. cd {}", name);
            println!("  2. slate slattery run src/main.st");
            Ok(())  // ← Make sure this returns Ok(())
        }
        SlatteryCommands::Build { name, output, release } => {
            let config = slate::slattery::build::BuildConfig::new(&name, &output, release)?;
            slate::slattery::build::build_app(&config)
        }
        SlatteryCommands::Run { file } => {
            run_slattery_app(&file)
        }
        SlatteryCommands::Clean { name } => {
            clean_app(&name)
        }
    }
}

fn clean_app(app_name: &str) -> Result<(), String> {
    println!("+-------------------------------------------+");
    println!("| Cleaning Slattery Application             |");
    println!("+-------------------------------------------+");
    println!("  App: {}", app_name);
    println!("+-------------------------------------------+");

    // Find the SlateScript project directory
    let slate_dir = slate::slattery::build::find_slate_dir()?;
    
    // Build directory path
    let build_dir = slate_dir
        .join("target")
        .join("slattery_builds")
        .join(app_name);
    
    // Remove build directory
    if build_dir.exists() {
        std::fs::remove_dir_all(&build_dir)
            .map_err(|e| format!("Failed to clean build directory: {}", e))?;
        println!("  [OK] Removed: {}", build_dir.display());
    } else {
        println!("  [INFO] Build directory not found: {}", build_dir.display());
    }
    
    // Remove output directory
    let output_dir = PathBuf::from("build");
    if output_dir.exists() {
        std::fs::remove_dir_all(&output_dir)
            .map_err(|e| format!("Failed to clean output directory: {}", e))?;
        println!("  [OK] Removed: ./build");
    } else {
        println!("  [INFO] Output directory not found");
    }
    
    println!("+-------------------------------------------+");
    println!("[OK] Clean complete!");
    println!("+-------------------------------------------+");
    
    Ok(())
}

fn run_slattery_app(file: &PathBuf) -> Result<(), String> {
    let source = std::fs::read_to_string(file)
        .map_err(|e| format!("Failed to read file '{}': {}", file.display(), e))?;

    println!("Running: {}\n", file.display());

    // Step 1: Parse with UI lexer and interpreter
    let mut ui_lexer = slate::slattery::sla_lexer::UiLexer::new(&source);
    let ui_tokens = ui_lexer.tokenize();

    let mut ui_interpreter = slate::slattery::sla_interpreter::UiInterpreter::new();
    let components = ui_interpreter.interpret(ui_tokens)
        .map_err(|e| format!("UI interpretation error: {}", e))?;

    // Step 2: Create a new AST interpreter for functions
    let mut ast_interpreter = slate::ast_interpreter::AstInterpreter::new();

    // Step 3: Register UI functions with the AST interpreter
    for (func_name, func_tokens) in &ui_interpreter.functions {
        println!("[INFO] Registering function: {}", func_name);
        
        // Build a complete function definition
        let mut main_tokens = Vec::new();
        
        // func name<>
        main_tokens.push(slate::lexer::Token::Func);
        main_tokens.push(slate::lexer::Token::Identifier(func_name.clone()));
        main_tokens.push(slate::lexer::Token::LessThan);
        main_tokens.push(slate::lexer::Token::GreaterThan);
        
        // Body
        main_tokens.push(slate::lexer::Token::LeftBrace);
        
        // Convert function body tokens from UI tokens to main tokens
        for token in func_tokens {
            match token {
                slate::slattery::sla_lexer::UiToken::Identifier(s) => {
                    main_tokens.push(slate::lexer::Token::Identifier(s.clone()));
                }
                slate::slattery::sla_lexer::UiToken::String(s) => {
                    main_tokens.push(slate::lexer::Token::String(s.clone()));
                }
                slate::slattery::sla_lexer::UiToken::Number(n) => {
                    main_tokens.push(slate::lexer::Token::Number(*n as i64));
                }
                slate::slattery::sla_lexer::UiToken::True => {
                    main_tokens.push(slate::lexer::Token::True);
                }
                slate::slattery::sla_lexer::UiToken::False => {
                    main_tokens.push(slate::lexer::Token::False);
                }
                slate::slattery::sla_lexer::UiToken::LeftParen => {
                    main_tokens.push(slate::lexer::Token::LeftParen);
                }
                slate::slattery::sla_lexer::UiToken::RightParen => {
                    main_tokens.push(slate::lexer::Token::RightParen);
                }
                slate::slattery::sla_lexer::UiToken::LeftBrace => {
                    main_tokens.push(slate::lexer::Token::LeftBrace);
                }
                slate::slattery::sla_lexer::UiToken::RightBrace => {
                    main_tokens.push(slate::lexer::Token::RightBrace);
                }
                slate::slattery::sla_lexer::UiToken::Comma => {
                    main_tokens.push(slate::lexer::Token::Comma);
                }
                slate::slattery::sla_lexer::UiToken::Equal => {
                    main_tokens.push(slate::lexer::Token::Equal);
                }
                slate::slattery::sla_lexer::UiToken::Semicolon => {
                    main_tokens.push(slate::lexer::Token::Semicolon);
                }
                _ => {}
            }
        }
        
        main_tokens.push(slate::lexer::Token::RightBrace);
        main_tokens.push(slate::lexer::Token::EOF);
        
        // Register the function with the AST interpreter
        if let Err(e) = ast_interpreter.run(&main_tokens) {
            eprintln!("[WARN] Failed to register function '{}': {}", func_name, e);
        } else {
            println!("[OK] Registered function: {}", func_name);
        }
    }

    // Step 4: Build component map for the renderer
    let mut component_map = std::collections::HashMap::new();
    for component in &components {
        let comp = component.borrow();
        let key = if let Some(name) = comp.identity.as_ref() {
            name.clone()
        } else {
            comp.component_type.clone()
        };
        component_map.insert(key, component.clone());
    }

    // Step 5: Find style files
    let mut style_files = Vec::new();
    if let Some(parent) = file.parent() {
        let styles_dir = parent.join("styles");
        if let Ok(entries) = std::fs::read_dir(&styles_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("sts") {
                    style_files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }

    // Step 6: Create renderer with the AST interpreter
    let mut renderer = slate::slattery::egui_renderer::EguiRenderer::new();
    renderer.interpreter = ast_interpreter;  // Use the registered functions
    renderer.set_components(component_map);
    renderer.load_styles(&style_files);
    
    // Step 7: Run the app
    slate::slattery::egui_renderer::run_egui_app_with_renderer(renderer)
        .map_err(|e| format!("Failed to run UI: {}", e))?;

    Ok(())
}