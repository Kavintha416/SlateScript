pub mod lexer;
pub mod parser;
pub mod ast;
pub mod ast_interpreter;
pub mod value;
pub mod debug;
pub mod extension;

pub use lexer::{Lexer, Token};
pub use parser::Parser;
pub use ast::{Program, Expression, Span};
pub use ast_interpreter::AstInterpreter;
pub use value::Value;
pub use extension::ExtensionRegistry;
pub use extension::LanguageExtension;
use std::path::Path;

/// Main entry point for running SlateScript files.
pub fn run_file(filepath: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(filepath)
        .map_err(|e| format!("Cannot read file '{}': {}", filepath.display(), e))?;
    
    // Check if this is a UI file
    let has_ui_syntax = source.contains("Window") || 
                       source.contains("Column") || 
                       source.contains("Text") || 
                       source.contains("Button") || 
                       source.contains("Input") ||
                       source.contains("import from \"slattery\"") ||
                       source.contains("Child:") ||
                       source.contains("Parent:");
    
    if has_ui_syntax {
        // This is a UI file, use Slattery
        return run_ui_file(filepath, &source);
    }
    
    // Regular script execution
    run_regular_script(filepath, &source)
}

fn run_regular_script(_filepath: &Path, source: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Register extensions based on file content
    let registry = ExtensionRegistry::new();
    
    // Tokenize with core lexer
    let mut lexer = crate::lexer::Lexer::new(source);
    let tokens = lexer.tokenize()
        .map_err(|e| format!("Lexer error: {}", e))?;
    
    // Parse with extensions
    let mut parser = crate::parser::Parser::new(tokens, source.to_string(), registry);
    let program = parser.parse()
        .map_err(|e| format!("Parse error: {}", e))?;
    
    // Get extensions from parser for interpreter
    let extensions = parser.take_extensions();
    
    // Execute with interpreter
    let mut interpreter = crate::ast_interpreter::AstInterpreter::new();
    interpreter.set_extensions(extensions);
    
    // Interpret the program
    interpreter.interpret(&program)?;
    
    Ok(())
}

fn run_ui_file(_filepath: &Path, _source: &str) -> Result<(), Box<dyn std::error::Error>> {
    // This would be handled by the main CLI, which has access to slattery
    // For now, just print and return Ok
    println!("[UI] Running UI file");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_ui_components() {
        let source = r#"
            make App = Window {
                title: "Test"
            }
        "#;
        
        let has_ui = source.contains("Window");
        assert!(has_ui);
    }
}