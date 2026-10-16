//! Run a Slattery app with optional per-app window icon.
//!
//! `slattery run <app>` reads `<app>/src/main.st` and renders it with
//! `<app>/assets/logo.png` as the window icon (if it exists). Falls
//! back to the default SlateScript logo otherwise.

use std::path::Path;

use slate_core::extension::ExtensionRegistry;
use slate_core::{Lexer, Parser};
use slate_sfile::SFileExtension;

use crate::icons;
use crate::ui_extension::SlatteryExtension;
use crate::ui_integration::UiFramework;

pub fn run_app(main_st: &Path, icon_path: Option<&Path>) -> Result<(), String> {
    let source = std::fs::read_to_string(main_st)
        .map_err(|e| format!("cannot read '{}': {}", main_st.display(), e))?;

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(SFileExtension::new()));
    registry.register(Box::new(SlatteryExtension::new()));

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize().map_err(|e| e.to_string())?;

    let mut parser = Parser::new(tokens, source.clone(), registry);
    let program = parser.parse().map_err(|e| e.to_string())?;

    // Icon resolution:
    //   1. If the app has its own assets/logo.png, use that.
    //   2. Otherwise, use the default SlateScript logo.
    let icon = icon_path
        .filter(|p| p.exists())
        .and_then(|p| icons::load_from_png(p).ok())
        .or_else(icons::default_icon);

    let mut framework = UiFramework::new();
    framework.render_ast(&program, Some(main_st), icon)
}