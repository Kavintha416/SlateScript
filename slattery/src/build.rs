// slattery/src/build.rs

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct BuildConfig {
    pub app_name: String,
    pub output_dir: PathBuf,
    pub release: bool,
    pub app_root: PathBuf,
}

impl BuildConfig {
    pub fn new(app_name: &str, output_dir: &str, release: bool) -> Result<Self, String> {
        println!("[DEBUG] BuildConfig::new called with app_name: {}", app_name);

        // Resolve and canonicalize the app root up front so every subsequent
        // operation works with an absolute, normalised path.
        let raw = PathBuf::from(app_name);
        let candidate = if raw.is_absolute() {
            raw
        } else {
            std::env::current_dir()
                .map_err(|e| format!("Failed to get current dir: {}", e))?
                .join(app_name)
        };

        if !candidate.exists() {
            return Err(format!("App '{}' not found at '{}'", app_name, candidate.display()));
        }

        // Canonicalize strips any trailing separators and resolves symlinks.
        // On Windows this also strips the \\?\ prefix we strip manually elsewhere.
        let app_root = strip_unc(
            fs::canonicalize(&candidate)
                .unwrap_or(candidate)
        );

        println!("[DEBUG] app_root: {}", app_root.display());

        let project_name = read_project_name(&app_root)?;
        println!("[DEBUG] project_name from Slattery.toml: {}", project_name);

        let final_name = if project_name.is_empty() || project_name == "." {
            app_root
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        } else {
            project_name
        };

        println!("[DEBUG] final_name: {}", final_name);

        Ok(Self {
            app_name: final_name,
            output_dir: PathBuf::from(output_dir),
            release,
            app_root,
        })
    }
}

// ---------------------------------------------------------------------------
// Helper: strip the \\?\ extended-length prefix Windows adds after
// canonicalize(), because Cargo and most tools do not accept it.
// ---------------------------------------------------------------------------
fn strip_unc(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    if s.starts_with(r"\\?\") {
        PathBuf::from(&s[4..])
    } else {
        p
    }
}

fn read_project_name(app_root: &Path) -> Result<String, String> {
    let toml_path = app_root.join("Slattery.toml");
    if !toml_path.exists() {
        return Err(format!("Slattery.toml not found in {}", app_root.display()));
    }

    let content = fs::read_to_string(&toml_path)
        .map_err(|e| format!("Failed to read Slattery.toml: {}", e))?;

    println!("[DEBUG] Slattery.toml content:\n{}", content);

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("name =") {
            let name = line
                .splitn(2, '=')
                .nth(1)
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            println!("[DEBUG] Found name: {}", name);
            if !name.is_empty() {
                return Ok(name);
            }
        }
    }

    // Fallback to the directory name.
    let folder_name = app_root
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    println!("[DEBUG] Using folder name as fallback: {}", folder_name);
    Ok(folder_name)
}

pub fn build_app(config: &BuildConfig) -> Result<(), String> {
    println!("+-------------------------------------------+");
    println!("| Building Slattery Application             |");
    println!("+-------------------------------------------+");
    println!("  App: {}", config.app_name);
    println!("  Output: {}", config.output_dir.display());
    println!("  Mode: {}", if config.release { "Release" } else { "Debug" });
    println!("+-------------------------------------------+");

    let slate_dir = find_slate_dir()?;
    println!("  SlateScript: {}", slate_dir.display());

    let src_path = config.app_root.join("src").join("main.st");
    if !src_path.exists() {
        return Err(format!(
            "src/main.st not found in '{}'. Run 'slate slattery new {}' first.",
            config.app_root.display(),
            config.app_name
        ));
    }

    let build_dir = slate_dir
        .join("target")
        .join("slattery_builds")
        .join(&config.app_name);

    fs::create_dir_all(&build_dir)
        .map_err(|e| format!("Failed to create build dir: {}", e))?;

    let build_src_dir = build_dir.join("src");
    fs::create_dir_all(&build_src_dir)
        .map_err(|e| format!("Failed to create build src dir: {}", e))?;

    // Copy .st source files (recursive so subdirectories are included).
    copy_directory(&config.app_root.join("src"), &build_src_dir, "st")?;
    println!("  [OK] Copied source files");

    let assets_src = config.app_root.join("assets");
    let assets_dest = build_dir.join("assets");
    if assets_src.exists() {
        fs::create_dir_all(&assets_dest)
            .map_err(|e| format!("Failed to create assets dir: {}", e))?;
        copy_all_files(&assets_src, &assets_dest)?;
        println!("  [OK] Copied assets");
    }

    let toml_src = config.app_root.join("Slattery.toml");
    if toml_src.exists() {
        fs::copy(&toml_src, build_dir.join("Slattery.toml"))
            .map_err(|e| format!("Failed to copy Slattery.toml: {}", e))?;
        println!("  [OK] Copied Slattery.toml");
    }

    generate_cargo_toml(&build_dir, &config.app_name, &slate_dir)?;
    println!("  [OK] Generated Cargo.toml");

    generate_main_rs(&build_src_dir)?;
    println!("  [OK] Generated main.rs");

    compile_app(&build_dir, config.release)?;
    println!("  [OK] Compiled app");

    copy_executable(&build_dir, &config.output_dir, &config.app_name, config.release)?;
    println!("  [OK] Copied executable to output");

    println!("+-------------------------------------------+");
    println!("[OK] Build complete!");
    println!(
        "  Executable: {}{}{}",
        config.output_dir.display(),
        std::path::MAIN_SEPARATOR,
        get_exe_name(&config.app_name)
    );
    println!("+-------------------------------------------+");

    Ok(())
}

// ---------------------------------------------------------------------------
// Locate the SlateScript project root.
//
// Strategy (in order of preference):
//   1. $SLATE_HOME environment variable
//   2. Walk up from the running executable's directory
//   3. Walk up from the current working directory
//
// Hard-coded OS-specific paths are intentionally omitted — they break the
// moment someone installs to a non-default location. $SLATE_HOME is the
// right escape hatch for unusual setups.
// ---------------------------------------------------------------------------
pub fn find_slate_dir() -> Result<PathBuf, String> {
    // 1. Explicit override.
    if let Ok(dir) = std::env::var("SLATE_HOME") {
        let path = strip_unc(PathBuf::from(&dir));
        if is_slate_root(&path) {
            return Ok(path);
        }
        // Warn but don't hard-fail — fall through to auto-detection.
        eprintln!(
            "[WARN] SLATE_HOME is set to '{}' but no valid Cargo.toml found there.",
            dir
        );
    }

    // 2. Walk up from the running executable.
    if let Ok(exe_path) = std::env::current_exe() {
        // exe_path.parent() is None only for a bare filename with no directory
        // component (extremely unlikely in practice but guard anyway).
        let start = exe_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        if let Some(found) = walk_up_for_slate_root(&start) {
            return Ok(found);
        }
    }

    // 3. Walk up from the current working directory.
    let cwd = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    if let Some(found) = walk_up_for_slate_root(&cwd) {
        return Ok(found);
    }

    Err(
        "Could not find the SlateScript project directory.\n\
         Set the SLATE_HOME environment variable to the SlateScript project root."
            .to_string(),
    )
}

/// Walk upward from `start`, returning the first directory that looks like the
/// SlateScript project root (has a Cargo.toml whose package name is "slate").
fn walk_up_for_slate_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    for _ in 0..15 {
        if is_slate_root(&current) {
            return Some(strip_unc(current));
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Returns true when `dir` is the SlateScript project root.
fn is_slate_root(dir: &Path) -> bool {
    let cargo_toml = dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return false;
    }
    fs::read_to_string(&cargo_toml)
        .map(|c| {
            // Match the exact package name declaration to avoid false positives
            // from workspace members like slate-core or slattery.
            c.contains("name = \"slate\"")
        })
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// File copy helpers
// ---------------------------------------------------------------------------

/// Recursively copy every file with the given extension from `src` into `dest`,
/// preserving the relative sub-directory structure.
fn copy_directory(src: &Path, dest: &Path, ext: &str) -> Result<(), String> {
    if !src.exists() {
        return Ok(());
    }

    for entry in
        fs::read_dir(src).map_err(|e| format!("Failed to read dir {}: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            // Recurse into subdirectory, mirroring the structure under dest.
            let sub_dest = dest.join(path.file_name().unwrap());
            fs::create_dir_all(&sub_dest)
                .map_err(|e| format!("Failed to create dir {}: {}", sub_dest.display(), e))?;
            copy_directory(&path, &sub_dest, ext)?;
        } else if path.is_file() {
            if path.extension().map_or(false, |e| e == ext) {
                let dest_path = dest.join(path.file_name().unwrap());
                fs::copy(&path, &dest_path)
                    .map_err(|e| format!("Failed to copy {}: {}", path.display(), e))?;
            }
        }
    }
    Ok(())
}

/// Recursively copy every file (regardless of extension) from `src` to `dest`.
fn copy_all_files(src: &Path, dest: &Path) -> Result<(), String> {
    if !src.exists() {
        return Ok(());
    }

    for entry in
        fs::read_dir(src).map_err(|e| format!("Failed to read dir {}: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            let sub_dest = dest.join(path.file_name().unwrap());
            fs::create_dir_all(&sub_dest)
                .map_err(|e| format!("Failed to create dir {}: {}", sub_dest.display(), e))?;
            copy_all_files(&path, &sub_dest)?;
        } else if path.is_file() {
            let dest_path = dest.join(path.file_name().unwrap());
            fs::copy(&path, &dest_path)
                .map_err(|e| format!("Failed to copy {}: {}", path.display(), e))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Cargo.toml generation
//
// Key decisions:
//   • No [workspace] table — this is a standalone crate placed inside the
//     SlateScript workspace's target/slattery_builds directory. Adding its
//     own [workspace] would make it a second workspace root, which Cargo
//     refuses when it's already inside another workspace.
//   • The `slate` path dependency uses a forward-slash path (safe on all
//     platforms) and is UNC-stripped so Cargo can parse it on Windows.
// ---------------------------------------------------------------------------
fn generate_cargo_toml(build_dir: &Path, app_name: &str, slate_dir: &Path) -> Result<(), String> {
    // Canonicalize and strip the \\?\ prefix so the path is usable as a
    // TOML string and Cargo accepts it on Windows.
    let abs_build_dir = strip_unc(
        fs::canonicalize(build_dir).unwrap_or_else(|_| build_dir.to_path_buf()),
    );
    let cargo_path = abs_build_dir.join("Cargo.toml");

    println!("[DEBUG] Writing Cargo.toml to: {}", cargo_path.display());

    let final_name = if app_name.is_empty() || app_name == "." {
        abs_build_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    } else {
        app_name.to_string()
    };

    // Convert to forward slashes for TOML portability.
    let slate_path = slate_dir.to_string_lossy().replace('\\', "/");
    println!("[DEBUG] slate_path: {}", slate_path);

    // Validate the name is a legal Cargo package name.
    let safe_name = final_name.replace(' ', "-");

    let content = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
slate = {{ path = "{slate}" }}

[target.'cfg(windows)'.dependencies]
winapi = {{ version = "0.3", features = ["winuser"] }}

[[bin]]
name = "{name}"
path = "src/main.rs"
"#,
        name = safe_name,
        slate = slate_path,
    );

    println!("[DEBUG] Cargo.toml content:\n{}", content);

    fs::write(&cargo_path, &content)
        .map_err(|e| format!("Failed to write Cargo.toml to {}: {}", cargo_path.display(), e))?;

    println!("[DEBUG] Successfully wrote Cargo.toml to {}", cargo_path.display());
    Ok(())
}

fn generate_main_rs(src_dir: &Path) -> Result<(), String> {
    let content = r#"// Auto-generated main.rs for Slattery app
// Uses dual-interpreter architecture (UI + AST)
// DO NOT EDIT - generated by `slate slattery build`

// Suppress the console window on Windows for GUI apps
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    // Resolve the app root relative to the executable's own directory,
    // so the app works regardless of the current working directory.
    let exe_path = std::env::current_exe()
        .expect("Failed to get executable path");
    let app_root = exe_path
        .parent()
        .expect("Executable has no parent directory");

    let main_st = app_root.join("src").join("main.st");

    if let Err(e) = slate::run_file(&main_st) {
        // With windows_subsystem = "windows" we have no console, so show a
        // message box on Windows and print on other platforms.
        let msg = format!("Slattery app error:\n{}", e);
        #[cfg(target_os = "windows")]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = OsStr::new(&msg)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let caption: Vec<u16> = OsStr::new("Error")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            unsafe {
                winapi::um::winuser::MessageBoxW(
                    std::ptr::null_mut(),
                    wide.as_ptr(),
                    caption.as_ptr(),
                    winapi::um::winuser::MB_OK | winapi::um::winuser::MB_ICONERROR,
                );
            }
        }
        #[cfg(not(target_os = "windows"))]
        eprintln!("{}", msg);

        std::process::exit(1);
    }
}
"#;

    fs::write(src_dir.join("main.rs"), content)
        .map_err(|e| format!("Failed to write main.rs: {}", e))
}

fn compile_app(build_dir: &Path, release: bool) -> Result<(), String> {
    let abs_path = strip_unc(
        fs::canonicalize(build_dir)
            .map_err(|e| format!("Failed to canonicalize path: {}", e))?,
    );

    let cargo_toml_path = abs_path.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return Err(format!("Cargo.toml not found at {}", cargo_toml_path.display()));
    }

    println!("[DEBUG] Building in: {}", abs_path.display());
    println!("[DEBUG] Using Cargo.toml: {}", cargo_toml_path.display());

    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    cmd.arg("--manifest-path");
    cmd.arg(&cargo_toml_path);
    if release {
        cmd.arg("--release");
    }

    println!(
        "[DEBUG] Running: cargo build{}",
        if release { " --release" } else { "" }
    );

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    if !status.success() {
        return Err("Cargo build failed".to_string());
    }

    Ok(())
}

fn copy_executable(
    build_dir: &Path,
    output_dir: &Path,
    app_name: &str,
    release: bool,
) -> Result<(), String> {
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    let abs_build_dir = strip_unc(
        fs::canonicalize(build_dir)
            .map_err(|e| format!("Failed to canonicalize build dir: {}", e))?,
    );

    let target_dir = if release {
        abs_build_dir.join("target").join("release")
    } else {
        abs_build_dir.join("target").join("debug")
    };

    let exe_name = get_exe_name(app_name);
    let exe_path = target_dir.join(&exe_name);

    if !exe_path.exists() {
        return Err(format!("Executable not found at: {}", exe_path.display()));
    }

    let dest_path = output_dir.join(&exe_name);
    fs::copy(&exe_path, &dest_path)
        .map_err(|e| format!("Failed to copy executable: {}", e))?;

    println!("[OK] Executable copied to: {}", dest_path.display());

    // Copy src/ next to the exe so run_file can find src/main.st.
    let src_dest = output_dir.join("src");
    fs::create_dir_all(&src_dest)
        .map_err(|e| format!("Failed to create output src dir: {}", e))?;
    copy_directory(&abs_build_dir.join("src"), &src_dest, "st")?;
    println!("[OK] Copied src/ next to executable");

    // Copy assets/ next to the exe (optional — only if present).
    let assets_src = abs_build_dir.join("assets");
    if assets_src.exists() {
        let assets_dest = output_dir.join("assets");
        fs::create_dir_all(&assets_dest)
            .map_err(|e| format!("Failed to create output assets dir: {}", e))?;
        copy_all_files(&assets_src, &assets_dest)?;
        println!("[OK] Copied assets/ next to executable");
    }

    Ok(())
}

fn get_exe_name(app_name: &str) -> String {
    if cfg!(windows) {
        format!("{}.exe", app_name)
    } else {
        app_name.to_string()
    }
}
