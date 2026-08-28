// slattery/src/lib.rs

// 1. Expose your Slattery modules to the outside world
pub mod install;
pub mod build;
pub mod sla_lexer;
pub mod sla_interpreter;
pub mod egui_renderer;
pub mod ui_integration;
pub mod styles;
pub mod ui_extension;
pub mod logger;
pub mod button_handler;
pub use button_handler::ButtonHandler;
// 2. Use slate-core types directly
use slate_core::extension::LanguageExtension;
use slate_core::ast::Expression;
use slate_core::ast_interpreter::AstInterpreter;
use slate_core::value::Value;
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
}

impl LanguageExtension for SlatteryExtension {
    fn name(&self) -> &str {
        "slattery"
    }

    fn clone_box(&self) -> Box<dyn LanguageExtension> {
        Box::new(self.clone())
    }

    fn parse_extension(&self, _tokens: &[Token], _pos: usize) -> Option<(Expression, usize)> {
        None
    }

    fn interpret_extension(
        &self,
        _expr: &Expression,
        _interpreter: &mut AstInterpreter,
    ) -> Result<Value, String> {
        Err("Not implemented".to_string())
    }

    fn handles_expression(&self, _expr: &Expression) -> bool {
        false
    }

    fn custom_tokens(&self) -> Vec<slate_core::extension::CustomToken> {
        vec![
            slate_core::extension::CustomToken { keyword: "Window".to_string(), token: Token::Window },
            slate_core::extension::CustomToken { keyword: "Column".to_string(), token: Token::Column },
            slate_core::extension::CustomToken { keyword: "Row".to_string(), token: Token::Row },
            slate_core::extension::CustomToken { keyword: "Text".to_string(), token: Token::Text },
            slate_core::extension::CustomToken { keyword: "Button".to_string(), token: Token::Button },
            slate_core::extension::CustomToken { keyword: "Input".to_string(), token: Token::Input },
            slate_core::extension::CustomToken { keyword: "Parent".to_string(), token: Token::Parent },
            slate_core::extension::CustomToken { keyword: "Child".to_string(), token: Token::Child },
            slate_core::extension::CustomToken { keyword: "on_tap".to_string(), token: Token::OnTap },
            slate_core::extension::CustomToken { keyword: "on_click".to_string(), token: Token::OnClick },
            slate_core::extension::CustomToken { keyword: "on_change".to_string(), token: Token::OnChange },
            slate_core::extension::CustomToken { keyword: "on_input".to_string(), token: Token::OnInput },
            slate_core::extension::CustomToken { keyword: "render".to_string(), token: Token::Render },
            slate_core::extension::CustomToken { keyword: "Identity".to_string(), token: Token::Identity },
            slate_core::extension::CustomToken { keyword: "Rewrite".to_string(), token: Token::Rewrite },
        ]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}