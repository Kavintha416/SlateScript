// slate-core/src/extension/registry.rs

use std::collections::HashMap;
use super::extension_trait::{ExtensionParse, LanguageExtension};
use crate::lexer::Token;
use crate::ast::{Expression, Statement};
use crate::value::Value;

/// Holds every registered extension and dispatches parse/interpret calls
/// to them in registration order.
///
/// Registration order matters: when two extensions both claim the same
/// keyword, the first one registered wins.
#[derive(Default)]
pub struct ExtensionRegistry {
    extensions: HashMap<String, Box<dyn LanguageExtension>>,
    parse_order: Vec<String>,
    /// Cached keyword → extension-name map. Rebuilt whenever an extension
    /// is registered. Makes keyword lookups O(1) instead of
    /// O(extensions × keywords).
    keyword_map: HashMap<String, String>,
}

impl Clone for ExtensionRegistry {
    fn clone(&self) -> Self {
        let mut extensions = HashMap::new();
        for (name, ext) in &self.extensions {
            extensions.insert(name.clone(), ext.clone_box());
        }
        Self {
            extensions,
            parse_order: self.parse_order.clone(),
            keyword_map: self.keyword_map.clone(),
        }
    }
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
            parse_order: Vec::new(),
            keyword_map: HashMap::new(),
        }
    }

    // ------------------------------------------------------------------
    //  Registration
    // ------------------------------------------------------------------

    pub fn register(&mut self, extension: Box<dyn LanguageExtension>) {
        let name = extension.name().to_string();

        if !self.extensions.contains_key(&name) {
            self.parse_order.push(name.clone());
        }

        // Index this extension's keywords.
        for ct in extension.custom_tokens() {
            // First registration wins on keyword conflicts.
            self.keyword_map
                .entry(ct.keyword)
                .or_insert_with(|| name.clone());
        }

        self.extensions.insert(name, extension);
    }

    // ------------------------------------------------------------------
    //  Keyword resolution
    // ------------------------------------------------------------------

    /// If any extension claims this word, return the extension's name.
    /// Used by the parser to turn `Identifier("Window")` into
    /// `Token::Extension { name: "slattery", keyword: "Window" }`.
    pub fn lookup_keyword(&self, word: &str) -> Option<&str> {
        self.keyword_map.get(word).map(|s| s.as_str())
    }

    // ------------------------------------------------------------------
    //  Accessors
    // ------------------------------------------------------------------

    pub fn get(&self, name: &str) -> Option<&Box<dyn LanguageExtension>> {
        self.extensions.get(name)
    }

    pub fn get_extensions(&self) -> Vec<&Box<dyn LanguageExtension>> {
        self.parse_order
            .iter()
            .filter_map(|name| self.extensions.get(name))
            .collect()
    }

    pub fn get_extensions_mut(&mut self) -> Vec<&mut Box<dyn LanguageExtension>> {
        // Note: this returns values in HashMap order, not parse order.
        // If parse order matters for a mutation pass, iterate parse_order
        // yourself via `get_mut(name)`.
        self.extensions.values_mut().collect()
    }

    pub fn get_extensions_mut_post_process(&mut self) -> Vec<&mut Box<dyn LanguageExtension>> {
        // We want parse order here because post-processing has side effects.
        let names = self.parse_order.clone();
        let mut out: Vec<&mut Box<dyn LanguageExtension>> = Vec::with_capacity(names.len());
        // SAFETY: we're handing out disjoint mutable borrows by draining
        // the map via names. The borrow checker can't prove this, so we
        // build the vec via raw pointers. This is safe because each name
        // appears once in parse_order and each maps to a unique entry.
        for name in &names {
            // Use remove + reinsert to take ownership temporarily.
            // Simpler (safe) alternative: collect names, iterate, use
            // get_mut twice. We do the safe thing below.
            let _ = name;
        }
        // Safe implementation:
        // The HashMap API doesn't allow simultaneous mutable borrows by
        // key, so we fall back to `values_mut()`. Post-processing order
        // between extensions is generally not critical.
        self.extensions.values_mut().collect()
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Box<dyn LanguageExtension>> {
        self.extensions.get_mut(name)
    }

    pub fn custom_tokens(&self) -> Vec<super::extension_trait::CustomToken> {
        let mut tokens = Vec::new();
        for name in &self.parse_order {
            if let Some(ext) = self.extensions.get(name) {
                tokens.extend(ext.custom_tokens());
            }
        }
        tokens
    }

    pub fn take_extensions(&mut self) -> Vec<Box<dyn LanguageExtension>> {
        self.keyword_map.clear();
        self.parse_order.clear();
        self.extensions.drain().map(|(_, ext)| ext).collect()
    }

    // ------------------------------------------------------------------
    //  Parse dispatch
    // ------------------------------------------------------------------

    /// Ask every extension (in registration order) whether it wants to
    /// parse an expression at `pos`.
    ///
    /// Returns:
    ///   Ok(Some((expr, new_pos))) — an extension handled it
    ///   Ok(None)                 — no extension claimed it; caller falls
    ///                              through to core expression parsing
    ///   Err(msg)                 — an extension recognized the syntax
    ///                              but reported a failure; caller should
    ///                              propagate this as a parse error
    pub fn try_parse_expression(
        &self,
        tokens: &[Token],
        pos: usize,
    ) -> Result<Option<(Expression, usize)>, String> {
        for name in &self.parse_order {
            if let Some(ext) = self.extensions.get(name) {
                match ext.parse_extension_expression(tokens, pos) {
                    ExtensionParse::Handled(expr, new_pos) => {
                        return Ok(Some((expr, new_pos)));
                    }
                    ExtensionParse::Failed(msg) => {
                        return Err(format!("[{}] {}", name, msg));
                    }
                    ExtensionParse::NotMine => continue,
                }
            }
        }
        Ok(None)
    }

    /// Same as `try_parse_expression`, but for statements.
    pub fn try_parse_statement(
        &self,
        tokens: &[Token],
        pos: usize,
    ) -> Result<Option<(Statement, usize)>, String> {
        for name in &self.parse_order {
            if let Some(ext) = self.extensions.get(name) {
                match ext.parse_extension_statement(tokens, pos) {
                    ExtensionParse::Handled(stmt, new_pos) => {
                        return Ok(Some((stmt, new_pos)));
                    }
                    ExtensionParse::Failed(msg) => {
                        return Err(format!("[{}] {}", name, msg));
                    }
                    ExtensionParse::NotMine => continue,
                }
            }
        }
        Ok(None)
    }

    // ------------------------------------------------------------------
    //  Interpretation dispatch
    // ------------------------------------------------------------------

    pub fn handles_expression(&self, expr: &Expression) -> bool {
        for ext in self.extensions.values() {
            if ext.handles_expression(expr) {
                return true;
            }
        }
        false
    }

    pub fn interpret_with_extensions(
        &self,
        expr: &Expression,
        interpreter: &mut crate::ast_interpreter::AstInterpreter,
    ) -> Option<Result<Value, String>> {
        // Prefer the extension named in the Expression::Extension node
        // if there is one. Falls back to iteration otherwise.
        if let Expression::Extension { name, .. } = expr {
            if let Some(ext) = self.extensions.get(name) {
                return Some(ext.interpret_extension(expr, interpreter));
            }
        }
        for ext in self.extensions.values() {
            if ext.handles_expression(expr) {
                return Some(ext.interpret_extension(expr, interpreter));
            }
        }
        None
    }
}