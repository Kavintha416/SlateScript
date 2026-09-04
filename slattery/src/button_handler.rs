// slattery/src/button_handler.rs

use slate_core::ast_interpreter::AstInterpreter;
use slate_core::value::Value;
use slate_core::lexer::Token;
use std::collections::HashMap;
use std::io::Write;

pub struct ButtonHandler {
    pub interpreter: AstInterpreter,
    pub function_cache: HashMap<String, Vec<Token>>,
}

impl ButtonHandler {
    pub fn new() -> Self {
        Self {
            interpreter: AstInterpreter::new(),
            function_cache: HashMap::new(),
        }
    }

    pub fn register_function(&mut self, name: &str, tokens: Vec<Token>) {
        self.function_cache.insert(name.to_string(), tokens);
    }

    pub fn execute_handler(&mut self, handler_name: &str, _args: &[Value]) -> Result<(), String> {
        if let Some(tokens) = self.function_cache.get(handler_name).cloned() {
            // Register native functions needed for execution
            self.interpreter.register_native_function("write".to_string(), Box::new(|args| {
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
            
            self.interpreter.register_native_function("Rewrite".to_string(), Box::new(|_args| {
                Ok(Value::Null)
            }));
            
            // Run the function body
            if let Err(e) = self.interpreter.run(&tokens) {
                return Err(e);
            }
            
            return Ok(());
        }

        Err(format!("Handler '{}' not found in cache", handler_name))
    }

    pub fn set_interpreter(&mut self, interpreter: AstInterpreter) {
        self.interpreter = interpreter;
    }
    
    pub fn get_function_names(&self) -> Vec<String> {
        self.function_cache.keys().cloned().collect()
    }
}