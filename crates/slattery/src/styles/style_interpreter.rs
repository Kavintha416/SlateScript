// slattery/src/styles/style_interpreter.rs

//! Style Interpreter
//! 
//! Interprets style blocks from .st files and creates style rules.

use std::collections::HashMap;

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
    
    // Add a method to set properties from SlateScript-style syntax
    pub fn set_property(&mut self, name: &str, value: StyleValue) {
        self.properties.insert(name.to_string(), value);
    }
}

pub struct StyleInterpreter {
    rules: Vec<StyleRule>,
}

impl StyleInterpreter {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }
    
    // Parse style blocks from .st files
    pub fn parse_style_blocks(&mut self, source: &str) -> Result<Vec<StyleRule>, String> {
        let lines: Vec<&str> = source.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i].trim();
            
            // Look for "style" keyword
            if line.starts_with("style ") {
                let selector = line.trim_start_matches("style ").trim().to_string();
                
                // Handle "style Window {" (brace on same line)
                let selector = selector.trim_end_matches('{').trim().to_string();
                
                i += 1;
                
                // Check if brace is on the same line or next line
                let found_open_brace = line.contains('{');
                if !found_open_brace {
                    // Expect '{' on next line
                    if i >= lines.len() || !lines[i].trim().starts_with('{') {
                        return Err(format!("Expected '{{' after style selector '{}'", selector));
                    }
                    i += 1;
                }
                
                // Parse properties until '}'
                let mut rule = StyleRule::new(selector.clone());
                
                while i < lines.len() && lines[i].trim() != "}" {
                    let prop_line = lines[i].trim();
                    if !prop_line.is_empty() {
                        // Parse: property: value
                        if let Some(colon_pos) = prop_line.find(':') {
                            let prop_name = prop_line[..colon_pos].trim().to_string();
                            let prop_value = prop_line[colon_pos + 1..].trim().to_string();
                            
                            // Convert property value to StyleValue
                            let value = self.parse_style_value(&prop_value);
                            rule.set_property(&prop_name, value);
                        }
                    }
                    i += 1;
                }
                
                // Expect '}'
                if i >= lines.len() || lines[i].trim() != "}" {
                    return Err(format!("Expected '}}' after style block '{}'", selector));
                }
                i += 1;
                
                self.rules.push(rule);
            } else {
                i += 1;
            }
        }
        
        Ok(self.rules.clone())
    }
    
    // Helper to parse style values
    fn parse_style_value(&self, value: &str) -> StyleValue {
        let value = value.trim();
        
        // Check for hex color
        if value.starts_with('#') && value.len() == 7 {
            return StyleValue::Color(value.to_string());
        }
        
        // Check for numbers
        if let Ok(num) = value.parse::<f64>() {
            return StyleValue::Number(num);
        }
        
        // Check for number with unit (e.g., 16px)
        if let Some(pos) = value.find(|c: char| c.is_alphabetic()) {
            if let Ok(num) = value[..pos].parse::<f64>() {
                return StyleValue::Unit(num, value[pos..].to_string());
            }
        }
        
        // Check for booleans
        if value == "true" {
            return StyleValue::Boolean(true);
        }
        if value == "false" {
            return StyleValue::Boolean(false);
        }
        
        // Check for named colors
        let lower = value.to_lowercase();
        if ["red", "green", "blue", "white", "black", "gray", "grey", 
            "yellow", "orange", "purple", "pink", "brown", "transparent"].contains(&lower.as_str()) {
            return StyleValue::Color(value.to_string());
        }
        
        // Default to string
        StyleValue::String(value.to_string())
    }
}