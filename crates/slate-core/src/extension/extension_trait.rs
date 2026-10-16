// slate-core/src/extension/extension_trait.rs
//
// The LanguageExtension trait.
//
// An extension declares its name, claims keywords via `custom_tokens`,
// and (optionally) parses statements and/or expressions from the token
// stream. Extensions that don't claim any syntax still register — they
// just never return Handled from either parse method.

use crate::lexer::Token;
use crate::ast::{Expression, Program, Statement};
use crate::value::Value;
use crate::ast_interpreter::AstInterpreter;
use std::any::Any;

// ============================================================================
//  ExtensionParse — the three-way result of an extension parse attempt
// ============================================================================

/// What an extension says when asked "is this construct yours?"
///
/// Using a three-way enum rather than `Option` means an extension can
/// say "this IS mine but it's malformed, here's the error" without the
/// parser misinterpreting the None as "not mine, try the next one."
#[derive(Debug, Clone)]
pub enum ExtensionParse<T> {
    /// The extension consumed tokens from `pos` up to some new position
    /// and produced a node.
    Handled(T, usize),

    /// The extension recognized this construct but it's malformed.
    /// The parser stops and reports this error. No further extensions
    /// get a turn, and core parsing does not run.
    Failed(String),

    /// The extension doesn't recognize this. Parser tries the next one.
    NotMine,
}

// ============================================================================
//  LanguageExtension
// ============================================================================

pub trait LanguageExtension: Send + Sync + 'static {
    /// Unique name for this extension (e.g. "slattery", "sfile", "sllm").
    fn name(&self) -> &str;

    /// Keywords this extension claims. The parser uses this to resolve
    /// `Token::Identifier("Window")` into
    /// `Token::Extension { name: "slattery", keyword: "Window" }`.
    ///
    /// Returning an empty Vec is fine — the extension just won't have
    /// any keyword claims (useful for extensions that only modify
    /// behavior via hooks, not syntax).
    fn custom_tokens(&self) -> Vec<CustomToken> {
        Vec::new()
    }

    // ------------------------------------------------------------------
    //  Parsing hooks
    // ------------------------------------------------------------------

    /// Try to parse an expression starting at `pos`.
    ///
    /// Called before core expression parsing at every expression site.
    /// Return `Handled(expr, new_pos)` if you consumed tokens,
    /// `Failed(msg)` if you recognized the syntax but it's broken,
    /// `NotMine` if this isn't yours.
    fn parse_extension_expression(
        &self,
        _tokens: &[Token],
        _pos: usize,
    ) -> ExtensionParse<Expression> {
        ExtensionParse::NotMine
    }

    /// Try to parse a statement starting at `pos`.
    ///
    /// Called before core statement parsing at every statement site.
    /// Same return semantics as `parse_extension_expression`.
    fn parse_extension_statement(
        &self,
        _tokens: &[Token],
        _pos: usize,
    ) -> ExtensionParse<Statement> {
        ExtensionParse::NotMine
    }

    // ------------------------------------------------------------------
    //  Interpretation hooks
    // ------------------------------------------------------------------

    /// Execute an extension expression. Called by the interpreter when
    /// it encounters `Expression::Extension`.
    fn interpret_extension(
        &self,
        expr: &Expression,
        interpreter: &mut AstInterpreter,
    ) -> Result<Value, String>;

    /// Does this extension handle the given expression?
    fn handles_expression(&self, expr: &Expression) -> bool;

    // ------------------------------------------------------------------
    //  Post-processing hook
    // ------------------------------------------------------------------

    /// Called after the full program is parsed. Use this for passes that
    /// need to see the whole AST (e.g. resolving imports).
    fn post_process_ast(&mut self, _program: &mut Program) -> Result<(), String> {
        Ok(())
    }

    // ------------------------------------------------------------------
    //  Clone / downcast plumbing
    // ------------------------------------------------------------------

    fn clone_box(&self) -> Box<dyn LanguageExtension>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl Clone for Box<dyn LanguageExtension> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

// ============================================================================
//  CustomToken — how extensions declare their keywords
// ============================================================================

#[derive(Debug, Clone)]
pub struct CustomToken {
    /// The raw keyword string the lexer will see, e.g. "Window", "on_click".
    pub keyword: String,

    /// The token the parser should synthesize when it sees this keyword.
    /// Almost always `Token::Extension { name, keyword }`.
    pub token: Token,
}

// ============================================================================
//  Legacy / utility types
// ============================================================================

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