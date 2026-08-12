//! UI Integration Module
//! 
//! Provides integration between the UI lexer, interpreter, and HTML renderer
//! to create a complete SlateScript UI framework.

use crate::slattery::sla_lexer::UiLexer;
use crate::slattery::sla_interpreter::UiInterpreter;
use crate::slattery::egui_renderer::EguiRenderer;
use std::path::Path;

pub struct UiFramework {
    lexer: UiLexer,
    interpreter: UiInterpreter,
    renderer: EguiRenderer,
}

impl UiFramework {
    pub fn new() -> Self {
        Self {
            lexer: UiLexer::new(""),
            interpreter: UiInterpreter::new(),
            renderer: EguiRenderer::new(),
        }
    }
    
    /// Parse and render a UI source file.
    ///
    /// `script_path` is the path to the `.st` file being run. It is used to
    /// locate the `styles/` directory relative to the app root (the parent of
    /// the `src/` directory that contains the script).
    pub fn parse_and_render(&mut self, source: &str, script_path: Option<&Path>) -> Result<(), String> {
        // Tokenize source code
        self.lexer = UiLexer::new(source);
        let tokens = self.lexer.tokenize();
        
        // Interpret tokens into components
        let components = self.interpreter.interpret(tokens)
            .map_err(|e| format!("UI interpretation error: {}", e))?;
        
        // Convert Vec to HashMap for egui renderer
        let mut component_map = std::collections::HashMap::new();
        for (index, comp) in components.iter().enumerate() {
            let comp_borrowed = comp.borrow();
            
            // Add by component type for all components
            component_map.insert(comp_borrowed.component_type.clone(), comp.clone());
            
            // Also add by identity if available
            if let Some(name) = comp_borrowed.identity.as_ref() {
                component_map.insert(name.clone(), comp.clone());
            }
            
            // Also add by index to ensure all components are accessible
            component_map.insert(format!("component:{}", index), comp.clone());
            
            println!("[DEBUG] Transferring component: {} (identity: {:?})", comp_borrowed.component_type, comp_borrowed.identity);
        }
        println!("[DEBUG] Total components transferred: {}", component_map.len());
        std::io::Write::flush(&mut std::io::stdout()).unwrap_or_default();
        
        // Resolve the styles directory relative to the script file.
        // For a script at <app_root>/src/main.st the app root is the parent of
        // the `src` directory, so styles live at <app_root>/styles/.
        let style_files = collect_style_files(script_path);
        
        // Transfer UI functions to the renderer
        let mut renderer = crate::slattery::egui_renderer::EguiRenderer::new();
        renderer.set_components(component_map);
        
        // Register UI functions with the main interpreter
        for (func_name, func_tokens) in &self.interpreter.functions {
            println!("[DEBUG] Registering UI function: {}", func_name);
            // Convert UI tokens to main interpreter tokens and register
            let mut main_tokens = Vec::new();
            for token in func_tokens {
                match token {
                    crate::slattery::sla_lexer::UiToken::Identifier(s) => {
                        main_tokens.push(crate::lexer::Token::Identifier(s.clone()));
                    }
                    crate::slattery::sla_lexer::UiToken::String(s) => {
                        main_tokens.push(crate::lexer::Token::String(s.clone()));
                    }
                    crate::slattery::sla_lexer::UiToken::Number(n) => {
                        main_tokens.push(crate::lexer::Token::Number(*n as i64));
                    }
                    crate::slattery::sla_lexer::UiToken::True => {
                        main_tokens.push(crate::lexer::Token::True);
                    }
                    crate::slattery::sla_lexer::UiToken::False => {
                        main_tokens.push(crate::lexer::Token::False);
                    }
                    crate::slattery::sla_lexer::UiToken::LeftParen => {
                        main_tokens.push(crate::lexer::Token::LeftParen);
                    }
                    crate::slattery::sla_lexer::UiToken::RightParen => {
                        main_tokens.push(crate::lexer::Token::RightParen);
                    }
                    crate::slattery::sla_lexer::UiToken::Comma => {
                        main_tokens.push(crate::lexer::Token::Comma);
                    }
                    _ => {
                        println!("[DEBUG] Skipping unsupported token in function: {:?}", token);
                    }
                }
            }
            
            // Create a function definition in the main interpreter
            let mut func_def_tokens = Vec::new();
            func_def_tokens.push(crate::lexer::Token::Func);
            func_def_tokens.push(crate::lexer::Token::Identifier(func_name.clone()));
            func_def_tokens.push(crate::lexer::Token::LessThan);
            func_def_tokens.push(crate::lexer::Token::GreaterThan);
            func_def_tokens.extend(main_tokens);
            
            // Execute the function definition to register it
            if let Err(e) = renderer.interpreter.run(&func_def_tokens) {
                eprintln!("[ERROR] Failed to register UI function '{}': {}", func_name, e);
            }
        }
        
        // Load styles and run the app
        renderer.load_styles(&style_files);
        crate::slattery::egui_renderer::run_egui_app_with_renderer(renderer)
            .map_err(|e| format!("Failed to render egui app: {}", e))
    }
}

/// Collect all `.sts` files from the styles directory that belongs to the
/// project containing `script_path`.
///
/// Resolution order (first directory that contains `.sts` files wins):
/// 1. `<app_root>/styles/`  where app_root = canonical(script_path).parent().parent()
/// 2. `<script_dir>/styles/`
/// 3. Walk up from CWD looking for a `styles/` directory (up to 4 levels)
/// 4. `styles/` relative to CWD
pub fn collect_style_files(script_path: Option<&Path>) -> Vec<String> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();

    if let Some(path) = script_path {
        // Canonicalize so relative paths like "main.st" get the full CWD prefix
        let abs = path.canonicalize().unwrap_or_else(|_| {
            std::env::current_dir().unwrap_or_default().join(path)
        });

        // <app_root>/styles  (e.g. testapp/styles when script is testapp/src/main.st)
        if let Some(app_root) = abs.parent().and_then(|p| p.parent()) {
            candidates.push(app_root.join("styles"));
        }
        // <script_dir>/styles
        if let Some(script_dir) = abs.parent() {
            candidates.push(script_dir.join("styles"));
        }
    }

    // Walk up from CWD (handles `slate run main.st` from inside testapp/src/)
    if let Ok(mut dir) = std::env::current_dir() {
        for _ in 0..4 {
            let candidate = dir.join("styles");
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
            if !dir.pop() { break; }
        }
    }

    for styles_dir in &candidates {
        if !styles_dir.exists() {
            continue;
        }

        let mut found: Vec<String> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(styles_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("sts") {
                    println!("[INFO] Found style file: {}", p.display());
                    found.push(p.to_string_lossy().to_string());
                }
            }
        }

        if !found.is_empty() {
            println!("[INFO] Loading styles from: {}", styles_dir.display());
            return found;
        }
    }

    println!("[INFO] No style files found");
    Vec::new()
}

impl Default for UiFramework {
    fn default() -> Self {
        Self::new()
    }
}
