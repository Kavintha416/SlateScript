// src/extension/extension_trait.rs

use crate::lexer::Token;
use crate::ast::{Expression, Statement, Program, Span};
use crate::value::Value;
use crate::ast_interpreter::AstInterpreter;

/// Extension that can hook into the SlateScript pipeline
pub trait LanguageExtension: Send + Sync {
    /// Name of the extension
    fn name(&self) -> &str;
    
    /// Parse custom syntax - called by the parser when it encounters unknown tokens
    /// Returns (Expression, new_position) if this extension handles the token at `pos`
    fn parse_extension(&self, tokens: &[Token], pos: usize) -> Option<(Expression, usize)>;
    
    /// Interpret a custom expression - called by the interpreter
    fn interpret_extension(
        &self, 
        expr: &Expression, 
        interpreter: &mut AstInterpreter
    ) -> Result<Value, String>;
    
    /// Check if this extension handles a specific expression
    fn handles_expression(&self, expr: &Expression) -> bool;
    
    /// Post-process the entire AST after parsing (optional)
    fn post_process_ast(&self, program: &mut Program) -> Result<(), String> {
        Ok(()) // Default: no-op
    }
    
    /// Get custom tokens that this extension adds to the lexer
    fn custom_tokens(&self) -> Vec<CustomToken> {
        Vec::new() // Default: no custom tokens
    }
}

/// Custom token definition for lexer extensions
#[derive(Debug, Clone)]
pub struct CustomToken {
    pub keyword: String,
    pub token: Token,
}

/// Extension token with metadata
#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionToken {
    pub token: Token,
    pub extension_name: String,
}

/// Context passed to extension during interpretation
pub struct InterpreterContext<'a> {
    pub interpreter: &'a mut AstInterpreter,
    pub function_name: Option<&'a str>,
    pub call_depth: usize,
}

impl<'a> InterpreterContext<'a> {
    pub fn new(interpreter: &'a mut AstInterpreter) -> Self {
        Self {
            interpreter,
            function_name: None,
            call_depth: 0,
        }
    }
}

/// Result of extension parsing
#[derive(Debug, Clone)]
pub enum ExtensionParseResult {
    /// Extension handled this token and produced an expression
    Handled(Expression, usize),
    /// Extension didn't handle this token
    NotHandled,
    /// Extension handled this token but parsing should continue in extension
    ContinueInExtension(usize),
}