// src/lib.rs

// Core modules
pub mod lexer;
pub mod debug;
pub mod value;
pub mod ast;
pub mod parser;
pub mod ast_interpreter;
pub mod slattery;

// Extension system (new)
pub mod extension;

use std::path::Path;
use crate::extension::ExtensionRegistry;
use crate::slattery::ui_extension::SlatteryExtension;

/// Main entry point for running SlateScript files.
/// 
/// This uses a unified parser and interpreter with optional extensions.
/// UI syntax is handled by the Slattery extension when detected.
pub fn run_file(filepath: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use crate::debug::{check_common_mistakes, DiagnosticsEngine};
    
    let source = std::fs::read_to_string(filepath)
        .map_err(|e| format!("Cannot read file '{}': {}", filepath.display(), e))?;
    
    // Check for common syntax mistakes
    let (diagnostics, has_errors) = check_common_mistakes(&source);
    for diag in diagnostics {
        DiagnosticsEngine::print(&diag);
    }
    
    if has_errors {
        return Err("Syntax errors found. Fix them before running.".into());
    }
    
    #[cfg(not(windows))]
    println!("Running: {}\n", filepath.display());
    
    // Register extensions based on file content
    let mut registry = ExtensionRegistry::new();
    register_extensions(&source, &mut registry);
    
    // Tokenize with core lexer
    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize()
        .map_err(|e| format!("Lexer error: {}", e))?;
    
    // Parse with extensions
    let mut parser = parser::Parser::new(tokens, source.clone(), registry);
    let program = parser.parse()
        .map_err(|e| format!("Parse error: {}", e))?;
    
    // Get extensions from parser for interpreter
    let extensions = parser.take_extensions();
    
    // Execute with interpreter
    let mut interpreter = ast_interpreter::AstInterpreter::new();
    interpreter.set_extensions(extensions);
    
    // Check if we have UI components to render
    let has_ui = detect_ui_components(&program);
    
    if has_ui {
        // Run UI program with egui renderer
        run_ui_program(&program, filepath, &source)?;
    } else {
        // Run regular program
        interpreter.interpret(&program)?;
    }
    
    #[cfg(not(windows))]
    println!("\n[OK] Script completed successfully!");
    
    Ok(())
}

fn register_extensions(source: &str, registry: &mut ExtensionRegistry) {
    let has_ui_syntax = source.contains("Window") || 
                        source.contains("Column") || 
                        source.contains("Text") || 
                        source.contains("Button") || 
                        source.contains("Input");
    
    if has_ui_syntax {
        println!("[INFO] Detected UI syntax, enabling Slattery extension");
        registry.register(Box::new(SlatteryExtension::new()));
    }
}

/// Detect if a program contains UI components
fn detect_ui_components(program: &ast::Program) -> bool {
    for stmt in &program.statements {
        if let ast::Statement::Expression(expr) = stmt {
            if contains_ui_expression(expr) {
                return true;
            }
        }
    }
    false
}

/// Recursively check if an expression contains UI elements
fn contains_ui_expression(expr: &ast::Expression) -> bool {
    match expr {
        ast::Expression::Extension { name, .. } => {
            name == "slattery"
        }
        ast::Expression::Binary { left, right, .. } => {
            contains_ui_expression(left) || contains_ui_expression(right)
        }
        ast::Expression::FunctionCall { arguments, .. } => {
            arguments.iter().any(contains_ui_expression)
        }
        ast::Expression::Array(elements, _) => {
            elements.iter().any(contains_ui_expression)
        }
        ast::Expression::Object(properties, _) => {
            properties.values().any(contains_ui_expression)
        }
        _ => false,
    }
}

/// Run a program with UI components using egui renderer
fn run_ui_program(
    program: &ast::Program,
    filepath: &Path,
    source: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::slattery::egui_renderer::EguiRenderer;
    use crate::slattery::sla_interpreter::UiInterpreter;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::cell::RefCell;
    
    println!("[INFO] Running UI program with Slattery...");
    
    // Extract UI components from the program
    // This is where we'd walk the AST and collect UI components
    // For now, we use the existing UI interpreter as a bridge
    let ui_interpreter = extract_ui_components(program, source)?;
    
    // Build component map
    let mut component_map = HashMap::new();
    for component in ui_interpreter.components.values() {
        let comp = component.borrow();
        let key = if let Some(name) = comp.identity.as_ref() {
            name.clone()
        } else {
            comp.component_type.clone()
        };
        component_map.insert(key, component.clone());
    }
    
    // Find style files
    let style_files = slattery::ui_integration::collect_style_files(Some(filepath));
    
    // Create renderer
    let mut renderer = EguiRenderer::new();
    
    // Register UI functions with the renderer's interpreter
    for (func_name, func_tokens) in &ui_interpreter.functions {
        register_ui_function(&mut renderer, func_name, func_tokens)?;
    }
    
    // Set components and load styles
    renderer.set_components(component_map);
    renderer.load_styles(&style_files);
    
    // Run the egui app
    crate::slattery::egui_renderer::run_egui_app_with_renderer(renderer)
        .map_err(|e| format!("Failed to run UI: {}", e))?;
    
    Ok(())
}

/// Extract UI components using the UI interpreter (transitional)
fn extract_ui_components(
    _program: &ast::Program,
    source: &str,
) -> Result<crate::slattery::sla_interpreter::UiInterpreter, Box<dyn std::error::Error>> {
    // For the transition period, we still use the UI lexer/interpreter
    // This will be replaced with direct AST walking once the extension system is complete
    use crate::slattery::sla_lexer::UiLexer;
    use crate::slattery::sla_interpreter::UiInterpreter;
    
    let mut ui_lexer = UiLexer::new(source);
    let ui_tokens = ui_lexer.tokenize();
    
    let mut ui_interpreter = UiInterpreter::new();
    ui_interpreter.interpret(ui_tokens)
        .map_err(|e| format!("UI interpretation error: {}", e))?;
    
    Ok(ui_interpreter)
}

/// Register a UI function with the renderer's interpreter
fn register_ui_function(
    renderer: &mut crate::slattery::egui_renderer::EguiRenderer,
    func_name: &str,
    func_tokens: &[crate::slattery::sla_lexer::UiToken],
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::lexer::Token;
    
    let mut main_tokens = Vec::new();
    
    // func name<>
    main_tokens.push(Token::Func);
    main_tokens.push(Token::Identifier(func_name.to_string()));
    main_tokens.push(Token::LessThan);
    main_tokens.push(Token::GreaterThan);
    main_tokens.push(Token::LeftBrace);
    
    // Convert function body tokens
    for token in func_tokens {
        match token {
            crate::slattery::sla_lexer::UiToken::Identifier(s) => {
                main_tokens.push(Token::Identifier(s.clone()));
            }
            crate::slattery::sla_lexer::UiToken::String(s) => {
                main_tokens.push(Token::String(s.clone()));
            }
            crate::slattery::sla_lexer::UiToken::Number(n) => {
                main_tokens.push(Token::Number(*n as i64));
            }
            crate::slattery::sla_lexer::UiToken::True => {
                main_tokens.push(Token::True);
            }
            crate::slattery::sla_lexer::UiToken::False => {
                main_tokens.push(Token::False);
            }
            crate::slattery::sla_lexer::UiToken::LeftParen => {
                main_tokens.push(Token::LeftParen);
            }
            crate::slattery::sla_lexer::UiToken::RightParen => {
                main_tokens.push(Token::RightParen);
            }
            crate::slattery::sla_lexer::UiToken::LeftBrace => {
                main_tokens.push(Token::LeftBrace);
            }
            crate::slattery::sla_lexer::UiToken::RightBrace => {
                main_tokens.push(Token::RightBrace);
            }
            crate::slattery::sla_lexer::UiToken::Comma => {
                main_tokens.push(Token::Comma);
            }
            crate::slattery::sla_lexer::UiToken::Equal => {
                main_tokens.push(Token::Equal);
            }
            crate::slattery::sla_lexer::UiToken::Semicolon => {
                main_tokens.push(Token::Semicolon);
            }
            _ => {}
        }
    }
    
    main_tokens.push(Token::RightBrace);
    main_tokens.push(Token::EOF);
    
    if let Err(e) = renderer.interpreter.run(&main_tokens) {
        eprintln!("[WARN] Failed to register function '{}': {}", func_name, e);
    } else {
        println!("[OK] Registered function: {}", func_name);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_ui_components() {
        // Test with a simple UI component
        let source = r#"
            make App = Window {
                title: "Test"
            }
        "#;
        // This would need a proper parser to test fully
        // For now, just test the detection function
        let has_ui = source.contains("Window");
        assert!(has_ui);
    }
}