// slate-sfile/src/loader.rs

use std::path::Path;
use slate_core::lexer::{Lexer, Token};
use slate_core::parser::Parser;
use slate_core::ast::Program;
use slate_core::extension::ExtensionRegistry;

#[derive(Clone)]
pub struct FileLoader {
    extensions: ExtensionRegistry,
}

impl FileLoader {
    pub fn new() -> Self {
        Self {
            extensions: ExtensionRegistry::new(),
        }
    }

    pub fn read_file(&self, path: &Path) -> Result<String, String> {
        std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file '{}': {}", path.display(), e))
    }

    pub fn tokenize(&self, source: &str) -> Result<Vec<Token>, String> {
        let mut lexer = Lexer::new(source);
        lexer.tokenize()
            .map_err(|e| format!("Lexer error: {}", e))
    }

    pub fn parse(&self, tokens: &[Token]) -> Result<Program, String> {
        let source = tokens.iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>().join(" ");
        let mut parser = Parser::new(tokens.to_vec(), source, self.extensions.clone());
        parser.parse()
            .map_err(|e| format!("Parse error: {}", e))
    }
}