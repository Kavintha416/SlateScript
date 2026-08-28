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
        
        let app_root = if Path::new(app_name).exists() {
            PathBuf::from(app_name)
        } else {
            let current = std::env::current_dir()
                .map_err(|e| format!("Failed to get current dir: {}", e))?;
            let possible = current.join(app_name);
            if possible.exists() {
                possible
            } else {
                return Err(format!("App '{}' not found", app_name));
            }
        };
        
        println!("[DEBUG] app_root: {}", app_root.display());

        let project_name = read_project_name(&app_root)?;
        println!("[DEBUG] project_name from Slattery.toml: {}", project_name);
        
        let final_name = if project_name == "." || project_name.is_empty() {
            app_root.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
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
        if line.starts_with("name = ") {
            let name = line
                .trim_start_matches("name = ")
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            println!("[DEBUG] Found name: {}", name);
            if !name.is_empty() {
                return Ok(name);
            }
        }
    }

    let folder_name = app_root.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
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
            "App '{}' not found. Run 'slate slattery new {}' first.",
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

    // Copy .st source files only
    copy_directory(&config.app_root.join("src"), &build_src_dir, "st")?;
    println!("  [OK] Copied source files");

    // Styles are embedded in .st files - no separate copy needed

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
    println!("  Executable: {}/{}", config.output_dir.display(), get_exe_name(&config.app_name));
    println!("  Run: ./{}/{}", config.output_dir.display(), get_exe_name(&config.app_name));
    println!("+-------------------------------------------+");

    Ok(())
}

pub fn find_slate_dir() -> Result<PathBuf, String> {
    // Check SLATE_HOME first
    if let Ok(dir) = std::env::var("SLATE_HOME") {
        let path = PathBuf::from(dir);
        if path.join("Cargo.toml").exists() {
            return Ok(path);
        }
    }

    // Check current executable path
    if let Ok(exe_path) = std::env::current_exe() {
        let mut current = exe_path.parent().unwrap_or(&PathBuf::from(".")).to_path_buf();
        
        for _ in 0..10 {
            if current.join("Cargo.toml").exists() {
                let content = fs::read_to_string(current.join("Cargo.toml"))
                    .unwrap_or_default();
                if content.contains("name = \"slate\"") {
                    return Ok(current);
                }
            }
            if !current.pop() {
                break;
            }
        }
    }

    // Check current directory
    let mut current = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    for _ in 0..10 {
        if current.join("Cargo.toml").exists() {
            let content = fs::read_to_string(current.join("Cargo.toml"))
                .unwrap_or_default();
            if content.contains("name = \"slate\"") {
                return Ok(current);
            }
        }
        if !current.pop() {
            break;
        }
    }

    // Check common locations based on OS
    #[cfg(windows)]
    let common_paths = vec![
        std::env::var("USERPROFILE").unwrap_or_default() + "\\Desktop\\SlateScript-main",
        std::env::var("USERPROFILE").unwrap_or_default() + "\\SlateScript",
        "C:\\Program Files\\SlateScript".to_string(),
    ];

    #[cfg(unix)]
    let common_paths = vec![
        std::env::var("HOME").unwrap_or_default() + "/SlateScript",
        std::env::var("HOME").unwrap_or_default() + "/Desktop/SlateScript-main",
    ];

    for path in common_paths {
        let p = PathBuf::from(path);
        if p.join("Cargo.toml").exists() {
            let content = fs::read_to_string(p.join("Cargo.toml"))
                .unwrap_or_default();
            if content.contains("name = \"slate\"") {
                return Ok(p);
            }
        }
    }

    Err("Could not find SlateScript project directory. Please set SLATE_HOME environment variable to the SlateScript project path.".to_string())
}

fn copy_directory(src: &Path, dest: &Path, ext: &str) -> Result<(), String> {
    if !src.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read dir {}: {}", src.display(), e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == ext {
                    let dest_path = dest.join(path.file_name().unwrap());
                    fs::copy(&path, &dest_path)
                        .map_err(|e| format!("Failed to copy {}: {}", path.display(), e))?;
                }
            }
        }
    }
    Ok(())
}

fn copy_all_files(src: &Path, dest: &Path) -> Result<(), String> {
    if !src.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read dir {}: {}", src.display(), e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if path.is_file() {
            let dest_path = dest.join(path.file_name().unwrap());
            fs::copy(&path, &dest_path)
                .map_err(|e| format!("Failed to copy {}: {}", path.display(), e))?;
        }
    }
    Ok(())
}

fn generate_cargo_toml(build_dir: &Path, app_name: &str, slate_dir: &Path) -> Result<(), String> {
    let abs_build_dir = build_dir.canonicalize()
        .unwrap_or_else(|_| build_dir.to_path_buf());
    
    let cargo_path = abs_build_dir.join("Cargo.toml");
    println!("[DEBUG] Writing Cargo.toml to: {}", cargo_path.display());
    println!("[DEBUG] app_name: {}", app_name);
    
    let final_name = if app_name == "." || app_name.is_empty() {
        abs_build_dir.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    } else {
        app_name.to_string()
    };
    println!("[DEBUG] final_name: {}", final_name);
    
    let slate_path = slate_dir.to_string_lossy().replace('\\', "/");
    println!("[DEBUG] slate_path: {}", slate_path);
    
    let content = format!(r#"[workspace]
members = ["."]

[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
slate = {{ path = "{}" }}

[target.'cfg(windows)'.dependencies]
winapi = {{ version = "0.3", features = ["winuser"] }}

[[bin]]
name = "{}"
path = "src/main.rs"
"#,
        final_name,
        slate_path,
        final_name
    );

    println!("[DEBUG] Cargo.toml content:\n{}", content);

    fs::write(&cargo_path, content)
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
    let abs_path = std::fs::canonicalize(build_dir)
        .map_err(|e| format!("Failed to canonicalize path: {}", e))?;

    let abs_path_str = abs_path.to_string_lossy().to_string();
    let clean_path_str = abs_path_str.trim_start_matches("\\\\?\\");
    let abs_path = PathBuf::from(clean_path_str);

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

    println!("[DEBUG] Running: cargo build{}", if release { " --release" } else { "" });
    println!("[DEBUG] In directory: {}", abs_path.display());

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    if !status.success() {
        return Err("Cargo build failed".to_string());
    }

    Ok(())
}

fn copy_executable(build_dir: &Path, output_dir: &Path, app_name: &str, release: bool) -> Result<(), String> {
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    let abs_build_dir = std::fs::canonicalize(build_dir)
        .map_err(|e| format!("Failed to canonicalize build dir: {}", e))?;

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

    // Copy src/ next to the exe so run_file can find src/main.st
    let src_dest = output_dir.join("src");
    fs::create_dir_all(&src_dest)
        .map_err(|e| format!("Failed to create output src dir: {}", e))?;
    copy_directory(&abs_build_dir.join("src"), &src_dest, "st")?;
    println!("[OK] Copied src/ next to executable");

    // Styles are embedded in .st files - no separate copy needed

    // Copy assets/ next to the exe (optional — only if present)
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
    #[cfg(windows)]
    {
        format!("{}.exe", app_name)
    }
    #[cfg(not(windows))]
    {
        app_name.to_string()
    }
}