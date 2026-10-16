// slattery/src/ui_extension.rs
//
// The Slattery extension for SlateScript.
//
// Claims:
//   - UI component keywords (Window, Button, ...)
//   - Style blocks:  style <Target> { prop: value ... }
//
// Parses style blocks into `Expression::Extension` nodes whose
// `type_name` is `Style:<encoded>` — a compact representation the
// renderer decodes at setup time.

use slate_core::extension::{CustomToken, ExtensionParse, LanguageExtension};
use slate_core::ast::{Expression, LiteralValue, Span, Statement};
use slate_core::value::Value;
use slate_core::ast_interpreter::AstInterpreter;
use slate_core::lexer::Token;
use std::collections::HashMap;

#[derive(Clone)]
pub struct SlatteryExtension {
    components: HashMap<String, ()>,
}

impl SlatteryExtension {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    // ------------------------------------------------------------------
    //  Helpers
    // ------------------------------------------------------------------

    fn is_component(word: &str) -> bool {
        matches!(
            word,
            "Window" | "Column" | "Row" | "Text" | "Button" | "Input"
        )
    }

    /// Parse a component body starting at `body_start` (the token *after*
    /// the component-type identifier and any closing `>`).
    fn parse_component_body(
        &self,
        tokens: &[Token],
        body_start: usize,
        component_type_name: &str,
    ) -> Result<(Expression, usize), String> {
        let mut i = body_start;

        let mut properties: HashMap<String, Expression> = HashMap::new();
        let mut children: Vec<Expression> = Vec::new();
        let mut events: HashMap<String, String> = HashMap::new();

        if !matches!(tokens.get(i), Some(Token::LeftBrace)) {
            // No body — treat as a bare component reference.
            let span = Span::new(i, i, 0, 0);
            let kind = component_kind(component_type_name)
                .ok_or_else(|| format!("Unknown component: {}", component_type_name))?;
            return Ok((
                Expression::UiComponent {
                    component_type: kind,
                    identity: None,
                    properties,
                    children,
                    events,
                    span,
                },
                i,
            ));
        }
        i += 1; // consume '{'

        while i < tokens.len() && !matches!(tokens.get(i), Some(Token::RightBrace)) {
            if matches!(tokens.get(i), Some(Token::Comma)) {
                i += 1;
                continue;
            }

            // ---- Property key ----
            let key: String = match tokens.get(i) {
                Some(Token::Identifier(name)) => {
                    let s = name.clone();
                    i += 1;
                    s
                }
                other => {
                    return Err(format!(
                        "Expected property name in {} body, got {:?}",
                        component_type_name, other
                    ));
                }
            };

            // ---- Colon ----
            if !matches!(tokens.get(i), Some(Token::Colon)) {
                return Err(format!("Expected ':' after property '{}'", key));
            }
            i += 1;

            // ---- Special keys ----
            if key == "Child" || key == "Parent" {
                if matches!(tokens.get(i), Some(Token::LessThan)) {
                    i += 1;  // skip '<'

                    let child_type_name = match tokens.get(i) {
                        Some(Token::Identifier(w)) if Self::is_component(w) => w.clone(),
                        other => {
                            return Err(format!(
                                "Expected component type after '<', got {:?}",
                                other
                            ));
                        }
                    };
                    i += 1;  // skip identifier

                    if !matches!(tokens.get(i), Some(Token::GreaterThan)) {
                        return Err("Expected '>' after component type".to_string());
                    }
                    i += 1;  // skip '>'

                    // `i` now points at the `{` of the child's body (or at whatever
                    // follows the type identifier if the child has no body).
                    let (child_expr, new_i) = self.parse_component_body(
                        tokens,
                        i,
                        &child_type_name,
                    )?;
                    children.push(child_expr);
                    i = new_i;
                } else if let Some(Token::Identifier(name)) = tokens.get(i) {
                    let name = name.clone();
                    children.push(Expression::Variable(name, Span::new(i, i + 1, 0, 0)));
                    i += 1;
                } else {
                    return Err(format!(
                        "Expected '<Component>' or identifier after '{}:'",
                        key
                    ));
                }
            } else if key.starts_with("on_") {
                match tokens.get(i) {
                    Some(Token::Identifier(handler)) => {
                        events.insert(key.clone(), handler.clone());
                        i += 1;
                    }
                    other => {
                        return Err(format!(
                            "Expected handler identifier after '{}:', got {:?}",
                            key, other
                        ));
                    }
                }
            } else if key == "Identity" {
                match tokens.get(i) {
                    Some(Token::Identifier(id)) => {
                        properties.insert(
                            "Identity".to_string(),
                            Expression::Literal(
                                LiteralValue::String(id.clone()),
                                Span::new(i, i + 1, 0, 0),
                            ),
                        );
                        i += 1;
                    }
                    Some(Token::String(s)) => {
                        properties.insert(
                            "Identity".to_string(),
                            Expression::Literal(
                                LiteralValue::String(s.clone()),
                                Span::new(i, i + 1, 0, 0),
                            ),
                        );
                        i += 1;
                    }
                    other => {
                        return Err(format!(
                            "Expected identifier or string after 'Identity:', got {:?}",
                            other
                        ));
                    }
                }
            } else {
                let (value_expr, new_i) = self.parse_property_value(tokens, i)?;
                properties.insert(key, value_expr);
                i = new_i;
            }

            if matches!(tokens.get(i), Some(Token::Comma)) {
                i += 1;
            }
        }

        if !matches!(tokens.get(i), Some(Token::RightBrace)) {
            return Err(format!(
                "Expected '}}' to close {} body",
                component_type_name
            ));
        }
        i += 1;

        let span = Span::new(body_start, i, 0, 0);
        let kind = component_kind(component_type_name)
            .ok_or_else(|| format!("Unknown component: {}", component_type_name))?;

        Ok((
            Expression::UiComponent {
                component_type: kind,
                identity: None,
                properties,
                children,
                events,
                span,
            },
            i,
        ))
    }

    fn parse_property_value(
        &self,
        tokens: &[Token],
        pos: usize,
    ) -> Result<(Expression, usize), String> {
        let span = Span::new(pos, pos + 1, 0, 0);
        match tokens.get(pos) {
            Some(Token::String(s)) => Ok((
                Expression::Literal(LiteralValue::String(s.clone()), span),
                pos + 1,
            )),
            Some(Token::Number(n)) => Ok((
                Expression::Literal(LiteralValue::Number(*n as f64), span),
                pos + 1,
            )),
            Some(Token::Float(f)) => Ok((
                Expression::Literal(LiteralValue::Number(*f), span),
                pos + 1,
            )),
            Some(Token::True) => Ok((
                Expression::Literal(LiteralValue::Boolean(true), span),
                pos + 1,
            )),
            Some(Token::False) => Ok((
                Expression::Literal(LiteralValue::Boolean(false), span),
                pos + 1,
            )),
            Some(Token::Identifier(name)) => {
                Ok((Expression::Variable(name.clone(), span), pos + 1))
            }
            other => Err(format!("Expected property value, got {:?}", other)),
        }
    }

    /// Parse `style <Target> { prop: value ... }` into a
    /// `Statement::Expression(Expression::Extension)` whose `type_name`
    /// encodes the parsed rule.
    ///
    /// Encoding format:  `Style:<target>:prop=val;prop=val;`
    ///
    /// The renderer decodes this at setup time. We use `=` between key
    /// and value inside the encoding (rather than `:`) so that values
    /// containing `:` (none currently, but future-proof) don't break
    /// the split.
    fn parse_style_block(
        &self,
        tokens: &[Token],
        pos: usize,
    ) -> ExtensionParse<Statement> {
        let mut i = pos + 1; // skip `style`

        // Target name
        let target = match tokens.get(i) {
            Some(Token::Identifier(name)) => {
                let s = name.clone();
                i += 1;
                s
            }
            other => {
                return ExtensionParse::Failed(format!(
                    "Expected style target after 'style', got {:?}",
                    other
                ));
            }
        };

        if !matches!(tokens.get(i), Some(Token::LeftBrace)) {
            return ExtensionParse::Failed(format!(
                "Expected '{{' after 'style {}'",
                target
            ));
        }
        i += 1; // consume '{'

        let mut encoded = format!("Style:{}:", target);

        while i < tokens.len() && !matches!(tokens.get(i), Some(Token::RightBrace)) {
            // ---- Property name ----
            //
            // Can contain hyphens: `background-color`, `font-size`,
            // `border-radius`, etc. The lexer emits each hyphenated
            // part as a separate `Identifier` with a `Minus` between.
            // We reconstruct the full name here.
            let mut prop = match tokens.get(i) {
                Some(Token::Identifier(p)) => {
                    let s = p.clone();
                    i += 1;
                    s
                }
                other => {
                    return ExtensionParse::Failed(format!(
                        "Expected property name in 'style {}', got {:?}",
                        target, other
                    ));
                }
            };

            // Consume any `- identifier` continuation
            while matches!(tokens.get(i), Some(Token::Minus))
                && matches!(tokens.get(i + 1), Some(Token::Identifier(_)))
            {
                prop.push('-');
                if let Some(Token::Identifier(part)) = tokens.get(i + 1) {
                    prop.push_str(part);
                }
                i += 2;
            }

            // ---- Colon ----
            if !matches!(tokens.get(i), Some(Token::Colon)) {
                return ExtensionParse::Failed(format!(
                    "Expected ':' after '{}' in 'style {}'",
                    prop, target
                ));
            }
            i += 1;

            // ---- Value ----
            //
            // Track whether we read a number so we can decide whether
            // a following identifier is a unit suffix (`px`, `em`, `%`).
            let mut was_number = false;
            let mut val = match tokens.get(i) {
                Some(Token::HexColor(h)) => {
                    let s = h.clone();
                    i += 1;
                    s
                }
                Some(Token::String(s)) => {
                    let v = s.clone();
                    i += 1;
                    v
                }
                Some(Token::Number(n)) => {
                    was_number = true;
                    let v = n.to_string();
                    i += 1;
                    v
                }
                Some(Token::Float(f)) => {
                    was_number = true;
                    let v = f.to_string();
                    i += 1;
                    v
                }
                Some(Token::Identifier(w)) => {
                    let v = w.clone();
                    i += 1;
                    v
                }
                other => {
                    return ExtensionParse::Failed(format!(
                        "Expected value after '{}:', got {:?}",
                        prop, other
                    ));
                }
            };

            // If the value was a number, a following identifier is a
            // unit like `px`, `em`, `%`. Concatenate.
            //
            // IMPORTANT: this must only run when the value we just read
            // was a number. Otherwise it will eat the next property
            // name, e.g. `color: white` followed by `border-radius:`
            // would absorb `border` as a unit suffix for `white`.
            if was_number {
                if let Some(Token::Identifier(unit)) = tokens.get(i) {
                    val.push_str(unit);
                    i += 1;
                }
            }

            encoded.push_str(&format!("{}={};", prop, val));
        }

        if !matches!(tokens.get(i), Some(Token::RightBrace)) {
            return ExtensionParse::Failed(format!(
                "Expected '}}' to close 'style {}'",
                target
            ));
        }
        i += 1;

        let span = Span::new(pos, i, 0, 0);
        let expr = Expression::Extension {
            name: "slattery".to_string(),
            type_name: encoded,
            span,
        };

        ExtensionParse::Handled(Statement::Expression(expr), i)
    }
}

/// Map a component type name to its `UiComponentType`.
fn component_kind(name: &str) -> Option<slate_core::ast::UiComponentType> {
    use slate_core::ast::UiComponentType;
    Some(match name {
        "Window" => UiComponentType::Window,
        "Column" => UiComponentType::Column,
        "Row"    => UiComponentType::Row,
        "Text"   => UiComponentType::Text,
        "Button" => UiComponentType::Button,
        "Input"  => UiComponentType::Input,
        _ => return None,
    })
}

impl Default for SlatteryExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageExtension for SlatteryExtension {
    fn name(&self) -> &str {
        "slattery"
    }

    fn clone_box(&self) -> Box<dyn LanguageExtension> {
        Box::new(self.clone())
    }

    fn custom_tokens(&self) -> Vec<CustomToken> {
        const KEYWORDS: &[&str] = &[
            // Components
            "Window", "Column", "Row", "Text", "Button", "Input",
            // Relationships
            "Parent", "Child",
            // Events
            "on_tap", "on_click", "on_change", "on_input",
            // Rendering + identity
            "render", "Identity", "Rewrite",
            // Style blocks
            "style",
        ];
        KEYWORDS
            .iter()
            .map(|k| CustomToken {
                keyword: k.to_string(),
                token: Token::Extension {
                    name: "slattery".to_string(),
                    keyword: k.to_string(),
                },
            })
            .collect()
    }

    // ------------------------------------------------------------------
    //  Statements
    // ------------------------------------------------------------------

    fn parse_extension_statement(
        &self,
        tokens: &[Token],
        pos: usize,
    ) -> ExtensionParse<Statement> {
        // Only `style` is handled at the statement level for now.
        // (Component definitions go through `parse_extension_expression`.)
        if matches!(tokens.get(pos), Some(Token::Identifier(w)) if w == "style") {
            return self.parse_style_block(tokens, pos);
        }
        ExtensionParse::NotMine
    }

    // ------------------------------------------------------------------
    //  Expressions
    // ------------------------------------------------------------------

    fn parse_extension_expression(
        &self,
        tokens: &[Token],
        pos: usize,
    ) -> ExtensionParse<Expression> {
        // Angle form: <Window> { ... }
        if matches!(tokens.get(pos), Some(Token::LessThan)) {
            let w = match tokens.get(pos + 1) {
                Some(Token::Identifier(w)) if Self::is_component(w) => w.clone(),
                _ => return ExtensionParse::NotMine,
            };
            if !matches!(tokens.get(pos + 2), Some(Token::GreaterThan)) {
                return ExtensionParse::NotMine;
            }
            return match self.parse_component_body(tokens, pos + 3, &w) {
                Ok((expr, new_pos)) => ExtensionParse::Handled(expr, new_pos),
                Err(msg) => ExtensionParse::Failed(msg),
            };
        }

        // Bare form: Window { ... }
        if let Some(Token::Identifier(w)) = tokens.get(pos) {
            if Self::is_component(w) {
                return match self.parse_component_body(tokens, pos + 1, w) {
                    Ok((expr, new_pos)) => ExtensionParse::Handled(expr, new_pos),
                    Err(msg) => ExtensionParse::Failed(msg),
                };
            }
        }

        ExtensionParse::NotMine
    }

    // ------------------------------------------------------------------
    //  Runtime
    // ------------------------------------------------------------------

    fn interpret_extension(
        &self,
        _expr: &Expression,
        _interpreter: &mut AstInterpreter,
    ) -> Result<Value, String> {
        // Style blocks and UI components aren't runtime values. The
        // renderer reads them from the AST directly.
        Ok(Value::Null)
    }

    fn handles_expression(&self, expr: &Expression) -> bool {
        match expr {
            Expression::UiComponent { .. } => true,
            Expression::UiRender { .. } => true,
            Expression::Extension { name, .. } => name == "slattery",
            _ => false,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}