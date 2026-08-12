// src/extension/registry.rs

use std::collections::HashMap;
use super::extension_trait::LanguageExtension;
use crate::lexer::Token;        // ADD THIS
use crate::ast::Expression;     // ADD THIS
use crate::value::Value;

#[derive(Default)]  // ADD THIS DERIVE
pub struct ExtensionRegistry {
    extensions: HashMap<String, Box<dyn LanguageExtension>>,
    parse_order: Vec<String>,
}

impl ExtensionRegistry {
    pub fn get_extensions_mut(&mut self) -> Vec<&mut Box<dyn LanguageExtension>> {
        self.extensions.values_mut().collect()
    }
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
            parse_order: Vec::new(),
        }
    }
    
    pub fn register(&mut self, extension: Box<dyn LanguageExtension>) {
        let name = extension.name().to_string();
        if !self.extensions.contains_key(&name) {
            self.parse_order.push(name.clone());
        }
        self.extensions.insert(name, extension);
    }
    
    pub fn get(&self, name: &str) -> Option<&Box<dyn LanguageExtension>> {
        self.extensions.get(name)
    }
    
    pub fn get_extensions(&self) -> Vec<&Box<dyn LanguageExtension>> {
        self.extensions.values().collect()
    }
    
    /// Try to parse with extensions in order
    pub fn try_parse_extension(
        &self, 
        tokens: &[Token], 
        pos: usize
    ) -> Option<(Expression, usize)> {
        for name in &self.parse_order {
            if let Some(ext) = self.extensions.get(name) {
                if let Some((expr, new_pos)) = ext.parse_extension(tokens, pos) {
                    return Some((expr, new_pos));
                }
            }
        }
        None
    }
    
    /// Check if any extension handles this expression
    pub fn handles_expression(&self, expr: &Expression) -> bool {
        for ext in self.extensions.values() {
            if ext.handles_expression(expr) {
                return true;
            }
        }
        false
    }
    
    /// Interpret an expression with extensions
    pub fn interpret_with_extensions(
        &self,
        expr: &Expression,
        interpreter: &mut crate::ast_interpreter::AstInterpreter,
    ) -> Option<Result<Value, String>> {
        for ext in self.extensions.values() {
            if ext.handles_expression(expr) {
                return Some(ext.interpret_extension(expr, interpreter));
            }
        }
        None
    }
    
    /// Get all custom tokens from all extensions
    pub fn custom_tokens(&self) -> Vec<super::extension_trait::CustomToken> {
        let mut tokens = Vec::new();
        for ext in self.extensions.values() {
            tokens.extend(ext.custom_tokens());
        }
        tokens
    }

    pub fn take_extensions(&mut self) -> Vec<Box<dyn LanguageExtension>> {
        self.extensions.drain().map(|(_, ext)| ext).collect()
    }
}