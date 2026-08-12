//! Slattery UI Interpreter
//! 
//! Handles component instantiation, property binding, event handling,
//! and state management for the Slattery UI framework.

use crate::slattery::sla_lexer::UiToken;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq)]
pub enum UiValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Function(String),
    Null,
}

#[derive(Debug, Clone)]
pub struct UiProperty {
    pub name: String,
    pub value: UiValue,
}

#[derive(Debug, Clone)]
pub struct UiEvent {
    pub name: String,
    pub handler: String,
}

#[derive(Debug, Clone)]
pub struct UiComponent {
    pub component_type: String,
    pub identity: Option<String>,
    pub properties: HashMap<String, UiValue>,
    pub children: Vec<Rc<RefCell<UiComponent>>>,
    pub events: HashMap<String, String>,
    pub parent: Option<Rc<RefCell<UiComponent>>>,
    pub relationship_type: Option<String>,
}

impl UiComponent {
    pub fn new(component_type: String) -> Self {
        Self {
            component_type,
            identity: None,
            properties: HashMap::new(),
            children: Vec::new(),
            events: HashMap::new(),
            parent: None,
            relationship_type: None,
        }
    }
    
    pub fn set_property(&mut self, name: String, value: UiValue) {
        self.properties.insert(name, value);
    }
    
    pub fn get_property(&self, name: &str) -> Option<&UiValue> {
        self.properties.get(name)
    }
    
    pub fn add_child(&mut self, child: Rc<RefCell<UiComponent>>) {
        self.children.push(child);
    }
    
    pub fn set_event(&mut self, event_name: String, handler: String) {
        self.events.insert(event_name, handler);
    }
    
    pub fn set_identity(&mut self, identity: String) {
        self.identity = Some(identity);
    }
    
    pub fn set_parent(&mut self, parent: Rc<RefCell<UiComponent>>) {
        self.parent = Some(parent);
    }
    
    pub fn set_relationship_type(&mut self, relationship: String) {
        self.relationship_type = Some(relationship);
    }
}

pub struct UiInterpreter {
    pub components: HashMap<String, Rc<RefCell<UiComponent>>>,
    current_component: Option<Rc<RefCell<UiComponent>>>,
    pub functions: HashMap<String, Vec<UiToken>>,
}

impl UiInterpreter {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            current_component: None,
            functions: HashMap::new(),
        }
    }

    fn parse_property_into_component(
        &mut self,
        component: &Rc<RefCell<UiComponent>>,
        tokens: &[UiToken],
        start: usize,
    ) -> Result<usize, String> {
        let mut i = start;

        let prop_name: String = match tokens.get(i) {
            Some(UiToken::Identifier(name)) => name.clone(),
            Some(UiToken::OnTap) => "on_tap".to_string(),
            Some(UiToken::OnClick) => "on_click".to_string(),
            Some(UiToken::OnChange) => "on_change".to_string(),
            Some(UiToken::OnInput) => "on_input".to_string(),
            Some(UiToken::Identity) => "Identity".to_string(),
            Some(UiToken::Parent) => return Err("Parent relationship should be handled at component level".to_string()),
            Some(UiToken::Child) => return Err("Child relationship should be handled at component level".to_string()),
            other => return Err(format!("Expected property name, got {:?}", other)),
        };
        i += 1;

        if matches!(tokens.get(i), Some(UiToken::Colon)) {
            i += 1;
        } else if prop_name != "Identity" {
            return Err("Expected ':' after property name".to_string());
        }

        match prop_name.as_str() {
            "Parent" => {
                let (parent_component, new_i) = self
                    .parse_component_instantiation(tokens, i)?
                    .ok_or_else(|| "Failed to parse parent component".to_string())?;
                component.borrow_mut().set_parent(parent_component.clone());
                component.borrow_mut().set_relationship_type("parent".to_string());
                parent_component.borrow_mut().add_child(component.clone());
                Ok(new_i)
            }
            "Child" => {
                let (child_component, new_i) = self
                    .parse_component_instantiation(tokens, i)?
                    .ok_or_else(|| "Failed to parse child component".to_string())?;
                component.borrow_mut().set_relationship_type("child".to_string());
                component.borrow_mut().add_child(child_component.clone());
                child_component.borrow_mut().set_parent(component.clone());
                Ok(new_i)
            }
            "child" => {
                let (child, new_i) = self
                    .parse_component_instantiation(tokens, i)?
                    .ok_or_else(|| "Failed to parse child component".to_string())?;
                component.borrow_mut().add_child(child);
                Ok(new_i)
            }
            "Identity" => {
                let id = match tokens.get(i) {
                    Some(UiToken::Identifier(name)) => name.clone(),
                    other => return Err(format!("Expected identifier after Identity, got {:?}", other)),
                };
                i += 1;
                component.borrow_mut().set_identity(id);
                Ok(i)
            }
            "children" => {
                if !matches!(tokens.get(i), Some(UiToken::LeftBracket)) {
                    return Err("Expected '[' after children:".to_string());
                }
                i += 1;

                while i < tokens.len() && !matches!(tokens[i], UiToken::RightBracket) {
                    let (child, new_i) = self
                        .parse_component_instantiation(tokens, i)?
                        .ok_or_else(|| "Failed to parse child component in children array".to_string())?;
                    component.borrow_mut().add_child(child);
                    i = new_i;
                    
                    if new_i <= i {
                        return Err("Failed to advance position in children array parsing".to_string());
                    }

                    if i < tokens.len() && matches!(tokens[i], UiToken::Comma) {
                        i += 1;
                    }
                }

                if !matches!(tokens.get(i), Some(UiToken::RightBracket)) {
                    return Err("Expected ']' to close children array".to_string());
                }
                i += 1;
                Ok(i)
            }
            "on_tap" | "on_click" | "on_change" | "on_input" => {
                let handler_name = match tokens.get(i) {
                    Some(UiToken::Identifier(name)) => name.clone(),
                    other => {
                        return Err(format!(
                            "Expected handler identifier after {}:, got {:?}",
                            prop_name, other
                        ))
                    }
                };
                i += 1;

                if matches!(tokens.get(i), Some(UiToken::LessThan)) {
                    i += 1;
                    while i < tokens.len() && !matches!(tokens[i], UiToken::GreaterThan) {
                        i += 1;
                    }
                    if matches!(tokens.get(i), Some(UiToken::GreaterThan)) {
                        i += 1;
                    } else {
                        return Err("Expected '>' after function arguments in handler call".to_string());
                    }
                }

                component
                    .borrow_mut()
                    .set_event(prop_name.to_string(), handler_name);
                Ok(i)
            }
            _ => {
                let value = match tokens.get(i) {
                    Some(UiToken::Identifier(name)) => UiValue::String(name.clone()),
                    Some(UiToken::String(s)) => UiValue::String(s.clone()),
                    Some(UiToken::Number(n)) => UiValue::Number(*n),
                    Some(UiToken::True) => UiValue::Boolean(true),
                    Some(UiToken::False) => UiValue::Boolean(false),
                    other => {
                        return Err(format!(
                            "Unexpected value token for property '{}': {:?}",
                            prop_name, other
                        ))
                    }
                };
                i += 1;
                component.borrow_mut().set_property(prop_name, value);
                Ok(i)
            }
        }
    }
    
    pub fn interpret(&mut self, tokens: Vec<UiToken>) -> Result<Vec<Rc<RefCell<UiComponent>>>, String> {
        let mut components = Vec::new();
        let mut i = 0;
        
        while i < tokens.len() {
            match &tokens[i] {
                UiToken::Make => {
                    if let Some((component, new_i)) = self.parse_component_definition(&tokens, i)? {
                        components.push(component);
                        i = new_i;
                    } else {
                        return Err("Failed to parse component definition".to_string());
                    }
                }
                UiToken::Render => {
                    if let Some((component, new_i)) = self.parse_render_statement(&tokens, i)? {
                        components.push(component);
                        i = new_i;
                    } else {
                        return Err("Failed to parse render statement".to_string());
                    }
                }
                UiToken::Func => {
                    if let Some(new_i) = self.parse_function_definition(&tokens, i)? {
                        i = new_i;
                    } else {
                        return Err("Failed to parse function definition".to_string());
                    }
                }
                UiToken::EOF => break,
                _ => i += 1,
            }
        }
        
        Ok(components)
    }
    
    fn parse_component_definition(&mut self, tokens: &[UiToken], start: usize) -> Result<Option<(Rc<RefCell<UiComponent>>, usize)>, String> {
        let mut i = start + 1;
        
        let component_name = if i < tokens.len() {
            if let UiToken::Identifier(name) = &tokens[i] {
                i += 1;
                name.clone()
            } else {
                return Err("Expected component name after 'make'".to_string());
            }
        } else {
            return Err("Unexpected end of tokens".to_string());
        };
        
        if i < tokens.len() && matches!(tokens[i], UiToken::Equal) {
            i += 1;
        } else {
            return Err("Expected '=' after component name".to_string());
        }
        
        if let Some((component, new_i)) = self.parse_component_instantiation(tokens, i)? {
            if component.borrow().identity.is_none() {
                component.borrow_mut().set_identity(component_name.clone());
            }
            self.components.insert(component_name.clone(), component.clone());
            Ok(Some((component, new_i)))
        } else {
            Err("Failed to parse component instantiation".to_string())
        }
    }
    
    fn parse_component_instantiation(&mut self, tokens: &[UiToken], start: usize) -> Result<Option<(Rc<RefCell<UiComponent>>, usize)>, String> {
        let mut i = start;
        
        let component_type = if i < tokens.len() {
            match &tokens[i] {
                UiToken::Window => "Window".to_string(),
                UiToken::Column => "Column".to_string(),
                UiToken::Row => "Row".to_string(),
                UiToken::Text => "Text".to_string(),
                UiToken::Button => "Button".to_string(),
                UiToken::Input => "Input".to_string(),
                UiToken::Parent | UiToken::Child => {
                    return Err("Parent/Child relationships should be inside component braces".to_string());
                }
                _ => return Err("Expected component type".to_string()),
            }
        } else {
            return Err("Unexpected end of tokens".to_string());
        };
        i += 1;
        
        let component = Rc::new(RefCell::new(UiComponent::new(component_type.clone())));

        if i < tokens.len() && matches!(tokens[i], UiToken::LeftBrace) {
            i += 1;
            
            while i < tokens.len() && !matches!(tokens[i], UiToken::RightBrace) {
                match &tokens[i] {
                    UiToken::Parent => {
                        i += 1;
                        if i < tokens.len() && matches!(tokens[i], UiToken::Colon) {
                            i += 1;
                            
                            if let Some((parent_component, new_i)) = self.parse_simple_component_reference(tokens, i)? {
                                component.borrow_mut().set_parent(parent_component.clone());
                                component.borrow_mut().set_relationship_type("parent".to_string());
                                parent_component.borrow_mut().add_child(component.clone());
                                i = new_i;
                            } else {
                                return Err("Failed to parse parent component".to_string());
                            }
                        } else {
                            return Err("Expected ':' after Parent".to_string());
                        }
                    }
                    UiToken::Child => {
                        i += 1;
                        
                        if i < tokens.len() && matches!(tokens[i], UiToken::Slash) {
                            i += 1;
                            if i < tokens.len() && matches!(tokens[i], UiToken::Parent) {
                                i += 1;
                                if i < tokens.len() && matches!(tokens[i], UiToken::Colon) {
                                    i += 1;
                                    
                                    if let Some((child_parent_component, new_i)) = self.parse_simple_component_reference(tokens, i)? {
                                        component.borrow_mut().set_relationship_type("child/parent".to_string());
                                        component.borrow_mut().add_child(child_parent_component.clone());
                                        child_parent_component.borrow_mut().set_parent(component.clone());
                                        i = new_i;
                                    } else {
                                        return Err("Failed to parse child/parent component".to_string());
                                    }
                                } else {
                                    return Err("Expected ':' after Child/Parent".to_string());
                                }
                            } else {
                                return Err("Expected 'Parent' after Child/".to_string());
                            }
                        } else if i < tokens.len() && matches!(tokens[i], UiToken::Colon) {
                            i += 1;
                            
                            if let Some((child_component, new_i)) = self.parse_simple_component_reference(tokens, i)? {
                                component.borrow_mut().set_relationship_type("child".to_string());
                                component.borrow_mut().add_child(child_component.clone());
                                child_component.borrow_mut().set_parent(component.clone());
                                i = new_i;
                            } else {
                                return Err("Failed to parse child component".to_string());
                            }
                        } else {
                            return Err("Expected ':' or '/Parent' after Child".to_string());
                        }
                    }
                    _ => {
                        i = self.parse_property_into_component(&component, tokens, i)?;
                    }
                }
                
                if i < tokens.len() && matches!(tokens[i], UiToken::Comma) {
                    i += 1;
                }
            }
            
            if i < tokens.len() && matches!(tokens[i], UiToken::RightBrace) {
                i += 1;
            } else {
                return Err("Expected '}'".to_string());
            }
        }
        
        Ok(Some((component, i)))
    }
    
    fn parse_attribute(&self, tokens: &[UiToken], start: usize) -> Option<(String, UiValue, usize)> {
        let mut i = start;
        
        let attr_name = if i < tokens.len() {
            match &tokens[i] {
                UiToken::Identity => "Identity".to_string(),
                UiToken::Identifier(name) => name.clone(),
                _ => return None,
            }
        } else {
            return None;
        };
        i += 1;
        
        if i < tokens.len() && matches!(tokens[i], UiToken::Equal) {
            i += 1;
        } else {
            return None;
        }
        
        if i < tokens.len() {
            let value = match &tokens[i] {
                UiToken::Identifier(name) => UiValue::String(name.clone()),
                UiToken::String(s) => UiValue::String(s.clone()),
                UiToken::Number(n) => UiValue::Number(*n),
                UiToken::True => UiValue::Boolean(true),
                UiToken::False => UiValue::Boolean(false),
                _ => return None,
            };
            i += 1;
            
            Some((attr_name, value, i))
        } else {
            None
        }
    }
    
    fn parse_property(&self, tokens: &[UiToken], start: usize) -> Option<(String, UiValue, usize)> {
        let mut i = start;
        
        let prop_name = if i < tokens.len() {
            match &tokens[i] {
                UiToken::Identifier(name) => name.clone(),
                _ => return None,
            }
        } else {
            return None;
        };
        i += 1;
        
        if i < tokens.len() && matches!(tokens[i], UiToken::Colon) {
            i += 1;
        } else {
            return None;
        }
        
        if i < tokens.len() {
            let value = match &tokens[i] {
                UiToken::Identifier(name) => {
                    if i + 1 < tokens.len() && matches!(tokens[i + 1], UiToken::LessThan) {
                        UiValue::String(format!("component:{}", name))
                    } else {
                        UiValue::String(name.clone())
                    }
                }
                UiToken::String(s) => UiValue::String(s.clone()),
                UiToken::Number(n) => UiValue::Number(*n),
                UiToken::True => UiValue::Boolean(true),
                UiToken::False => UiValue::Boolean(false),
                UiToken::LeftBracket => {
                    i += 1;
                    let mut count = 0;
                    let mut depth = 0;
                    
                    while i < tokens.len() && (depth > 0 || !matches!(tokens[i], UiToken::RightBracket)) {
                        match &tokens[i] {
                            UiToken::LeftBracket => depth += 1,
                            UiToken::RightBracket => {
                                if depth == 0 { break; }
                                depth -= 1;
                            }
                            UiToken::Identifier(name) if depth == 0 => {
                                if ["Window", "Column", "Row", "Text", "Button", "Input"].contains(&name.as_str()) {
                                    count += 1;
                                }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                    
                    if i < tokens.len() && matches!(tokens[i], UiToken::RightBracket) {
                        i += 1;
                    }
                    
                    UiValue::String(format!("array:{}", count))
                }
                UiToken::Func => UiValue::String("function".to_string()),
                UiToken::Get => UiValue::String("get".to_string()),
                UiToken::Identity => UiValue::String("identity".to_string()),
                UiToken::Rewrite => UiValue::String("rewrite".to_string()),
                _ => return None,
            };
            
            Some((prop_name, value, i))
        } else {
            None
        }
    }
    
    fn parse_render_statement(&mut self, tokens: &[UiToken], start: usize) -> Result<Option<(Rc<RefCell<UiComponent>>, usize)>, String> {
        let mut i = start + 1;
        
        if i < tokens.len() && matches!(tokens[i], UiToken::LessThan) {
            i += 1;
            
            if i < tokens.len() {
                if let UiToken::Identifier(component_name) = &tokens[i] {
                    if let Some(component) = self.components.get(component_name) {
                        i += 1;
                        
                        if i < tokens.len() && matches!(tokens[i], UiToken::GreaterThan) {
                            i += 1;
                            return Ok(Some((component.clone(), i)));
                        }
                    }
                }
            }
        }
        
        Err("Failed to parse render statement".to_string())
    }
    
    fn parse_function_definition(&mut self, tokens: &[UiToken], start: usize) -> Result<Option<usize>, String> {
        let mut i = start + 1;
        
        if i < tokens.len() {
            if let UiToken::Identifier(func_name) = &tokens[i] {
                i += 1;
                
                if i < tokens.len() {
                    if matches!(tokens[i], UiToken::LessThan) {
                        i += 1;
                        
                        while i < tokens.len() && !matches!(tokens[i], UiToken::GreaterThan) {
                            i += 1;
                        }
                        
                        if i < tokens.len() && matches!(tokens[i], UiToken::GreaterThan) {
                            i += 1;
                        } else {
                            return Err(format!("Expected '>' after function parameters for '{}'", func_name));
                        }
                    } else {
                        return Err(format!("Expected '<' after function name '{}', found: {:?}", func_name, tokens[i]));
                    }
                } else {
                    return Err(format!("Expected '<' after function name '{}', but no more tokens", func_name));
                }
                
                let mut body_tokens: Vec<UiToken> = Vec::new();
                
                if i < tokens.len() && matches!(tokens[i], UiToken::LeftBrace) {
                    i += 1;
                    let mut brace_depth = 1;
                    
                    while i < tokens.len() && brace_depth > 0 {
                        match &tokens[i] {
                            UiToken::LeftBrace => {
                                brace_depth += 1;
                                body_tokens.push(tokens[i].clone());
                                i += 1;
                            }
                            UiToken::RightBrace => {
                                brace_depth -= 1;
                                if brace_depth > 0 {
                                    body_tokens.push(tokens[i].clone());
                                }
                                i += 1;
                            }
                            UiToken::Func | UiToken::Render | UiToken::EOF if brace_depth == 1 => {
                                break;
                            }
                            _ => {
                                body_tokens.push(tokens[i].clone());
                                i += 1;
                            }
                        }
                    }
                } else {
                    return Err(format!("Expected '{{' after function parameters for '{}'", func_name));
                }
                
                self.functions.insert(func_name.clone(), body_tokens);
                return Ok(Some(i));
            }
        }
        
        Err("Failed to parse function definition".to_string())
    }
    
    pub fn get_component_by_identity(&self, identity: &str) -> Option<Rc<RefCell<UiComponent>>> {
        for component in self.components.values() {
            let comp = component.borrow();
            if let Some(id) = &comp.identity {
                if id == identity {
                    return Some(component.clone());
                }
            }
        }
        None
    }
    
    fn parse_simple_component_reference(&mut self, tokens: &[UiToken], start: usize) -> Result<Option<(Rc<RefCell<UiComponent>>, usize)>, String> {
        let mut i = start;
        
        if i < tokens.len() && matches!(tokens[i], UiToken::LessThan) {
            i += 1;
        } else {
            return Ok(None);
        }
        
        let component_type = if i < tokens.len() {
            match &tokens[i] {
                UiToken::Window => "Window".to_string(),
                UiToken::Column => "Column".to_string(),
                UiToken::Row => "Row".to_string(),
                UiToken::Text => "Text".to_string(),
                UiToken::Button => "Button".to_string(),
                UiToken::Input => "Input".to_string(),
                _ => return Err("Expected component type".to_string()),
            }
        } else {
            return Err("Unexpected end of tokens".to_string());
        };
        i += 1;
        
        let component = Rc::new(RefCell::new(UiComponent::new(component_type.clone())));
        
        if i < tokens.len() && matches!(tokens[i], UiToken::GreaterThan) {
            i += 1;
        } else {
            return Err("Expected '>' after component type".to_string());
        }

        if i < tokens.len() && matches!(tokens[i], UiToken::LeftBrace) {
            i += 1;

            while i < tokens.len() && !matches!(tokens[i], UiToken::RightBrace) {
                match &tokens[i] {
                    UiToken::Child => {
                        i += 1;
                        if i < tokens.len() && matches!(tokens[i], UiToken::Colon) {
                            i += 1;
                            if let Some((child, new_i)) = self.parse_simple_component_reference(tokens, i)? {
                                component.borrow_mut().add_child(child.clone());
                                child.borrow_mut().set_parent(component.clone());
                                i = new_i;
                            } else {
                                return Err("Failed to parse child component in brace body".to_string());
                            }
                        } else {
                            return Err("Expected ':' after Child".to_string());
                        }
                    }
                    UiToken::Parent => {
                        i += 1;
                        if i < tokens.len() && matches!(tokens[i], UiToken::Colon) {
                            i += 1;
                            if let Some((parent, new_i)) = self.parse_simple_component_reference(tokens, i)? {
                                component.borrow_mut().set_parent(parent.clone());
                                parent.borrow_mut().add_child(component.clone());
                                i = new_i;
                            } else {
                                return Err("Failed to parse parent component in brace body".to_string());
                            }
                        } else {
                            return Err("Expected ':' after Parent".to_string());
                        }
                    }
                    _ => {
                        i = self.parse_property_into_component(&component, tokens, i)?;
                    }
                }

                if i < tokens.len() && matches!(tokens[i], UiToken::Comma) {
                    i += 1;
                }
            }

            if i < tokens.len() && matches!(tokens[i], UiToken::RightBrace) {
                i += 1;
            } else {
                return Err("Expected '}' to close component body".to_string());
            }
        }
        
        Ok(Some((component, i)))
    }

    pub fn rewrite_component_property(&mut self, identity: &str, property: &str, value: UiValue) -> Result<(), String> {
        if let Some(component) = self.get_component_by_identity(identity) {
            component.borrow_mut().set_property(property.to_string(), value);
            Ok(())
        } else {
            Err(format!("Component with identity '{}' not found", identity))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_component_creation() {
        let mut interpreter = UiInterpreter::new();
        
        let tokens = vec![
            UiToken::Make,
            UiToken::Identifier("App".to_string()),
            UiToken::Equal,
            UiToken::Identifier("Window".to_string()),
            UiToken::LeftBrace,
            UiToken::Identifier("title".to_string()),
            UiToken::Colon,
            UiToken::String("Test App".to_string()),
            UiToken::RightBrace,
            UiToken::EOF,
        ];
        
        let result = interpreter.interpret(tokens);
        assert!(result.is_ok());
        
        let components = result.unwrap();
        assert_eq!(components.len(), 1);
        
        let app = components[0].borrow();
        assert_eq!(app.component_type, "Window");
        assert_eq!(app.get_property("title"), Some(&UiValue::String("Test App".to_string())));
    }
    
    #[test]
    fn test_parent_child_relationship() {
        let mut interpreter = UiInterpreter::new();
        
        let tokens = vec![
            UiToken::Make,
            UiToken::Identifier("App".to_string()),
            UiToken::Equal,
            UiToken::Identifier("Window".to_string()),
            UiToken::LeftBrace,
            UiToken::Parent,
            UiToken::Colon,
            UiToken::LessThan,
            UiToken::Identifier("Column".to_string()),
            UiToken::GreaterThan,
            UiToken::RightBrace,
            UiToken::EOF,
        ];
        
        let result = interpreter.interpret(tokens);
        assert!(result.is_ok());
        
        let components = result.unwrap();
        assert_eq!(components.len(), 1);
        
        let app = components[0].borrow();
        assert_eq!(app.component_type, "Window");
        assert_eq!(app.relationship_type, Some("parent".to_string()));
    }
}