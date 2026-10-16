//! slattery — CLI for the Slattery UI framework.
//!
//! Usage:
//!   slattery new <name>              scaffold a new Slattery project
//!   slattery run <app>               run a Slattery app with its own icon
//!   slattery build <app> [--release] build a standalone binary
//!                                     (not yet implemented)
//!   slattery --version               print version
//!   slattery --help                  print usage

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return ExitCode::from(1);
    }

    if args[1] == "--version" || args[1] == "-V" {
        println!("slattery {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    if args[1] == "--help" || args[1] == "-h" {
        print_usage();
        return ExitCode::SUCCESS;
    }

    match args[1].as_str() {
        "new" => run_new(&args),
        "run" => run_run(&args),
        "build" => run_build(&args),
        other => {
            eprintln!("unknown subcommand: {}", other);
            println!();
            print_usage();
            ExitCode::from(1)
        }
    }
}

fn run_new(args: &[String]) -> ExitCode {
    if args.len() < 3 {
        eprintln!("usage: slattery new <name>");
        return ExitCode::from(1);
    }
    let name = &args[2];

    match slattery::install::create_app_environment(name) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn run_run(args: &[String]) -> ExitCode {
    if args.len() < 3 {
        eprintln!("usage: slattery run <app>");
        return ExitCode::from(1);
    }
    let app = &args[2];
    let app_dir = Path::new(app);

    if !app_dir.exists() {
        eprintln!("error: app directory '{}' not found", app);
        return ExitCode::from(1);
    }

    let main_st = app_dir.join("src").join("main.st");
    if !main_st.exists() {
        eprintln!(
            "error: '{}' not found. Is '{}' a Slattery app?",
            main_st.display(),
            app
        );
        return ExitCode::from(1);
    }

    let icon_path = app_dir.join("assets").join("logo.png");
    let icon = if icon_path.exists() {
        Some(icon_path)
    } else {
        None
    };

    match slattery::run::run_app(&main_st, icon.as_deref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn run_build(args: &[String]) -> ExitCode {
    if args.len() < 3 {
        eprintln!("usage: slattery build <app> [--release]");
        return ExitCode::from(1);
    }
    let app = &args[2];
    let release = args.iter().any(|a| a == "--release");

    // Not yet implemented: the generated main.rs calls `slate::run_file`,
    // but `slate` is a binary, not a library. Once `slate` is split into
    // a library crate + binary, this becomes a two-line call to
    // `slattery::build::build_app`.
    let _ = (app, release);
    eprintln!("error: `slattery build` is not implemented yet.");
    eprintln!();
    eprintln!("In the meantime you can run an app directly:");
    eprintln!("  slattery run {}", app);
    ExitCode::from(1)
}

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  slattery new <name>                scaffold a new Slattery project");
    eprintln!("  slattery run <app>                 run a Slattery app");
    eprintln!("  slattery build <app> [--release]   build a Slattery app (not yet implemented)");
    eprintln!("  slattery --version                 print version");
    eprintln!("  slattery --help                    print this message");
}