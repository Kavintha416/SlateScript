//! Style Interpreter
//! 
//! Interprets CSS-like style tokens and creates style rules for Slattery UI components.

use std::collections::HashMap;
use super::style_lexer::StyleToken;

#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue {
    Color(String),
    Number(f64),
    String(String),
    Unit(f64, String), // value with unit (e.g., 16px)
    Boolean(bool),
    None,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum StyleState {
    Normal,
    Hover,
    Active,
    Focus,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selector: String,
    pub properties: HashMap<String, StyleValue>,
    pub state: StyleState,
    pub important: bool,
}

impl StyleRule {
    pub fn new(selector: String) -> Self {
        Self {
            selector,
            properties: HashMap::new(),
            state: StyleState::Normal,
            important: false,
        }
    }
}

pub struct StyleInterpreter {
    rules: Vec<StyleRule>,
    current_rule: Option<StyleRule>,
}

impl StyleInterpreter {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            current_rule: None,
        }
    }
    
    pub fn interpret(&mut self, tokens: Vec<StyleToken>) -> Result<Vec<StyleRule>, String> {
        let mut i = 0;
        
        while i < tokens.len() {
            match &tokens[i] {
                StyleToken::EOF => break,
                
                StyleToken::Style | StyleToken::Define => {
                    i += 1;
                    if let Some(rule) = self.parse_style_rule(&tokens, &mut i)? {
                        self.rules.push(rule);
                    }
                }
                
                StyleToken::Import => {
                    i = self.parse_import(&tokens, &mut i)?;
                }
                
                _ => {
                    // Try to parse as a standalone selector rule
                    if let Some(rule) = self.parse_selector_rule(&tokens, &mut i)? {
                        self.rules.push(rule);
                    } else {
                        i += 1; // Skip unknown token
                    }
                }
            }
        }
        
        Ok(self.rules.clone())
    }
    
    fn parse_style_rule(&mut self, tokens: &[StyleToken], i: &mut usize) -> Result<Option<StyleRule>, String> {
        // Parse selector
        let selector = self.parse_selector(tokens, i)?;
        
        // Expect {
        if *i >= tokens.len() || !matches!(tokens[*i], StyleToken::LeftBrace) {
            return Err("Expected '{' after selector".to_string());
        }
        *i += 1;
        
        let mut rule = StyleRule::new(selector);
        
        // Parse properties
        while *i < tokens.len() && !matches!(tokens[*i], StyleToken::RightBrace) {
            if let Some((property, value)) = self.parse_property(tokens, i)? {
                rule.properties.insert(property, value);
            }
            
            // Expect ; or }
            if *i < tokens.len() && matches!(tokens[*i], StyleToken::Semicolon) {
                *i += 1;
            }
        }
        
        // Expect }
        if *i >= tokens.len() || !matches!(tokens[*i], StyleToken::RightBrace) {
            return Err("Expected '}' after properties".to_string());
        }
        *i += 1;
        
        Ok(Some(rule))
    }
    
    fn parse_selector_rule(&mut self, tokens: &[StyleToken], i: &mut usize) -> Result<Option<StyleRule>, String> {
        let selector = self.parse_selector(tokens, i)?;
        
        // Expect {
        if *i >= tokens.len() || !matches!(tokens[*i], StyleToken::LeftBrace) {
            return Ok(None);
        }
        *i += 1;
        
        let mut rule = StyleRule::new(selector);
        
        // Parse properties
        while *i < tokens.len() && !matches!(tokens[*i], StyleToken::RightBrace) {
            if let Some((property, value)) = self.parse_property(tokens, i)? {
                rule.properties.insert(property, value);
            }
            
            // Expect ; or }
            if *i < tokens.len() && matches!(tokens[*i], StyleToken::Semicolon) {
                *i += 1;
            }
        }
        
        // Expect }
        if *i >= tokens.len() || !matches!(tokens[*i], StyleToken::RightBrace) {
            return Err("Expected '}' after properties".to_string());
        }
        *i += 1;
        
        Ok(Some(rule))
    }
    
    fn parse_selector(&self, tokens: &[StyleToken], i: &mut usize) -> Result<String, String> {
        if *i >= tokens.len() {
            return Err("Expected selector".to_string());
        }
        
        let mut selector = String::new();
        
        match &tokens[*i] {
            StyleToken::Selector(s) => {
                selector.push_str(s);
                *i += 1;
            }
            StyleToken::ClassSelector(s) => {
                selector.push('.');
                selector.push_str(s);
                *i += 1;
            }
            StyleToken::IdSelector(s) => {
                selector.push('#');
                selector.push_str(s);
                *i += 1;
            }
            StyleToken::ComponentSelector(s) => {
                selector.push('@');
                selector.push_str(s);
                *i += 1;
            }
            _ => return Err("Invalid selector".to_string()),
        }
        
        // Check for pseudo-classes/states
        if *i < tokens.len() && matches!(tokens[*i], StyleToken::Colon) {
            *i += 1;
            if *i < tokens.len() {
                match &tokens[*i] {
                    StyleToken::Hover => {
                        selector.push_str(":hover");
                        *i += 1;
                    }
                    StyleToken::Active => {
                        selector.push_str(":active");
                        *i += 1;
                    }
                    StyleToken::Focus => {
                        selector.push_str(":focus");
                        *i += 1;
                    }
                    StyleToken::Disabled => {
                        selector.push_str(":disabled");
                        *i += 1;
                    }
                    StyleToken::PseudoClass(s) => {
                        selector.push(':');
                        selector.push_str(s);
                        *i += 1;
                    }
                    _ => {}
                }
            }
        }
        
        Ok(selector)
    }
    
    fn parse_property(&self, tokens: &[StyleToken], i: &mut usize) -> Result<Option<(String, StyleValue)>, String> {
        if *i >= tokens.len() {
            return Ok(None);
        }
        
        // Parse property name
        let property = match &tokens[*i] {
            StyleToken::Property(s) => s.clone(),
            StyleToken::Selector(s) => s.clone(),
            _ => return Ok(None),
        };
        *i += 1;
        
        // Expect :
        if *i >= tokens.len() || !matches!(tokens[*i], StyleToken::Colon) {
            return Err("Expected ':' after property name".to_string());
        }
        *i += 1;
        
        // Parse value
        if *i >= tokens.len() {
            return Err("Expected value after ':'".to_string());
        }
        
        let value = match &tokens[*i] {
            StyleToken::Value(s) => {
                let v = StyleValue::String(s.clone());
                *i += 1;
                v
            }
            StyleToken::String(s) => {
                let v = StyleValue::String(s.clone());
                *i += 1;
                v
            }
            StyleToken::Color(s) => {
                let v = StyleValue::Color(s.clone());
                *i += 1;
                v
            }
            StyleToken::Number(n) => {
                let n = *n;
                *i += 1;
                // Check for unit (e.g. "px", "em", "rem")
                if *i < tokens.len() && matches!(&tokens[*i], StyleToken::Selector(_)) {
                    let unit = if let StyleToken::Selector(s) = &tokens[*i] { s.clone() } else { String::new() };
                    *i += 1;
                    StyleValue::Unit(n, unit)
                } else {
                    StyleValue::Number(n)
                }
            }
            StyleToken::Selector(s) => {
                let v = StyleValue::String(s.clone());
                *i += 1;
                v
            }
            _ => return Err(format!("Invalid value token: {:?}", tokens[*i])),
        };
        
        // Check for !important
        let mut _important = false;
        if *i < tokens.len() && matches!(tokens[*i], StyleToken::Important) {
            _important = true;
            *i += 1;
        }
        
        Ok(Some((property, value)))
    }
    
    fn parse_import(&self, tokens: &[StyleToken], i: &mut usize) -> Result<usize, String> {
        // Skip import for now
        while *i < tokens.len() && !matches!(tokens[*i], StyleToken::Semicolon) {
            *i += 1;
        }
        if *i < tokens.len() {
            *i += 1; // Skip ;
        }
        Ok(*i)
    }
}
