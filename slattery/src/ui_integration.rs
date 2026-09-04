// slattery/src/ui_integration.rs

use crate::sla_lexer::UiLexer;
use crate::sla_interpreter::UiInterpreter;
use crate::egui_renderer::EguiRenderer;
use slate_core::lexer::Token;
use slate_core::value::Value;
use std::path::Path;
use slate_sfile::SFileExtension;
use std::io::Write;

pub struct UiFramework {
    lexer: UiLexer,
    interpreter: UiInterpreter,
    renderer: EguiRenderer,
    sfile_extension: SFileExtension,
}

impl UiFramework {
    pub fn new() -> Self {
        Self {
            lexer: UiLexer::new(""),
            interpreter: UiInterpreter::new(),
            renderer: EguiRenderer::new(),
            sfile_extension: SFileExtension::new(),
        }
    }
    
    pub fn parse_and_render(&mut self, source: &str, _script_path: Option<&Path>) -> Result<(), String> {
        if let Some(path) = _script_path {
            self.sfile_extension.set_current_file(path);
        }
        
        self.process_imports(source, _script_path)?;
        
        self.lexer = UiLexer::new(source);
        let tokens = self.lexer.tokenize();
        
        let components = self.interpreter.interpret(tokens)
            .map_err(|e| format!("UI interpretation error: {}", e))?;
        
        let mut component_map = std::collections::HashMap::new();
        for (index, comp) in components.iter().enumerate() {
            let comp_borrowed = comp.borrow();
            
            if let Some(name) = comp_borrowed.identity.as_ref() {
                if !component_map.contains_key(name) {
                    component_map.insert(name.clone(), comp.clone());
                }
            } else {
                component_map.insert(format!("component:{}", index), comp.clone());
            }
        }
        

        
        let mut renderer = crate::egui_renderer::EguiRenderer::new();
        renderer.set_components(component_map);
        
        renderer.load_styles_from_source(source);
        self.load_styles_from_imports(&mut renderer)?;
        
        // Register functions from interpreter
        for (func_name, func_tokens) in &self.interpreter.functions {
            let main_tokens = convert_ui_tokens_to_main_tokens(func_tokens);
            
            let mut func_def_tokens = Vec::new();
            func_def_tokens.push(Token::Func);
            func_def_tokens.push(Token::Identifier(func_name.clone()));
            func_def_tokens.push(Token::LessThan);
            
            let params = extract_parameters(&main_tokens);
            
            for (i, param) in params.iter().enumerate() {
                if i > 0 {
                    func_def_tokens.push(Token::Comma);
                }
                func_def_tokens.push(Token::Identifier(param.clone()));
            }
            
            func_def_tokens.push(Token::GreaterThan);
            func_def_tokens.push(Token::LeftBrace);
            
            let body_tokens = extract_body_tokens(&main_tokens);
            func_def_tokens.extend(body_tokens);
            
            func_def_tokens.push(Token::RightBrace);
            func_def_tokens.push(Token::EOF);
            
            renderer.function_cache.insert(func_name.clone(), func_def_tokens.clone());
            
            // Register write function with flush
            renderer.interpreter.register_native_function("write".to_string(), Box::new(|args| {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        print!(" ");
                    }
                    print!("{}", arg.to_string());
                }
                println!();
                Write::flush(&mut std::io::stdout()).unwrap_or_default();
                Ok(Value::Null)
            }));
            
            if let Err(e) = renderer.interpreter.run(&func_def_tokens) {
                eprintln!("Interpreter registration failed: {}", e);
            }
        }
        
        self.register_imported_functions(&mut renderer)?;
        
        crate::egui_renderer::run_egui_app_with_renderer(renderer)
            .map_err(|e| format!("Failed to render egui app: {}", e))
    }
    
    fn process_imports(&self, source: &str, _script_path: Option<&Path>) -> Result<(), String> {
        let mut lexer = UiLexer::new(source);
        let tokens = lexer.tokenize();
        
        let mut i = 0;
        while i < tokens.len() {
            if let crate::sla_lexer::UiToken::Import = &tokens[i] {
                i += 1;
                
                if i < tokens.len() && matches!(tokens[i], crate::sla_lexer::UiToken::From) {
                    i += 1;
                    
                    if i < tokens.len() {
                        if let crate::sla_lexer::UiToken::String(_s) = &tokens[i] {
                            i += 1;
                            
                            if i < tokens.len() && 
                               (matches!(tokens[i], crate::sla_lexer::UiToken::LeftBrace) || 
                                matches!(tokens[i], crate::sla_lexer::UiToken::LeftBracket)) {
                                i += 1;
                                
                                let mut file_paths = Vec::new();
                                while i < tokens.len() && 
                                      !matches!(tokens[i], crate::sla_lexer::UiToken::RightBrace) &&
                                      !matches!(tokens[i], crate::sla_lexer::UiToken::RightBracket) {
                                    if let crate::sla_lexer::UiToken::String(path) = &tokens[i] {
                                        file_paths.push(path.clone());
                                    }
                                    i += 1;
                                }
                                
                                self.sfile_extension.import_files(&file_paths)?;
                            }
                        }
                    }
                }
            }
            i += 1;
        }
        
        Ok(())
    }
    
    fn load_styles_from_imports(&self, renderer: &mut EguiRenderer) -> Result<(), String> {
        let loaded_files = self.sfile_extension.get_loaded_files();
        
        for (path, _) in loaded_files {
            if let Ok(source) = std::fs::read_to_string(&path) {
                renderer.load_styles_from_source(&source);
            }
        }
        
        Ok(())
    }
    
    fn register_imported_functions(&self, renderer: &mut EguiRenderer) -> Result<(), String> {
        let imported_functions = self.sfile_extension.get_all_imported_functions();
        
        for (name, params, body) in imported_functions {
            let mut func_tokens = Vec::new();
            func_tokens.push(Token::Func);
            func_tokens.push(Token::Identifier(name.clone()));
            func_tokens.push(Token::LessThan);
            
            for (i, param) in params.iter().enumerate() {
                if i > 0 {
                    func_tokens.push(Token::Comma);
                }
                func_tokens.push(Token::Identifier(param.clone()));
            }
            
            func_tokens.push(Token::GreaterThan);
            func_tokens.push(Token::LeftBrace);
            
            // Convert body statements to tokens (simplified)
            for _stmt in &body {
                // This would need proper statement-to-token conversion
            }
            
            func_tokens.push(Token::RightBrace);
            func_tokens.push(Token::EOF);
            
            renderer.function_cache.insert(name.clone(), func_tokens);
        }
        
        Ok(())
    }
}

// Add this public function here
pub fn collect_style_files(_base_dir: Option<&Path>) -> Vec<String> {
    let mut style_files = Vec::new();
    
    // Check current directory for .sts files
    if let Ok(entries) = std::fs::read_dir(".") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("sts") {
                style_files.push(path.to_string_lossy().to_string());
            }
        }
    }
    
    // Check for styles directory
    let styles_dir = std::path::Path::new("styles");
    if styles_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(styles_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("sts") {
                    style_files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    
    // Check for src directory
    let src_dir = std::path::Path::new("src");
    if src_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("sts") {
                    style_files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    
    style_files
}

// Helper functions
fn convert_ui_tokens_to_main_tokens(tokens: &[crate::sla_lexer::UiToken]) -> Vec<Token> {
    let mut result = Vec::new();
    
    for token in tokens {
        match token {
            crate::sla_lexer::UiToken::Identifier(name) => {
                // Check if it's a known function name
                if name == "write" {
                    result.push(Token::Write);
                } else {
                    result.push(Token::Identifier(name.clone()));
                }
            }
            crate::sla_lexer::UiToken::String(s) => {
                result.push(Token::String(s.clone()));
            }
            crate::sla_lexer::UiToken::Number(n) => {
                if *n == n.floor() {
                    result.push(Token::Number(*n as i64));
                } else {
                    result.push(Token::Float(*n));
                }
            }
            crate::sla_lexer::UiToken::True => {
                result.push(Token::True);
            }
            crate::sla_lexer::UiToken::False => {
                result.push(Token::False);
            }
            crate::sla_lexer::UiToken::LeftParen => {
                result.push(Token::LeftParen);
            }
            crate::sla_lexer::UiToken::RightParen => {
                result.push(Token::RightParen);
            }
            crate::sla_lexer::UiToken::Comma => {
                result.push(Token::Comma);
            }
            crate::sla_lexer::UiToken::Colon => {
                result.push(Token::Colon);
            }
            crate::sla_lexer::UiToken::LessThan => {
                result.push(Token::LessThan);
            }
            crate::sla_lexer::UiToken::GreaterThan => {
                result.push(Token::GreaterThan);
            }
            crate::sla_lexer::UiToken::LeftBrace => {
                result.push(Token::LeftBrace);
            }
            crate::sla_lexer::UiToken::RightBrace => {
                result.push(Token::RightBrace);
            }
            crate::sla_lexer::UiToken::Semicolon => {
                result.push(Token::Semicolon);
            }
            crate::sla_lexer::UiToken::Equal => {
                result.push(Token::Equal);
            }
            crate::sla_lexer::UiToken::EqualEqual => {
                result.push(Token::EqualEqual);
            }
            crate::sla_lexer::UiToken::Rewrite => {
                result.push(Token::Rewrite);
            }
            crate::sla_lexer::UiToken::Func => {
                result.push(Token::Func);
            }
            _ => {}
        }
    }
    
    result
}

fn extract_parameters(tokens: &[Token]) -> Vec<String> {
    let mut params = Vec::new();
    let mut in_params = false;
    
    for token in tokens {
        match token {
            Token::LessThan => {
                in_params = true;
            }
            Token::GreaterThan => {
                in_params = false;
            }
            Token::Identifier(name) if in_params => {
                params.push(name.clone());
            }
            _ => {}
        }
    }
    
    params
}

fn extract_body_tokens(tokens: &[Token]) -> Vec<Token> {
    let mut body = Vec::new();
    let mut in_body = false;
    let mut brace_depth = 0;
    
    for token in tokens {
        match token {
            Token::LeftBrace => {
                if brace_depth == 0 {
                    in_body = true;
                }
                brace_depth += 1;
                if brace_depth > 1 {
                    body.push(token.clone());
                }
            }
            Token::RightBrace => {
                brace_depth -= 1;
                if brace_depth > 0 {
                    body.push(token.clone());
                } else {
                    in_body = false;
                }
            }
            _ if in_body => {
                body.push(token.clone());
            }
            _ => {}
        }
    }
    
    body
}