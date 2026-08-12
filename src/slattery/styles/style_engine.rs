//! Style Engine
//! 
//! Manages style rules, cascading, and provides style resolution for components.

use std::collections::HashMap;
use super::style_interpreter::{StyleRule, StyleValue, StyleState, StyleInterpreter};

#[derive(Clone)]
pub struct StyleEngine {
    rules: Vec<StyleRule>,
    component_styles: HashMap<String, Vec<StyleRule>>,
}

impl StyleEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            component_styles: HashMap::new(),
        }
    }
    
    pub fn parse_styles(&mut self, content: &str) -> Result<(), String> {
        let mut interpreter = StyleInterpreter::new();
        let mut lexer = super::style_lexer::StyleLexer::new(content);
        let tokens = lexer.tokenize();
        
        let rules = interpreter.interpret(tokens)
            .map_err(|e| format!("Failed to parse styles: {}", e))?;
        
        self.add_rules(rules);
        Ok(())
    }
    
    pub fn add_rules(&mut self, rules: Vec<StyleRule>) {
        for rule in rules {
            self.add_rule(rule);
        }
    }
    
    pub fn add_rule(&mut self, rule: StyleRule) {
        // Extract component name from selector if it's a component selector
        if rule.selector.starts_with('@') {
            let component_name = rule.selector[1..].split(':').next().unwrap_or(&rule.selector[1..]);
            self.component_styles
                .entry(component_name.to_string())
                .or_insert_with(Vec::new)
                .push(rule);
        } else {
            self.rules.push(rule);
        }
    }
    
    pub fn get_style_for_component(&self, component_type: &str, identity: Option<&str>, classes: &[String], state: StyleState) -> HashMap<String, StyleValue> {
        let mut styles = HashMap::new();
        
        // 1. Component type styles (@Button) — lowest specificity
        if let Some(component_rules) = self.component_styles.get(component_type) {
            for rule in component_rules {
                if self.rule_matches(rule, component_type, identity, classes, state) {
                    for (prop, value) in &rule.properties {
                        styles.insert(prop.clone(), value.clone());
                    }
                }
            }
        }

        // 2. Global rules — class (.foo) and id (#id) selectors, higher specificity
        for rule in &self.rules {
            if self.rule_matches(rule, component_type, identity, classes, state) {
                for (prop, value) in &rule.properties {
                    if !styles.contains_key(prop) || rule.important
                        || rule.selector.starts_with('#')   // id beats type
                        || rule.selector.starts_with('.')   // class beats type
                    {
                        styles.insert(prop.clone(), value.clone());
                    }
                }
            }
        }
        
        styles
    }
    
    fn rule_matches(&self, rule: &StyleRule, component_type: &str, identity: Option<&str>, classes: &[String], state: StyleState) -> bool {
        let selector = &rule.selector;
        
        // Check state match
        let state_match = match state {
            StyleState::Normal => !selector.contains(':'),
            StyleState::Hover => selector.contains(":hover"),
            StyleState::Active => selector.contains(":active"),
            StyleState::Focus => selector.contains(":focus"),
            StyleState::Disabled => selector.contains(":disabled"),
        };
        
        if !state_match {
            return false;
        }
        
        let selector_no_state = selector.split(':').next().unwrap_or(selector);
        
        // Component selector (@Component)
        if selector_no_state.starts_with('@') {
            let component_name = &selector_no_state[1..];
            return component_name == component_type;
        }
        
        // ID selector (#id)
        if selector_no_state.starts_with('#') {
            let id_name = &selector_no_state[1..];
            return identity.map_or(false, |id| id == id_name);
        }
        
        // Class selector (.class)
        if selector_no_state.starts_with('.') {
            let class_name = &selector_no_state[1..];
            return classes.contains(&class_name.to_string());
        }
        
        // Universal selector (*) matches everything
        if selector_no_state == "*" {
            return true;
        }
        
        // Type selector (component name)
        selector_no_state == component_type
    }
    
    pub fn load_from_string(&mut self, content: &str) -> Result<(), String> {
        let mut lexer = super::style_lexer::StyleLexer::new(content);
        let tokens = lexer.tokenize();
        
        let mut interpreter = super::style_interpreter::StyleInterpreter::new();
        let rules = interpreter.interpret(tokens)?;
        
        self.add_rules(rules);
        Ok(())
    }
    
    pub fn load_from_file(&mut self, file_path: &str) -> Result<(), String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read style file '{}': {}", file_path, e))?;
        
        self.load_from_string(&content)
    }
}

// CSS property mappings to egui
impl StyleEngine {
    pub fn parse_color(&self, color: &str) -> Option<egui::Color32> {
        // Parse hex colors (#RRGGBB or #RRGGBBAA)
        if color.starts_with('#') {
            let hex = &color[1..];
            if hex.len() == 6 {
                if let Ok(r) = u8::from_str_radix(&hex[0..2], 16) {
                    if let Ok(g) = u8::from_str_radix(&hex[2..4], 16) {
                        if let Ok(b) = u8::from_str_radix(&hex[4..6], 16) {
                            return Some(egui::Color32::from_rgb(r, g, b));
                        }
                    }
                }
            }
        }
        
        // Parse named colors
        match color.to_lowercase().as_str() {
            "red" => Some(egui::Color32::RED),
            "green" => Some(egui::Color32::GREEN),
            "blue" => Some(egui::Color32::BLUE),
            "white" => Some(egui::Color32::WHITE),
            "black" => Some(egui::Color32::BLACK),
            "gray" | "grey" => Some(egui::Color32::GRAY),
            "yellow" => Some(egui::Color32::YELLOW),
            "orange" => Some(egui::Color32::from_rgb(255, 165, 0)),
            "purple" => Some(egui::Color32::from_rgb(128, 0, 128)),
            "pink" => Some(egui::Color32::from_rgb(255, 192, 203)),
            "brown" => Some(egui::Color32::from_rgb(165, 42, 42)),
            "transparent" => Some(egui::Color32::TRANSPARENT),
            _ => None,
        }
    }
}
