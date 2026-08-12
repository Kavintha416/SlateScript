// src/slattery/install.rs

use std::fs;
use std::path::Path;
use std::io::Write;

// Embed your logo as a byte array at compile time
// Place logo.png in the project root (SlateScript-main/logo.png)
pub const LOGO_PNG: &[u8] = include_bytes!("../../logo.png");

pub fn create_app_environment(name: &str) -> Result<(), std::io::Error> {
    println!("Creating Slattery application: {}", name);
    println!("+-------------------------------------------+");
    
    // Create folder structure
    fs::create_dir_all(format!("{}/src", name))?;
    println!("  [OK] Created src/");
    fs::create_dir_all(format!("{}/assets", name))?;
    println!("  [OK] Created assets/");
    fs::create_dir_all(format!("{}/styles", name))?;
    println!("  [OK] Created styles/");
    
    // Write your logo from embedded bytes
    fs::write(format!("{}/assets/logo.png", name), LOGO_PNG)?;
    println!("  [OK] Created assets/logo.png");
    
    // Create the main.st template
    let main_template = r#"import from "slattery" {Window, Column, Text, Button, Identity, Rewrite}

make App = Window {
    title: "My Slattery App",
    Child: <Column> {
        Child: <Text> {
            value: "Welcome to your new App!"
        },
        Child: <Button> {
            label: "Click me!",
            on_click: main
        }
    }
}

func main<> {
    write("Hello World")
}
"#;
    fs::write(format!("{}/src/main.st", name), main_template)?;
    println!("  [OK] Created src/main.st");
    
    // Create styles template
    let main_style_template = r#"/* Main Styles for Test App */

/* Component-specific styles */
@Text {
    color: #1A1A1A;
    font-size: 18px;
}

@Button {
    background-color: #FF3B30;
    color: white;
    border-radius: 8px;
    font-size: 16px;
}

@Button:hover {
    background-color: #FF6B60;
}

/* ID-based styles */
#title_text {
    color: #FF3B30;
    font-size: 24px;
    font-weight: bold;
    background-color: #F5F5F5;
    padding: 16px;
    border-radius: 4px;
}

#subtitle_text {
    color: #666666;
    font-size: 16px;
    font-style: italic;
}

#test_button {
    background-color: #4CAF50;
    color: white;
    border-radius: 12px;
    font-size: 18px;
    padding: 12px 24px;
}

/* Window styles */
@Window {
    background-color: #FFFFFF;
}
"#;
    fs::write(format!("{}/styles/main.sts", name), main_style_template)?;
    println!("  [OK] Created styles/main.sts");
    
    // Create README.txt
    let readme = format!(r#"================================================
{} - Slattery App
================================================

A Slattery application built with SlateScript.

Project Structure:
------------------
{app}/
├── src/
│   └── main.st      # Main application code
├── styles/
│   └── main.sts     # Style definitions
├── assets/
│   └── logo.png     # Application icon
└── Slattery.toml    # Project configuration

Running:
--------
slate run {app}/src/main.st

Building:
---------
slate slattery build {app}

"#, name, app = name);

    fs::write(format!("{}/README.txt", name), readme)?;
    println!("  [OK] Created README.txt");
    
    // Create Slattery.toml project file
    let toml_content = format!(r#"[project]
name = "{}"
version = "0.1.0"
authors = ["Your Name <your@email.com>"]
description = "A Slattery application"
edition = "2024"

[app]
entry = "src/main.st"
styles_dir = "styles"
assets_dir = "assets"

[dependencies]
slattery = "1.0.0"

[build]
output_dir = "build"
release = false
"#, name);
    
    fs::write(format!("{}/Slattery.toml", name), toml_content)?;
    println!("  [OK] Created Slattery.toml");
    
    println!("+-------------------------------------------+");
    println!("[OK] Application '{}' created successfully!", name);
    println!("[INFO] Next steps:");
    println!("  1. cd {}", name);
    println!("  2. slate run src/main.st");
    println!("+-------------------------------------------+");
    
    Ok(())
}