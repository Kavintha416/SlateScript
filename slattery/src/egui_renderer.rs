// src/slattery/egui_renderer.rs

use crate::sla_interpreter::{UiComponent, UiValue};
use crate::styles::{StyleApplier, StyleEngine};
use crate::styles::style_interpreter::StyleValue;
use crate::logger::DevLogger;
use slate_core::ast_interpreter::AstInterpreter;
use slate_core::value::Value;
use slate_core::lexer::Token;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Instant;
use std::path::Path;
use egui;

// ============================================================
//  DEVTOOLS TABS (ASCII Only)
// ============================================================

#[derive(PartialEq, Clone, Copy)]
pub enum DevtoolsTab {
    Elements,
    Styles,
    Console,
    Performance,
}

impl DevtoolsTab {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Elements => "Elements",
            Self::Styles   => "Styles",
            Self::Console  => "Console",
            Self::Performance => "Performance",
        }
    }
}

// ============================================================
//  DEVTOOLS THEME (Dark, Professional)
// ============================================================

struct DevtoolsTheme;

impl DevtoolsTheme {
    fn header_color() -> egui::Color32 {
        egui::Color32::from_rgb(255, 59, 48) // Slate red
    }
    
    fn bg_color() -> egui::Color32 {
        egui::Color32::from_rgb(30, 30, 30)
    }
    
    fn panel_bg() -> egui::Color32 {
        egui::Color32::from_rgb(25, 25, 25)
    }
    
    fn text_color() -> egui::Color32 {
        egui::Color32::from_rgb(240, 240, 240)
    }
    
    fn muted_text() -> egui::Color32 {
        egui::Color32::from_rgb(160, 160, 160)
    }
    
    fn accent_color() -> egui::Color32 {
        egui::Color32::from_rgb(100, 180, 255)
    }
    
    fn border_color() -> egui::Color32 {
        egui::Color32::from_rgb(60, 60, 60)
    }
}

// ============================================================
//  EGUI RENDERER
// ============================================================

pub struct EguiRenderer {
    pub components: HashMap<String, Rc<RefCell<UiComponent>>>,
    pub ui_state: HashMap<String, String>,
    pub style_engine: Rc<RefCell<StyleEngine>>,
    pub style_applier: StyleApplier,
    pub interpreter: AstInterpreter,
    pub logger: DevLogger,
    pub function_cache: HashMap<String, Vec<Token>>,
    pub render_time: Option<f64>,
    pub frame_count: usize,
    pub last_frame_time: Instant,
    pub fps: f64,
}

impl EguiRenderer {
    pub fn new() -> Self {
        let style_engine = Rc::new(RefCell::new(StyleEngine::new()));
        let style_applier = StyleApplier::new(style_engine.clone());
        Self {
            components: HashMap::new(),
            ui_state: HashMap::new(),
            style_engine,
            style_applier,
            interpreter: AstInterpreter::new(),
            logger: DevLogger::new(),
            function_cache: HashMap::new(),
            render_time: None,
            frame_count: 0,
            last_frame_time: Instant::now(),
            fps: 0.0,
        }
    }

    pub fn with_logger(mut self, logger: DevLogger) -> Self {
        self.logger = logger;
        self
    }

    pub fn new_with_handler(button_handler: crate::button_handler::ButtonHandler) -> Self {
        let mut renderer = Self::new();
        renderer.function_cache = button_handler.function_cache;
        renderer.interpreter = button_handler.interpreter;
        renderer
    }

    pub fn set_components(&mut self, components: HashMap<String, Rc<RefCell<UiComponent>>>) {
        self.components = components;
        self.logger.log_info(&format!("Loaded {} components", self.components.len()));
    }

    pub fn register_function(&mut self, name: &str, tokens: Vec<Token>) {
        self.function_cache.insert(name.to_string(), tokens);
        self.logger.log_debug(&format!("Registered function: {}", name));
    }

    pub fn get_function_count(&self) -> usize {
        self.function_cache.len()
    }

    pub fn get_last_render_time(&self) -> Option<f64> {
        self.render_time
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.logger.get_logs()
    }

    pub fn clear_logs(&self) {
        self.logger.clear();
    }

    pub fn evaluate_command(&mut self, command: &str) -> Result<String, String> {
        let mut temp_interpreter = AstInterpreter::new();
        
        let tokens = vec![
            Token::Identifier("write".to_string()),
            Token::LeftParen,
            Token::String(command.to_string()),
            Token::RightParen,
            Token::EOF,
        ];
        
        match temp_interpreter.run(&tokens) {
            Ok(_) => Ok(format!("Executed: {}", command)),
            Err(e) => Err(format!("Failed: {}", e)),
        }
    }

    pub fn load_styles(&mut self, style_files: &[String]) {
        for file_path in style_files {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                if let Err(e) = self.style_engine.borrow_mut().parse_style_blocks(&content) {
                    self.logger.log_warn(&format!("Failed to load styles from {}: {}", file_path, e));
                } else {
                    self.logger.log_info(&format!("Loaded styles from: {}", file_path));
                }
            }
        }
    }

    // Load styles directly from source
    pub fn load_styles_from_source(&mut self, source: &str) {
        if let Err(e) = self.style_engine.borrow_mut().parse_style_blocks(source) {
            self.logger.log_warn(&format!("Failed to parse style blocks from source: {}", e));
        } else {
            self.logger.log_info("Loaded style blocks from source");
        }
    }

    // ============================================================
    //  INTERPRETER HELPER METHODS
    // ============================================================

    pub fn has_function(&self, name: &str) -> bool {
        if self.function_cache.contains_key(name) {
            return true;
        }
        self.interpreter.has_function(name)
    }

    pub fn list_functions(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for (name, _) in &self.function_cache {
            names.push(name.clone());
        }
        names
    }

    pub fn get_interpreter_mut(&mut self) -> &mut AstInterpreter {
        &mut self.interpreter
    }

    // ============================================================
    //  EXECUTE SLATE FUNCTION
    // ============================================================

    pub fn execute_slate_function(&mut self, function_name: &str, args: &[Value]) -> Result<(), String> {
        println!("[DEBUG] execute_slate_function called: {} with {:?} args", function_name, args);
        
        // Check if function exists in cache
        if let Some(tokens) = self.function_cache.get(function_name).cloned() {
            println!("[DEBUG] Found function '{}' in cache with {} tokens", function_name, tokens.len());
            
            // Log the tokens for debugging
            for (i, token) in tokens.iter().enumerate() {
                println!("[DEBUG]   Token {}: {:?}", i, token);
            }
            
            // Create a fresh interpreter for this execution
            let mut temp_interpreter = AstInterpreter::new();
            
            // Register native functions needed
            temp_interpreter.register_native_function("write".to_string(), Box::new(|args| {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        print!(" ");
                    }
                    print!("{}", arg.to_string());
                }
                println!();
                Ok(Value::Null)
            }));
            
            temp_interpreter.register_native_function("Rewrite".to_string(), Box::new(|_args| {
                Ok(Value::Null)
            }));
            
            // Run the function tokens
            println!("[DEBUG] Running function tokens through interpreter...");
            if let Err(e) = temp_interpreter.run(&tokens) {
                println!("[DEBUG] Interpreter error: {}", e);
                return Err(e);
            }
            println!("[DEBUG] Interpreter execution successful");
            
            // Process any Rewrite statements
            println!("[DEBUG] Processing Rewrite statements...");
            self.process_rewrite_statements(&tokens, args)?;
            
            println!("[DEBUG] Handler '{}' execution completed", function_name);
            return Ok(());
        }
        
        // If not in cache, check if it's in the interpreter
        if self.interpreter.has_function(function_name) {
            println!("[DEBUG] Function '{}' found in interpreter", function_name);
            // Try to execute using interpreter
            return Ok(());
        }
        
        println!("[DEBUG] Function '{}' NOT FOUND", function_name);
        Err(format!("Function '{}' not found in interpreter", function_name))
    }
    
    fn strip_rewrite_statements(&self, tokens: &[Token]) -> Vec<Token> {
        let mut result = Vec::new();
        let mut i = 0;
        
        while i < tokens.len() {
            if matches!(tokens[i], Token::Rewrite) {
                i += 1;
                if i < tokens.len() && matches!(tokens[i], Token::Identifier(_)) {
                    i += 1;
                }
                if i < tokens.len() && matches!(tokens[i], Token::LeftBrace) {
                    let mut brace_count = 1;
                    i += 1;
                    while i < tokens.len() && brace_count > 0 {
                        match &tokens[i] {
                            Token::LeftBrace => brace_count += 1,
                            Token::RightBrace => brace_count -= 1,
                            _ => {}
                        }
                        i += 1;
                    }
                }
            } else {
                result.push(tokens[i].clone());
                i += 1;
            }
        }
        
        result
    }
    
    fn process_rewrite_statements(&mut self, tokens: &[Token], _args: &[Value]) -> Result<(), String> {
        let mut i = 0;
        let mut found_rewrites = 0;
        
        while i < tokens.len() {
            match &tokens[i] {
                Token::Rewrite => {
                    println!("[DEBUG] Found Rewrite statement at position {}", i);
                    found_rewrites += 1;
                    i += 1;
                    
                    // Next should be component identity
                    if i < tokens.len() {
                        if let Token::Identifier(component_id) = &tokens[i] {
                            println!("[DEBUG] Rewrite target component: {}", component_id);
                            i += 1;
                            
                            // Next should be LeftBrace
                            if i < tokens.len() && matches!(tokens[i], Token::LeftBrace) {
                                i += 1;
                                
                                // Parse properties
                                let mut properties_to_update = std::collections::HashMap::new();
                                
                                while i < tokens.len() && !matches!(tokens[i], Token::RightBrace) {
                                    if let Token::Identifier(prop_name) = &tokens[i] {
                                        println!("[DEBUG]   Property: {}", prop_name);
                                        i += 1;
                                        
                                        if i < tokens.len() && matches!(tokens[i], Token::Colon) {
                                            i += 1;
                                            
                                            // Parse the value expression
                                            let mut value_tokens = Vec::new();
                                            let mut paren_count = 0;
                                            
                                            while i < tokens.len() {
                                                match &tokens[i] {
                                                    Token::LeftParen => {
                                                        paren_count += 1;
                                                        value_tokens.push(tokens[i].clone());
                                                        i += 1;
                                                    }
                                                    Token::RightParen => {
                                                        if paren_count > 0 {
                                                            paren_count -= 1;
                                                            value_tokens.push(tokens[i].clone());
                                                            i += 1;
                                                        } else {
                                                            break;
                                                        }
                                                    }
                                                    Token::Comma | Token::RightBrace if paren_count == 0 => {
                                                        break;
                                                    }
                                                    _ => {
                                                        value_tokens.push(tokens[i].clone());
                                                        i += 1;
                                                    }
                                                }
                                            }
                                            
                                            // Evaluate the expression
                                            if let Some(value) = self.evaluate_expression(&value_tokens) {
                                                println!("[DEBUG]   Value: {}", value);
                                                properties_to_update.insert(prop_name.clone(), value);
                                            }
                                        }
                                    } else {
                                        i += 1;
                                    }
                                }
                                
                                // Apply the rewrite
                                for (prop, value) in properties_to_update {
                                    println!("[DEBUG] Rewriting component '{}' property '{}' = '{}'", 
                                            component_id, prop, value);
                                    if let Err(e) = self.rewrite_component(component_id, &prop, &value) {
                                        println!("[DEBUG] Failed to rewrite: {}", e);
                                        self.logger.log_warn(&format!("Failed to rewrite component: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {
                    i += 1;
                }
            }
        }
        
        if found_rewrites > 0 {
            println!("[DEBUG] Processed {} rewrite statements", found_rewrites);
        }
        Ok(())
    }
    
    fn evaluate_expression(&mut self, tokens: &[Token]) -> Option<String> {
        println!("[DEBUG] Evaluating expression: {:?}", tokens);
        
        // Simple evaluation for now
        let mut result = String::new();
        let mut i = 0;
        
        while i < tokens.len() {
            match &tokens[i] {
                Token::String(s) => {
                    result.push_str(s);
                    i += 1;
                }
                Token::Identifier(id) => {
                    result.push_str(id);
                    i += 1;
                }
                Token::Number(n) => {
                    result.push_str(&n.to_string());
                    i += 1;
                }
                Token::Float(f) => {
                    result.push_str(&f.to_string());
                    i += 1;
                }
                Token::Plus => {
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }
        
        Some(result)
    }
    
    fn simple_evaluate(&self, tokens: &[Token]) -> Option<String> {
        let mut result = String::new();
        let mut i = 0;
        
        while i < tokens.len() {
            match &tokens[i] {
                Token::String(s) => {
                    result.push_str(s);
                    i += 1;
                }
                Token::Identifier(id) => {
                    result.push_str(id);
                    i += 1;
                }
                Token::Number(n) => {
                    result.push_str(&n.to_string());
                    i += 1;
                }
                Token::Plus => {
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }
        
        Some(result)
    }

    // ============================================================
    //  RENDER COMPONENTS
    // ============================================================

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let start = Instant::now();
        self.frame_count += 1;

        if self.frame_count % 30 == 0 {
            let elapsed = self.last_frame_time.elapsed();
            self.fps = 30.0 / elapsed.as_secs_f64();
            self.last_frame_time = Instant::now();
        }

        // Fill entire window with white background
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, egui::Color32::WHITE);

        // Get the window component
        let window = self.components.values()
            .find(|c| c.borrow().component_type == "Window")
            .cloned();

        if let Some(wc) = window {
            let window_comp = wc.borrow();
            
            // 1. Set the OS window title from the 'title' property
            let title = window_comp.get_property("title")
                .and_then(|v| if let UiValue::String(s) = v { Some(s.clone()) } else { None })
                .unwrap_or_else(|| "SlateScript App".to_string());
            
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(title));
            
            // 2. Render children directly
            self.render_window(ui, &wc);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No Window component found")
                        .size(20.0)
                        .color(egui::Color32::GRAY)
                );
            });
        }

        self.render_time = Some(start.elapsed().as_secs_f64() * 1000.0);
    }

    fn render_window(&mut self, ui: &mut egui::Ui, window_component: &Rc<RefCell<UiComponent>>) {
        let window = window_component.borrow();
        
        let styles = self.style_applier.resolve_styles(&window);
        self.style_applier.apply_window_styles(&window, &styles, ui);

        // Set the OS window title (NOT rendered as text inside)
        let title = window.get_property("title")
            .and_then(|v| if let UiValue::String(s) = v { Some(s.clone()) } else { None })
            .unwrap_or_else(|| "SlateScript App".to_string());
        
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(title));

        // Render children directly
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(12, 12))
            .show(ui, |ui| {
                for child in &window.children {
                    self.render_component(ui, child);
                }

                if let Some(UiValue::String(child_ref)) = window.get_property("child") {
                    if let Some(child_comp) = self.components.get(child_ref).cloned() {
                        self.render_component(ui, &child_comp);
                    }
                }

                if let Some(UiValue::String(children_ref)) = window.get_property("children") {
                    if children_ref.starts_with("array:") {
                        let current_id = window.identity.as_ref().map(|s| s.as_str()).unwrap_or("");
                        let children_to_render: Vec<_> = self.components.iter()
                            .filter(|(name, child_comp)| {
                                let comp_type = child_comp.borrow().component_type.clone();
                                comp_type != "Window" && name.as_str() != current_id
                            })
                            .map(|(_, child_comp)| child_comp.clone())
                            .collect();
                        for child_comp in children_to_render {
                            self.render_component(ui, &child_comp);
                        }
                    }
                }
            });
    }

    fn render_component(&mut self, ui: &mut egui::Ui, component: &Rc<RefCell<UiComponent>>) {
        let comp = component.borrow();

        match comp.component_type.as_str() {
            "Column" => {
                ui.vertical(|ui| {
                    if let Some(UiValue::Number(spacing)) = comp.get_property("spacing") {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, *spacing as f32);
                    }
                    self.render_column_children(ui, &comp);
                });
            }

            "Row" => {
                ui.horizontal(|ui| {
                    if let Some(UiValue::Number(spacing)) = comp.get_property("spacing") {
                        ui.spacing_mut().item_spacing = egui::vec2(*spacing as f32, 0.0);
                    }
                    self.render_row_children(ui, &comp);
                });
            }

            "Text" => {
                let text_value = comp.get_property("value")
                    .and_then(|v| if let UiValue::String(s) = v { Some(s.clone()) } else { None })
                    .unwrap_or_else(|| "Text".to_string());
                
                let display_text = if let Some(id) = &comp.identity {
                    self.ui_state.get(id).cloned().unwrap_or(text_value)
                } else {
                    text_value
                };

                if comp.children.is_empty() {
                    self.style_applier.apply_text_styles(&comp, ui, &display_text);
                } else {
                    self.render_text_container(ui, &comp, &display_text);
                }
            }

            "Button" => {
                println!("[DEBUG] Button rendering - component: {:?}", comp.component_type);
                println!("[DEBUG] Button events: {:?}", comp.events);
                println!("[DEBUG] Button identity: {:?}", comp.identity);
                println!("[DEBUG] Button properties: {:?}", comp.properties);
                
                let clicked = self.style_applier.apply_button_styles(&comp, ui);
                
                if clicked {
                    println!("[DEBUG] Button CLICKED!");
                    
                    // Get the on_click handler
                    if let Some(handler_name) = comp.events.get("on_click").cloned() {
                        println!("[DEBUG] Found on_click handler: {}", handler_name);
                        
                        // Build arguments from properties
                        let mut args = Vec::new();
                        if let Some(UiValue::String(arg)) = comp.get_property("arg") {
                            args.push(Value::String(arg.clone()));
                        }
                        
                        // Log before execution
                        self.logger.log_info(&format!("Button clicked - executing: {}", handler_name));
                        
                        // Execute the handler
                        match self.execute_slate_function(&handler_name, &args) {
                            Ok(_) => {
                                println!("[DEBUG] Handler '{}' executed successfully", handler_name);
                                self.logger.log_info(&format!("Handler '{}' executed successfully", handler_name));
                            }
                            Err(e) => {
                                println!("[DEBUG] Handler '{}' failed: {}", handler_name, e);
                                self.logger.log_error(&format!("Handler '{}' failed: {}", handler_name, e));
                            }
                        }
                    } else {
                        println!("[DEBUG] No on_click handler found!");
                        self.logger.log_warn("Button clicked but no on_click handler defined");
                    }
                }
            }

            "Input" => {
                let id = comp.identity.as_ref().unwrap_or(&"unknown".to_string()).clone();
                let placeholder = comp.get_property("placeholder")
                    .and_then(|v| if let UiValue::String(s) = v { Some(s.clone()) } else { None })
                    .unwrap_or_else(|| "Enter text...".to_string());
                
                let mut text = self.ui_state
                    .get(&format!("input_{}", id))
                    .cloned()
                    .unwrap_or_default();

                let response = ui.add(
                    egui::TextEdit::singleline(&mut text)
                        .hint_text(placeholder)
                        .desired_width(200.0)
                );

                if response.changed() {
                    self.ui_state.insert(format!("input_{}", id), text.clone());
                    
                    if let Some(handler_name) = comp.events.get("on_change").cloned() {
                        self.logger.log_info(&format!("Input changed - executing: {}", handler_name));
                        let args = vec![Value::String(text.clone())];
                        
                        match self.execute_slate_function(&handler_name, &args) {
                            Ok(_) => {
                                self.logger.log_info(&format!("Handler '{}' executed successfully", handler_name));
                            }
                            Err(e) => {
                                self.logger.log_error(&format!("on_change handler '{}' failed: {}", handler_name, e));
                            }
                        }
                    }
                }
            }

            _ => {
                ui.label(
                    egui::RichText::new(format!("Unknown component: {}", comp.component_type))
                        .color(egui::Color32::YELLOW)
                );
                self.logger.log_warn(&format!("Unknown component type: {}", comp.component_type));
            }
        }
    }

    fn render_text_container(&mut self, ui: &mut egui::Ui, comp: &UiComponent, title: &str) {
        let frame = egui::Frame::new()
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 200, 200)))
            .inner_margin(egui::Margin::symmetric(16, 12))
            .outer_margin(egui::Margin::symmetric(4, 6));

        frame.show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .size(16.0)
                        .color(egui::Color32::from_rgb(50, 50, 50))
                        .strong()
                );
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                for child in &comp.children {
                    self.render_component(ui, child);
                }
            });
        });
    }

    fn render_column_children(&mut self, ui: &mut egui::Ui, column: &UiComponent) {
        for child in &column.children {
            self.render_component(ui, child);
        }
    }

    fn render_row_children(&mut self, ui: &mut egui::Ui, row: &UiComponent) {
        for child in &row.children {
            self.render_component(ui, child);
        }
    }

    // ============================================================
    //  COMPONENT TREE FOR DEVTOOLS
    // ============================================================

    pub fn collect_component_tree(&self) -> Vec<DevtoolsEntry> {
        let mut entries = Vec::new();
        let windows: Vec<_> = self.components.iter()
            .filter(|(_, c)| c.borrow().component_type == "Window")
            .map(|(_, c)| c.clone())
            .collect();

        for w in windows {
            self.collect_recursive(&w, 0, &mut entries);
        }
        entries
    }

    fn collect_recursive(&self, comp: &Rc<RefCell<UiComponent>>, depth: usize, out: &mut Vec<DevtoolsEntry>) {
        let c = comp.borrow();
        let styles = self.style_applier.resolve_styles(&c);
        
        out.push(DevtoolsEntry {
            depth,
            component_type: c.component_type.clone(),
            identity: c.identity.clone(),
            properties: c.properties.clone(),
            styles,
            children_count: c.children.len(),
        });

        for child in &c.children {
            self.collect_recursive(child, depth + 1, out);
        }
    }

    pub fn get_component_by_identity(&self, identity: &str) -> Option<Rc<RefCell<UiComponent>>> {
        self.components.get(identity).cloned()
    }

    pub fn rewrite_component(&mut self, identity: &str, property: &str, value: &str) -> Result<(), String> {
        println!("[DEBUG] rewrite_component called: {} property={} value={}", identity, property, value);
        
        if let Some(comp) = self.get_component_by_identity(identity) {
            let mut comp_borrowed = comp.borrow_mut();
            comp_borrowed.set_property(property.to_string(), UiValue::String(value.to_string()));
            
            // Update UI state for text components
            if property == "value" || property == "text" {
                self.ui_state.insert(identity.to_string(), value.to_string());
            }
            
            println!("[DEBUG] Successfully rewrote component");
            self.logger.log_debug(&format!("Rewrote component '{}' property '{}' = '{}'", identity, property, value));
            Ok(())
        } else {
            let msg = format!("Component '{}' not found", identity);
            println!("[DEBUG] {}", msg);
            self.logger.log_debug(&msg);
            Err(msg)
        }
    }
}

// ============================================================
//  DEVTOOLS ENTRY
// ============================================================

pub struct DevtoolsEntry {
    pub depth: usize,
    pub component_type: String,
    pub identity: Option<String>,
    pub properties: HashMap<String, UiValue>,
    pub styles: HashMap<String, crate::styles::StyleValue>,
    pub children_count: usize,
}

// ============================================================
//  SLATTERY APP (egui Framework)
// ============================================================

pub struct SlatteryApp {
    pub renderer: EguiRenderer,
    pub components: HashMap<String, Rc<RefCell<UiComponent>>>,
    pub devtools_open: bool,
    pub devtools_selected: Option<usize>,
    pub devtools_tab: DevtoolsTab,
    pub console_input: String,
}

impl SlatteryApp {
    pub fn new(components: HashMap<String, Rc<RefCell<UiComponent>>>) -> Self {
        let mut renderer = EguiRenderer::new();
        renderer.set_components(components.clone());
        Self {
            renderer,
            components,
            devtools_open: false,
            devtools_selected: None,
            devtools_tab: DevtoolsTab::Elements,
            console_input: String::new(),
        }
    }

    pub fn new_with_styles(
        components: HashMap<String, Rc<RefCell<UiComponent>>>,
        style_files: Vec<String>,
    ) -> Self {
        let mut app = Self::new(components);
        app.renderer.load_styles(&style_files);
        app
    }

    pub fn new_with_renderer(renderer: EguiRenderer) -> Self {
        let components = renderer.components.clone();
        Self {
            renderer,
            components,
            devtools_open: false,
            devtools_selected: None,
            devtools_tab: DevtoolsTab::Elements,
            console_input: String::new(),
        }
    }

    pub fn get_renderer_mut(&mut self) -> &mut EguiRenderer {
        &mut self.renderer
    }
}

// ============================================================
//  EFRAIM APP IMPLEMENTATION (CORRECTED FOR YOUR VERSION)
// ============================================================

impl eframe::App for SlatteryApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        
        // Toggle devtools with Ctrl+Shift+I
        if ctx.input(|i| {
            i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::I)
        }) {
            self.devtools_open = !self.devtools_open;
            if self.devtools_open {
                self.renderer.logger.log_info("DevTools opened");
            } else {
                self.renderer.logger.log_info("DevTools closed");
            }
        }

        // ============================================================
        //  DEVTOOLS WINDOW
        // ============================================================

        if self.devtools_open {
            egui::Window::new("Slattery DevTools")
                .default_open(true)
                .default_size(egui::vec2(700.0, 450.0))
                .min_size(egui::vec2(500.0, 350.0))
                .resizable(true)
                .title_bar(true)
                .show(&ctx, |ui| {
                    self.render_devtools(ui);
                });
        }

        // ============================================================
        //  MAIN APP - Render directly in the provided ui
        // ============================================================

        self.renderer.render(ui);
    }
}

// ============================================================
//  DEVTOOLS RENDERER (impl SlatteryApp)
// ============================================================

impl SlatteryApp {
    fn render_devtools(&mut self, ui: &mut egui::Ui) {
        // DevTools styling
        let theme_bg = DevtoolsTheme::bg_color();
        let _theme_panel = DevtoolsTheme::panel_bg(); // Fixed: added underscore
        let theme_text = DevtoolsTheme::text_color();
        let theme_muted = DevtoolsTheme::muted_text();
        let theme_accent = DevtoolsTheme::accent_color();
        let theme_border = DevtoolsTheme::border_color();
        
        // Use a frame for the devtools
        egui::Frame::new()
            .fill(theme_bg)
            .stroke(egui::Stroke::new(1.0, theme_border))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Slattery DevTools")
                            .size(16.0)
                            .strong()
                            .color(theme_text)
                    );
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").clicked() {
                            self.devtools_open = false;
                        }
                    });
                });
                
                ui.separator();
                
                // Tab bar
                ui.horizontal(|ui| {
                    let tabs = [
                        DevtoolsTab::Elements,
                        DevtoolsTab::Styles,
                        DevtoolsTab::Console,
                        DevtoolsTab::Performance,
                    ];
                    
                    for tab in tabs {
                        let selected = self.devtools_tab == tab;
                        let button = egui::Button::new(
                            egui::RichText::new(tab.label())
                                .color(if selected { theme_accent } else { theme_muted })
                        );
                        
                        if ui.add(button).clicked() {
                            self.devtools_tab = tab;
                        }
                    }
                });
                
                ui.separator();
                
                // Tab content
                match self.devtools_tab {
                    DevtoolsTab::Elements => self.render_elements_tab(ui),
                    DevtoolsTab::Styles => self.render_styles_tab(ui),
                    DevtoolsTab::Console => self.render_console_tab(ui),
                    DevtoolsTab::Performance => self.render_performance_tab(ui),
                }
            });
    }
    
    fn render_elements_tab(&mut self, ui: &mut egui::Ui) {
        let theme_text = DevtoolsTheme::text_color();
        let theme_muted = DevtoolsTheme::muted_text();
        let theme_accent = DevtoolsTheme::accent_color();
        
        let entries = self.renderer.collect_component_tree();
        
        if entries.is_empty() {
            ui.label(
                egui::RichText::new("No components found")
                    .color(theme_muted)
            );
            return;
        }
        
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (i, entry) in entries.iter().enumerate() {
                    let indent = "  ".repeat(entry.depth);
                    let is_selected = self.devtools_selected == Some(i);
                    
                    let text_color = if is_selected { theme_accent } else { theme_text };
                    
                    let label = format!(
                        "{}{} <{}>{}",
                        indent,
                        entry.component_type,
                        entry.identity.as_deref().unwrap_or(""),
                        if entry.children_count > 0 {
                            format!(" ({} children)", entry.children_count)
                        } else {
                            String::new()
                        }
                    );
                    
                    let response = ui.selectable_label(
                        is_selected,
                        egui::RichText::new(label).color(text_color)
                    );
                    
                    if response.clicked() {
                        self.devtools_selected = Some(i);
                    }
                }
            });
    }
    
    fn render_styles_tab(&mut self, ui: &mut egui::Ui) {
        let theme_text = DevtoolsTheme::text_color();
        let theme_muted = DevtoolsTheme::muted_text();
        
        ui.label(
            egui::RichText::new("Styles")
                .size(14.0)
                .strong()
                .color(theme_text)
        );
        
        ui.separator();
        
        if let Some(selected_idx) = self.devtools_selected {
            let entries = self.renderer.collect_component_tree();
            if let Some(entry) = entries.get(selected_idx) {
                ui.label(
                    egui::RichText::new(format!("Component: {}", entry.component_type))
                        .color(theme_text)
                );
                
                ui.add_space(4.0);
                
                if entry.styles.is_empty() {
                    ui.label(
                        egui::RichText::new("No styles applied")
                            .color(theme_muted)
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (prop, value) in &entry.styles {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(prop)
                                            .color(theme_muted)
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(": {}", format_style_value(value)))
                                            .color(theme_text)
                                    );
                                });
                            }
                        });
                }
            } else {
                ui.label(
                    egui::RichText::new("Select a component in Elements tab")
                        .color(theme_muted)
                );
            }
        } else {
            ui.label(
                egui::RichText::new("Select a component in Elements tab")
                    .color(theme_muted)
            );
        }
    }
    
    fn render_console_tab(&mut self, ui: &mut egui::Ui) {
        let theme_text = DevtoolsTheme::text_color();
        let theme_muted = DevtoolsTheme::muted_text();
        
        ui.label(
            egui::RichText::new("Console")
                .size(14.0)
                .strong()
                .color(theme_text)
        );
        
        ui.separator();
        
        // Logs
        egui::ScrollArea::vertical()
            .id_salt("devtools_console_logs")
            .auto_shrink([false, false])
            .max_height(300.0)
            .show(ui, |ui| {
                let logs = self.renderer.get_logs();
                
                if logs.is_empty() {
                    ui.label(
                        egui::RichText::new("No logs")
                            .color(theme_muted)
                    );
                } else {
                    for log in logs {
                        ui.label(
                            egui::RichText::new(log)
                                .color(theme_text)
                                .monospace()
                        );
                    }
                }
            });
        
        // Input
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(">")
                    .color(theme_text)
                    .monospace()
            );
            
            let input = egui::TextEdit::singleline(&mut self.console_input)
                .desired_width(f32::INFINITY)
                .font(egui::TextStyle::Monospace)
                .hint_text("Enter command...");
            
            let response = ui.add(input);
            
            let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            
            if submitted {
                let command = self.console_input.clone();
                self.console_input.clear();
                
                match self.renderer.evaluate_command(&command) {
                    Ok(result) => {
                        self.renderer.logger.log_info(&result);
                    }
                    Err(e) => {
                        self.renderer.logger.log_error(&e);
                    }
                }
            }
        });
    }
    
    fn render_performance_tab(&mut self, ui: &mut egui::Ui) {
        let theme_text = DevtoolsTheme::text_color();
        let theme_muted = DevtoolsTheme::muted_text();
        
        ui.label(
            egui::RichText::new("Performance")
                .size(14.0)
                .strong()
                .color(theme_text)
        );
        
        ui.separator();
        
        let render_time = self.renderer.get_last_render_time().unwrap_or(0.0);
        let fps = self.renderer.fps;
        let frame_count = self.renderer.frame_count;
        
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Frame count:")
                    .color(theme_muted)
            );
            ui.label(
                egui::RichText::new(frame_count.to_string())
                    .color(theme_text)
            );
        });
        
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("FPS:")
                    .color(theme_muted)
            );
            ui.label(
                egui::RichText::new(format!("{:.1}", fps))
                    .color(theme_text)
            );
        });
        
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Last render time:")
                    .color(theme_muted)
            );
            ui.label(
                egui::RichText::new(format!("{:.2} ms", render_time))
                    .color(theme_text)
            );
        });
        
        ui.separator();
        
        // Component count
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Components:")
                    .color(theme_muted)
            );
            ui.label(
                egui::RichText::new(self.components.len().to_string())
                    .color(theme_text)
            );
        });
        
        // Function count
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Registered functions:")
                    .color(theme_muted)
            );
            ui.label(
                egui::RichText::new(self.renderer.get_function_count().to_string())
                    .color(theme_text)
            );
        });
    }
}

// Helper function to format style values
fn format_style_value(value: &StyleValue) -> String {
    match value {
        StyleValue::Color(c) => format!("color: {}", c),
        StyleValue::Number(n) => format!("number: {}", n),
        StyleValue::String(s) => format!("string: {}", s),
        StyleValue::Unit(n, unit) => format!("unit: {}{}", n, unit),
        StyleValue::Boolean(b) => format!("bool: {}", b),
        StyleValue::None => "none".to_string(),
    }
}

// ============================================================
//  PUBLIC ENTRY POINTS
// ============================================================

pub fn run_egui_app(components: HashMap<String, Rc<RefCell<UiComponent>>>) -> Result<(), String> {
    let style_files = crate::ui_integration::collect_style_files(None);
    run_egui_app_with_styles(components, style_files)
}

pub fn run_egui_app_with_styles(
    components: HashMap<String, Rc<RefCell<UiComponent>>>,
    style_files: Vec<String>,
) -> Result<(), String> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([800.0, 600.0])
        .with_min_inner_size([400.0, 300.0]);

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "SlateScript App",
        native_options,
        Box::new(|_cc| {
            Ok(Box::new(SlatteryApp::new_with_styles(components, style_files)))
        }),
    )
    .map_err(|e| e.to_string())
}

pub fn run_egui_app_with_renderer(renderer: EguiRenderer) -> Result<(), String> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([800.0, 600.0])
        .with_min_inner_size([400.0, 300.0]);

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "SlateScript App",
        native_options,
        Box::new(|_cc| {
            Ok(Box::new(SlatteryApp::new_with_renderer(renderer)))
        }),
    )
    .map_err(|e| e.to_string())
}

// ============================================================
//  EXTRA COMPONENT SUPPORT
// ============================================================

// Add methods for additional component types
impl EguiRenderer {
    // Checkbox component
    pub fn render_checkbox(&mut self, ui: &mut egui::Ui, comp: &UiComponent) {
        let id = comp.identity.as_ref().unwrap_or(&"unknown".to_string()).clone();
        let label = comp.get_property("label")
            .and_then(|v| if let UiValue::String(s) = v { Some(s.clone()) } else { None })
            .unwrap_or_else(|| "Checkbox".to_string());
        
        let mut checked = self.ui_state
            .get(&format!("checkbox_{}", id))
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);
        
        let response = ui.checkbox(&mut checked, label);
        
        if response.changed() {
            self.ui_state.insert(format!("checkbox_{}", id), checked.to_string());
            
            if let Some(handler_name) = comp.events.get("on_change").cloned() {
                let args = vec![Value::Bool(checked)];
                let _ = self.execute_slate_function(&handler_name, &args);
            }
        }
    }
    
    // Image component (FIXED - no Result returned)
    pub fn render_image(&mut self, ui: &mut egui::Ui, comp: &UiComponent) {
        let src = comp.get_property("src")
            .and_then(|v| if let UiValue::String(s) = v { Some(s.clone()) } else { None })
            .unwrap_or_default();
        
        let width = comp.get_property("width")
            .and_then(|v| if let UiValue::Number(n) = v { Some(*n as f32) } else { None })
            .unwrap_or(200.0);
        
        let height = comp.get_property("height")
            .and_then(|v| if let UiValue::Number(n) = v { Some(*n as f32) } else { None })
            .unwrap_or(200.0);
        
        // egui::Image::from_uri doesn't return Result - it returns Image directly
        let image = egui::Image::from_uri(&src);
        let _ = ui.add(image.fit_to_exact_size(egui::vec2(width, height)));
    }
    
    // Slider component
    pub fn render_slider(&mut self, ui: &mut egui::Ui, comp: &UiComponent) {
        let id = comp.identity.as_ref().unwrap_or(&"unknown".to_string()).clone();
        
        let min = comp.get_property("min")
            .and_then(|v| if let UiValue::Number(n) = v { Some(*n as f32) } else { None })
            .unwrap_or(0.0);
        
        let max = comp.get_property("max")
            .and_then(|v| if let UiValue::Number(n) = v { Some(*n as f32) } else { None })
            .unwrap_or(100.0);
        
        let mut value = self.ui_state
            .get(&format!("slider_{}", id))
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(min);
        
        let response = ui.add(
            egui::Slider::new(&mut value, min..=max)
        );
        
        if response.changed() {
            self.ui_state.insert(format!("slider_{}", id), value.to_string());
            
            if let Some(handler_name) = comp.events.get("on_change").cloned() {
                let args = vec![Value::Float(value as f64)];
                let _ = self.execute_slate_function(&handler_name, &args);
            }
        }
    }
}