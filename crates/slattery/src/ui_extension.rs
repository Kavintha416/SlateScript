// slattery/src/ui_extension.rs

use slate_core::extension::{LanguageExtension, CustomToken, ExtensionParse};
use slate_core::ast::{
    Expression, LiteralValue, Span, Statement, UiComponentType,
};
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
        Self { components: HashMap::new() }
    }

    /// Is `word` a UI component type name?
    fn is_component(word: &str) -> bool {
        matches!(word, "Window" | "Column" | "Row" | "Text" | "Button" | "Input")
    }

    fn component_kind(word: &str) -> Option<UiComponentType> {
        Some(match word {
            "Window" => UiComponentType::Window,
            "Column" => UiComponentType::Column,
            "Row"    => UiComponentType::Row,
            "Text"   => UiComponentType::Text,
            "Button" => UiComponentType::Button,
            "Input"  => UiComponentType::Input,
            _ => return None,
        })
    }

    // ------------------------------------------------------------------
    //  Component parsing
    // ------------------------------------------------------------------

    /// Parse a component body starting at `body_start` (the token *after*
    /// the component-type identifier and any closing `>`).
    ///
    /// Returns the `Expression::UiComponent` and the position just past
    /// the closing `}`.
    fn parse_component_body(
        &self,
        tokens: &[Token],
        body_start: usize,
        component_type: UiComponentType,
    ) -> Result<(Expression, usize), String> {
        let mut i = body_start;

        let mut properties: HashMap<String, Expression> = HashMap::new();
        let mut children: Vec<Expression> = Vec::new();
        let mut events: HashMap<String, String> = HashMap::new();

        if !matches!(tokens.get(i), Some(Token::LeftBrace)) {
            // No body — treat as a bare component reference.
            let span = Span::new(i, i, 0, 0);
            return Ok((
                Expression::UiComponent {
                    component_type,
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
            // Skip stray commas
            if matches!(tokens.get(i), Some(Token::Comma)) {
                i += 1;
                continue;
            }

            let key: String = match tokens.get(i) {
                Some(Token::Identifier(name)) => { let s = name.clone(); i += 1; s }
                other => {
                    return Err(format!(
                        "Expected property name in {} body, got {:?}",
                        component_type.as_str(), other
                    ));
                }
            };

            // --- Colon ---
            if !matches!(tokens.get(i), Some(Token::Colon)) {
                return Err(format!("Expected ':' after property '{}'", key));
            }
            i += 1;

            // --- Special keys ---
            if key == "Child" || key == "Parent" {
                // `<Component> { ... }` form
                if matches!(tokens.get(i), Some(Token::LessThan)) {
                    i += 1;
                    let child_type_name = match tokens.get(i) {
                        Some(Token::Identifier(w)) if Self::is_component(w) => w.clone(),
                        other => return Err(format!(
                            "Expected component type after '<', got {:?}", other
                        )),
                    };
                    let child_kind = Self::component_kind(&child_type_name).unwrap();
                    i += 1;
                    if !matches!(tokens.get(i), Some(Token::GreaterThan)) {
                        return Err("Expected '>' after component type".to_string());
                    }
                    i += 1;
                    let (child_expr, new_i) = self.parse_component_body(tokens, i, child_kind)?;
                    children.push(child_expr);
                    i = new_i;
                } else if let Some(Token::Identifier(name)) = tokens.get(i) {
                    // `Child: SomeVar` — reference by identity
                    let name = name.clone();
                    children.push(Expression::Variable(name, Span::new(i, i + 1, 0, 0)));
                    i += 1;
                } else {
                    return Err(format!(
                        "Expected '<Component>' or identifier after '{}:'", key
                    ));
                }
            } else if key.starts_with("on_") {
                // Event handler: expect an identifier
                match tokens.get(i) {
                    Some(Token::Identifier(handler)) => {
                        events.insert(key.clone(), handler.clone());
                        i += 1;
                    }
                    other => return Err(format!(
                        "Expected handler identifier after '{}:', got {:?}", key, other
                    )),
                }
            } else if key == "Identity" {
                // `Identity: name` or `Identity: "name"`
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
                    other => return Err(format!(
                        "Expected identifier or string after 'Identity:', got {:?}", other
                    )),
                }
            } else {
                // Regular property: literal or identifier value
                let (value_expr, new_i) = self.parse_property_value(tokens, i)?;
                properties.insert(key, value_expr);
                i = new_i;
            }

            // Optional trailing comma
            if matches!(tokens.get(i), Some(Token::Comma)) {
                i += 1;
            }
        }

        if !matches!(tokens.get(i), Some(Token::RightBrace)) {
            return Err(format!(
                "Expected '}}' to close {} body", component_type.as_str()
            ));
        }
        i += 1; // consume '}'

        let span = Span::new(body_start, i, 0, 0);
        Ok((
            Expression::UiComponent {
                component_type,
                identity: None,
                properties,
                children,
                events,
                span,
            },
            i,
        ))
    }

    /// Parse a single property value: string, number, bool, or identifier.
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
            Some(Token::Identifier(name)) => Ok((
                Expression::Variable(name.clone(), span),
                pos + 1,
            )),
            other => Err(format!("Expected property value, got {:?}", other)),
        }
    }
}

impl LanguageExtension for SlatteryExtension {
    fn name(&self) -> &str { "slattery" }

    fn clone_box(&self) -> Box<dyn LanguageExtension> {
        Box::new(self.clone())
    }

    fn custom_tokens(&self) -> Vec<CustomToken> {
        const KEYWORDS: &[&str] = &[
            "Window", "Column", "Row", "Text", "Button", "Input",
            "Parent", "Child",
            "on_tap", "on_click", "on_change", "on_input",
            "render", "Identity", "Rewrite",
        ];
        KEYWORDS.iter().map(|k| CustomToken {
            keyword: k.to_string(),
            token: Token::Extension {
                name: "slattery".to_string(),
                keyword: k.to_string(),
            },
        }).collect()
    }

    fn parse_extension_statement(
        &self,
        tokens: &[Token],
        pos: usize,
    ) -> ExtensionParse<Statement> {
        // Match: make <name> = Window { ... }
        //        make <name> = <Window> { ... }
        if !matches!(tokens.get(pos), Some(Token::Make)) {
            return ExtensionParse::NotMine;
        }

        let name = match tokens.get(pos + 1) {
            Some(Token::Identifier(n)) => n.clone(),
            _ => return ExtensionParse::NotMine,
        };
        if !matches!(tokens.get(pos + 2), Some(Token::Equal)) {
            return ExtensionParse::NotMine;
        }

        // Two forms for the RHS:
        //   Bare:   Window { ... }
        //   Angle:  <Window> { ... }
        let (kind, body_start) = match tokens.get(pos + 3) {
            Some(Token::Identifier(w)) if Self::is_component(w) => {
                (Self::component_kind(w).unwrap(), pos + 4)
            }
            Some(Token::LessThan) => {
                let w = match tokens.get(pos + 4) {
                    Some(Token::Identifier(w)) if Self::is_component(w) => w.clone(),
                    _ => return ExtensionParse::NotMine,
                };
                if !matches!(tokens.get(pos + 5), Some(Token::GreaterThan)) {
                    return ExtensionParse::NotMine;
                }
                (Self::component_kind(&w).unwrap(), pos + 6)
            }
            _ => return ExtensionParse::NotMine,
        };

        match self.parse_component_body(tokens, body_start, kind) {
            Ok((expr, new_pos)) => {
                let span = Span::new(pos, new_pos, 0, 0);
                ExtensionParse::Handled(
                    Statement::Assignment { name, value: expr, span },
                    new_pos,
                )
            }
            Err(msg) => ExtensionParse::Failed(msg),
        }
    }

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
            let kind = Self::component_kind(&w).unwrap();
            return match self.parse_component_body(tokens, pos + 3, kind) {
                Ok((expr, new_pos)) => ExtensionParse::Handled(expr, new_pos),
                Err(msg) => ExtensionParse::Failed(msg),
            };
        }

        // Bare form: Window { ... }
        if let Some(Token::Identifier(w)) = tokens.get(pos) {
            if Self::is_component(w) {
                let kind = Self::component_kind(w).unwrap();
                return match self.parse_component_body(tokens, pos + 1, kind) {
                    Ok((expr, new_pos)) => ExtensionParse::Handled(expr, new_pos),
                    Err(msg) => ExtensionParse::Failed(msg),
                };
            }
        }

        ExtensionParse::NotMine
    }

    fn interpret_extension(
        &self,
        _expr: &Expression,
        _interpreter: &mut AstInterpreter,
    ) -> Result<Value, String> {
        Ok(Value::Null)
    }

    fn handles_expression(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::UiComponent { .. } | Expression::UiRender { .. })
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}