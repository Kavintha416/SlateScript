// src/slattery/ui_extension.rs

use crate::extension::*;
use crate::lexer::Token;
use crate::ast::{Expression, Program, Span};
use crate::value::Value;
use crate::ast_interpreter::AstInterpreter;
use std::collections::HashMap;

pub struct SlatteryExtension {
    // UI-specific state
    components: HashMap<String, ()>,
}

impl SlatteryExtension {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }
}

impl LanguageExtension for SlatteryExtension {
    fn name(&self) -> &str {
        "slattery"
    }
    
    fn custom_tokens(&self) -> Vec<CustomToken> {
        vec![
            CustomToken { keyword: "Window".to_string(), token: Token::Window },
            CustomToken { keyword: "Column".to_string(), token: Token::Column },
            CustomToken { keyword: "Row".to_string(), token: Token::Row },
            CustomToken { keyword: "Text".to_string(), token: Token::Text },
            CustomToken { keyword: "Button".to_string(), token: Token::Button },
            CustomToken { keyword: "Input".to_string(), token: Token::Input },
            CustomToken { keyword: "Parent".to_string(), token: Token::Parent },
            CustomToken { keyword: "Child".to_string(), token: Token::Child },
            CustomToken { keyword: "on_tap".to_string(), token: Token::OnTap },
            CustomToken { keyword: "on_click".to_string(), token: Token::OnClick },
            CustomToken { keyword: "on_change".to_string(), token: Token::OnChange },
            CustomToken { keyword: "on_input".to_string(), token: Token::OnInput },
            CustomToken { keyword: "render".to_string(), token: Token::Render },
            CustomToken { keyword: "Identity".to_string(), token: Token::Identity },
            CustomToken { keyword: "Rewrite".to_string(), token: Token::Rewrite },
        ]
    }
    
    fn parse_extension(&self, tokens: &[Token], pos: usize) -> Option<(Expression, usize)> {
        // TODO: Implement full UI parsing
        // For now, return None (let core parser handle it)
        None
    }
    
    fn interpret_extension(
        &self,
        expr: &Expression,
        _interpreter: &mut AstInterpreter,
    ) -> Result<Value, String> {
        // Handle UI expressions
        match expr {
            Expression::UiComponent { component_type, .. } => {
                Ok(Value::String(format!("UI Component: {:?}", component_type)))
            }
            Expression::UiRender { .. } => {
                Ok(Value::Null)
            }
            _ => Err(format!("Unknown UI expression: {:?}", expr)),
        }
    }
    
    fn handles_expression(&self, expr: &Expression) -> bool {
        matches!(expr, 
            Expression::UiComponent { .. } | 
            Expression::UiRender { .. }
        )
    }
    
    fn post_process_ast(&self, _program: &mut Program) -> Result<(), String> {
        Ok(())
    }
}