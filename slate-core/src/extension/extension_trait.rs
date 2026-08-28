// slate-core/src/extension/extension_trait.rs

use crate::lexer::Token;
use crate::ast::{Expression, Program};
use crate::value::Value;
use crate::ast_interpreter::AstInterpreter;
use std::any::Any;

pub trait LanguageExtension: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn parse_extension(&self, tokens: &[Token], pos: usize) -> Option<(Expression, usize)>;
    fn interpret_extension(&self, expr: &Expression, interpreter: &mut AstInterpreter) -> Result<Value, String>;
    fn handles_expression(&self, expr: &Expression) -> bool;
    fn post_process_ast(&mut self, _program: &mut Program) -> Result<(), String> {
        Ok(())
    }
    fn custom_tokens(&self) -> Vec<CustomToken> {
        Vec::new()
    }
    fn clone_box(&self) -> Box<dyn LanguageExtension>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl Clone for Box<dyn LanguageExtension> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Debug, Clone)]
pub struct CustomToken {
    pub keyword: String,
    pub token: Token,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionToken {
    pub token: Token,
    pub extension_name: String,
}

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

#[derive(Debug, Clone)]
pub enum ExtensionParseResult {
    Handled(Expression, usize),
    NotHandled,
    ContinueInExtension(usize),
}