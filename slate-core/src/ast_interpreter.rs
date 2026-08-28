//! AST-based Interpreter for SlateScript
//! 
//! This interpreter executes AST nodes directly, providing better error handling
//! and more robust function support than the token-based interpreter.

use crate::ast::*;
use crate::value::{Value, ControlFlow, Environment};
use crate::extension::ExtensionRegistry;
use std::collections::HashMap;
use std::cell::RefCell;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AstFunction {
    pub name: String,
    pub parameters: Vec<String>,
    pub body: Vec<Statement>,
}

pub struct AstInterpreter {
    pub environment: Environment,
    functions: HashMap<String, AstFunction>,
    native_functions: HashMap<String, Box<dyn Fn(&[Value]) -> Result<Value, String>>>,
    call_stack: Vec<String>,
    max_call_depth: usize,
    max_iterations: usize,
    extensions: RefCell<Option<ExtensionRegistry>>,
    current_file: Option<PathBuf>,  // Add this field
}

impl AstInterpreter {
    pub fn register_native_function(
        &mut self,
        name: String,
        func: Box<dyn Fn(&[Value]) -> Result<Value, String>>,
    ) {
        self.native_functions.insert(name, func);
    }

    // Add this method to the impl block:
    pub fn set_current_file(&mut self, path: &std::path::Path) {
        self.current_file = Some(path.to_path_buf());
    }

    // Add this method to get the current file:
    pub fn get_current_file(&self) -> Option<&PathBuf> {
        self.current_file.as_ref()
    }

    pub fn new() -> Self {
        let mut interpreter = Self {
            environment: Environment::new(),
            functions: HashMap::new(),
            native_functions: HashMap::new(),
            call_stack: Vec::new(),
            max_call_depth: 1000,
            max_iterations: 100000,
            extensions: RefCell::new(None),
            current_file: None,  // Initialize
        };
        interpreter.register_native_functions();
        interpreter
    }

    pub fn set_extensions(&mut self, extensions: ExtensionRegistry) {
        *self.extensions.borrow_mut() = Some(extensions);
    }

    pub fn get_env_mut(&mut self) -> &mut Environment {
        &mut self.environment
    }

    pub fn register_function(&mut self, name: &str, params: Vec<String>, body: Vec<Statement>) {
        self.functions.insert(
            name.to_string(),
            AstFunction {
                name: name.to_string(),
                parameters: params,
                body,
            },
        );
    }

    /// Check if a function exists in the interpreter
    pub fn has_function(&self, name: &str) -> bool {
        // Check native functions
        for key in self.native_functions.keys() {
            if key.eq_ignore_ascii_case(name) {
                return true;
            }
        }
        
        // Check user-defined functions
        for key in self.functions.keys() {
            if key.eq_ignore_ascii_case(name) {
                return true;
            }
        }
        
        false
    }

    /// Get a function by name (returns Option)
    pub fn get_function(&self, name: &str) -> Option<&AstFunction> {
        // Check user-defined functions - case insensitive
        for (key, func) in &self.functions {
            if key.eq_ignore_ascii_case(name) {
                return Some(func);
            }
        }
        None
    }

    fn register_native_functions(&mut self) {
        // Register write function (with newline)
        self.native_functions.insert("write".to_string(), Box::new(|args| {
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", arg.to_string());
            }
            println!();
            Ok(Value::Null)
        }));

        // Register write_no_newline function
        self.native_functions.insert("write_no_newline".to_string(), Box::new(|args| {
            if args.len() != 1 {
                return Err("write_no_newline() expects exactly 1 argument".to_string());
            }
            print!("{}", args[0].to_string());
            Ok(Value::Null)
        }));

        self.native_functions.insert("generate".to_string(), Box::new(|args| {
            if args.len() != 3 {
                return Err("generate() expects 3 arguments: model_name, prompt, max_tokens".to_string());
            }
            
            let model_name = match &args[0] {
                Value::String(s) => s.clone(),
                _ => return Err("First argument must be a string (model name)".to_string()),
            };
            
            let prompt = match &args[1] {
                Value::String(s) => s.clone(),
                _ => return Err("Second argument must be a string (prompt)".to_string()),
            };
            
            let max_tokens = match &args[2] {
                Value::Int(n) => *n as usize,
                Value::Float(f) => *f as usize,
                _ => return Err("Third argument must be a number (max tokens)".to_string()),
            };
            
            // Try to use sLLM extension if available
            // For now, return a placeholder with the model name
            Ok(Value::String(format!(
                "[sLLM] Generated from '{}': '{}' ({} tokens)",
                model_name, prompt, max_tokens
            )))
        }));
        // Register type checking function
        self.native_functions.insert("type".to_string(), Box::new(|args| {
            if args.len() != 1 {
                return Err("type() expects exactly 1 argument".to_string());
            }
            let type_name = match args[0] {
                Value::String(_) => "string",
                Value::Int(_) => "int",
                Value::Float(_) => "float",
                Value::Bool(_) => "bool",
                Value::Null => "null",
                Value::Array(_) => "array",
                Value::Object(_) => "object",
            };
            Ok(Value::String(type_name.to_string()))
        }));

        // Register len function
        self.native_functions.insert("len".to_string(), Box::new(|args| {
            if args.len() != 1 {
                return Err("len() expects exactly 1 argument".to_string());
            }
            match &args[0] {
                Value::String(s) => Ok(Value::Int(s.len() as i64)),
                Value::Array(a) => Ok(Value::Int(a.len() as i64)),
                Value::Object(o) => Ok(Value::Int(o.len() as i64)),
                _ => Err("len() only works on strings, arrays, and objects".to_string()),
            }
        }));

        // Register to_string function
        self.native_functions.insert("to_string".to_string(), Box::new(|args| {
            if args.len() != 1 {
                return Err("to_string() expects exactly 1 argument".to_string());
            }
            Ok(Value::String(args[0].to_string()))
        }));

        // Register to_int function
        self.native_functions.insert("to_int".to_string(), Box::new(|args| {
            if args.len() != 1 {
                return Err("to_int() expects exactly 1 argument".to_string());
            }
            match &args[0] {
                Value::String(s) => {
                    s.parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| format!("Cannot convert '{}' to int", s))
                }
                Value::Float(f) => Ok(Value::Int(*f as i64)),
                Value::Int(i) => Ok(Value::Int(*i)),
                _ => Err("Cannot convert value to int".to_string()),
            }
        }));

        // Register to_float function
        self.native_functions.insert("to_float".to_string(), Box::new(|args| {
            if args.len() != 1 {
                return Err("to_float() expects exactly 1 argument".to_string());
            }
            match &args[0] {
                Value::String(s) => {
                    s.parse::<f64>()
                        .map(Value::Float)
                        .map_err(|_| format!("Cannot convert '{}' to float", s))
                }
                Value::Int(i) => Ok(Value::Float(*i as f64)),
                Value::Float(f) => Ok(Value::Float(*f)),
                _ => Err("Cannot convert value to float".to_string()),
            }
        }));
    }

    pub fn interpret(&mut self, program: &Program) -> Result<Value, String> {
        // Initialize loop counter for while loops
        self.environment.set("__loop_counter", Value::Int(0));
        
        for statement in &program.statements {
            match self.execute_statement(statement)? {
                ControlFlow::Return(v) => return Ok(v),
                ControlFlow::Break => break,
                ControlFlow::Continue => continue,
            }
        }
        Ok(Value::Null)
    }

    pub fn execute_statement(&mut self, statement: &Statement) -> Result<ControlFlow, String> {
        match statement {
            Statement::Expression(expr) => {
                self.evaluate_expression(expr)?;
                Ok(ControlFlow::Continue)
            }
            
            Statement::Assignment { name, value, span: _ } => {
                let val = self.evaluate_expression(value)?;
                self.environment.set(name, val);
                Ok(ControlFlow::Continue)
            }
            
            Statement::FunctionDefinition { name, parameters, body, .. } => {
                let func = AstFunction {
                    name: name.clone(),
                    parameters: parameters.clone(),
                    body: body.clone(),
                };
                self.functions.insert(name.clone(), func);
                Ok(ControlFlow::Continue)
            }
            
            Statement::Write { value, .. } => {
                let val = self.evaluate_expression(value)?;
                if let Some(write_func) = self.native_functions.get("write") {
                    write_func(&[val])?;
                }
                Ok(ControlFlow::Continue)
            }
            
            Statement::If { condition, then_branch, elif_branches, else_branch, .. } => {
                let cond_val = self.evaluate_expression(condition)?;
                if cond_val.is_truthy() {
                    return self.execute_block(then_branch);
                }
                
                for (elif_cond, elif_body) in elif_branches {
                    let elif_val = self.evaluate_expression(elif_cond)?;
                    if elif_val.is_truthy() {
                        return self.execute_block(elif_body);
                    }
                }
                
                if let Some(else_branch) = else_branch {
                    return self.execute_block(else_branch);
                }
                
                Ok(ControlFlow::Continue)
            }
            
            Statement::While { condition, body, counter_var, span: _ } => {
                // If there's a counter variable, initialize it to 0
                if let Some(var_name) = counter_var {
                    self.environment.set(var_name, Value::Int(0));
                }
                
                let mut iterations = 0;
                while iterations < self.max_iterations {
                    let cond_val = self.evaluate_expression(condition)?;
                    if !cond_val.is_truthy() {
                        break;
                    }
                    
                    match self.execute_block(body)? {
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::Break => break,
                        ControlFlow::Continue => (),
                    }
                    
                    iterations += 1;
                }
                
                if iterations >= self.max_iterations {
                    return Err("Maximum loop iterations exceeded".to_string());
                }
                
                Ok(ControlFlow::Continue)
            }
            
            Statement::ImportStatement { from, items: _, .. } => {
                // Handle imports - for now just register package functions
                if from == "slattery" {
                    // Slattery UI imports are handled by the UI system
                }
                Ok(ControlFlow::Continue)
            }
            
            Statement::Return { value, .. } => {
                let val = match value {
                    Some(expr) => self.evaluate_expression(expr)?,
                    None => Value::Null,
                };
                Ok(ControlFlow::Return(val))
            }
            
            Statement::Train { model, dataset, config, .. } => {
                let model_val = self.evaluate_expression(model)?;
                let dataset_val = self.evaluate_expression(dataset)?;
                
                // Extract model and dataset names
                let model_name = match &model_val {
                    Value::String(s) => s.clone(),
                    _ => return Err("Model must be a string identifier".to_string()),
                };
                
                let dataset_name = match &dataset_val {
                    Value::String(s) => s.clone(),
                    _ => return Err("Dataset must be a string identifier".to_string()),
                };
                
                // Parse config values into string map for sLLM extension
                let mut config_map = HashMap::new();
                for (key, expr) in config {
                    let value = self.evaluate_expression(expr)?;
                    let value_str = match value {
                        Value::Int(n) => n.to_string(),
                        Value::Float(n) => n.to_string(),
                        Value::String(s) => s,
                        Value::Bool(b) => b.to_string(),
                        _ => return Err(format!("Config value for '{}' must be a primitive type", key)),
                    };
                    config_map.insert(key.clone(), value_str);
                }
                
                // Try to use sLLM extension if available
                if let Some(extensions) = self.extensions.borrow_mut().as_mut() {
                    for ext in extensions.get_extensions_mut() {
                        if ext.name() == "sllm" {
                            // Use the extension's interface if available
                            // The extension should handle the train_model call through its trait
                            // For now, use the fallback implementation
                            break;
                        }
                    }
                }
                
                // Fallback to basic training simulation
                let epochs = config_map.get("epochs")
                    .and_then(|e| e.parse().ok())
                    .unwrap_or(3);
                let learning_rate = config_map.get("learning_rate")
                    .and_then(|lr| lr.parse().ok())
                    .unwrap_or(3e-4);
                
                println!("[sLLM] Starting training...");
                println!("  Model: {}", model_name);
                println!("  Dataset: {}", dataset_name);
                println!("  Epochs: {}", epochs);
                println!("  Learning Rate: {}", learning_rate);
                
                for epoch in 0..epochs {
                    println!("  Epoch {}/{}", epoch + 1, epochs);
                }
                
                println!("[sLLM] Training complete!");
                
                Ok(ControlFlow::Continue)
            }
        }
    }

    fn execute_block(&mut self, statements: &[Statement]) -> Result<ControlFlow, String> {
        for stmt in statements {
            match self.execute_statement(stmt)? {
                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                ControlFlow::Break => return Ok(ControlFlow::Break),
                ControlFlow::Continue => continue,
            }
        }
        Ok(ControlFlow::Continue)
    }

    // ============ EXPRESSION EVALUATION ============

    pub fn evaluate_expression(&mut self, expression: &Expression) -> Result<Value, String> {
        // First, handle extension expressions
        if let Expression::Extension { name, type_name, .. } = expression {
            return Ok(Value::String(format!("Extension: {} ({})", name, type_name)));
        }

        // Handle UI components
        if let Expression::UiComponent { component_type, .. } = expression {
            return Ok(Value::String(format!("UI Component: {:?}", component_type)));
        }

        if let Expression::UiRender { .. } = expression {
            return Ok(Value::Null);
        }

        // For other expressions, try extensions first
        let has_extension_handler = {
            if let Some(extensions) = self.extensions.borrow().as_ref() {
                let mut found = false;
                for ext in extensions.get_extensions() {
                    if ext.handles_expression(expression) {
                        found = true;
                        break;
                    }
                }
                found
            } else {
                false
            }
        };

        if has_extension_handler {
            return Ok(Value::String("Extension handled".to_string()));
        }

        // Core evaluation
        match expression {
            Expression::Literal(literal, _) => self.evaluate_literal(literal),
            
            Expression::Variable(name, span) => {
                self.environment.get(name)
                    .cloned()
                    .ok_or_else(|| format!("Variable '{}' not found at line {}", name, span.line))
            }
            
            Expression::Binary { left, operator, right, .. } => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                self.evaluate_binary_operation(&left_val, operator, &right_val)
            }
            
            Expression::FunctionCall { name, arguments, span } => {
                // Check call stack depth
                if self.call_stack.len() >= self.max_call_depth {
                    return Err(format!("Maximum call depth exceeded: {}", self.max_call_depth));
                }
                
                let mut arg_values = Vec::new();
                for arg in arguments {
                    arg_values.push(self.evaluate_expression(arg)?);
                }
                
                self.call_stack.push(name.clone());
                let result = self.call_function(name, arg_values, span);
                self.call_stack.pop();
                result
            }
            
            Expression::Array(elements, _) => {
                let mut values = Vec::new();
                for elem in elements {
                    values.push(self.evaluate_expression(elem)?);
                }
                Ok(Value::Array(values))
            }
            
            Expression::Object(properties, _) => {
                let mut map = HashMap::new();
                for (key, value_expr) in properties {
                    let value = self.evaluate_expression(value_expr)?;
                    map.insert(key.clone(), value);
                }
                Ok(Value::Object(map))
            }
            
            Expression::PropertyAccess { object, property, span } => {
                let obj_val = self.evaluate_expression(object)?;
                match obj_val {
                    Value::Object(map) => {
                        map.get(property)
                            .cloned()
                            .ok_or_else(|| format!("Property '{}' not found at line {}", property, span.line))
                    }
                    Value::String(s) if property == "len" => {
                        Ok(Value::Int(s.len() as i64))
                    }
                    Value::Array(arr) if property == "len" => {
                        Ok(Value::Int(arr.len() as i64))
                    }
                    _ => Err(format!("Cannot access property '{}' on non-object at line {}", property, span.line))
                }
            }
            
            Expression::MethodCall { object, method, arguments, span } => {
                let obj_val = self.evaluate_expression(object)?;
                
                // Handle string methods
                if let Value::String(s) = &obj_val {
                    match method.as_str() {
                        "len" => {
                            if !arguments.is_empty() {
                                return Err(format!("len() takes no arguments at line {}", span.line));
                            }
                            Ok(Value::Int(s.len() as i64))
                        }
                        "to_upper" => {
                            if !arguments.is_empty() {
                                return Err(format!("to_upper() takes no arguments at line {}", span.line));
                            }
                            Ok(Value::String(s.to_uppercase()))
                        }
                        "to_lower" => {
                            if !arguments.is_empty() {
                                return Err(format!("to_lower() takes no arguments at line {}", span.line));
                            }
                            Ok(Value::String(s.to_lowercase()))
                        }
                        "trim" => {
                            if !arguments.is_empty() {
                                return Err(format!("trim() takes no arguments at line {}", span.line));
                            }
                            Ok(Value::String(s.trim().to_string()))
                        }
                        "contains" => {
                            if arguments.len() != 1 {
                                return Err(format!("contains() expects 1 argument at line {}", span.line));
                            }
                            let arg = self.evaluate_expression(&arguments[0])?;
                            let substr = arg.to_string();
                            Ok(Value::Bool(s.contains(&substr)))
                        }
                        _ => Err(format!("Unknown method '{}' on string at line {}", method, span.line))
                    }
                }
                // Handle array methods
                else if let Value::Array(arr) = &obj_val {
                    match method.as_str() {
                        "len" => {
                            if !arguments.is_empty() {
                                return Err(format!("len() takes no arguments at line {}", span.line));
                            }
                            Ok(Value::Int(arr.len() as i64))
                        }
                        "push" => {
                            if arguments.len() != 1 {
                                return Err(format!("push() expects 1 argument at line {}", span.line));
                            }
                            let arg = self.evaluate_expression(&arguments[0])?;
                            let mut new_arr = arr.clone();
                            new_arr.push(arg);
                            Ok(Value::Array(new_arr))
                        }
                        "pop" => {
                            if !arguments.is_empty() {
                                return Err(format!("pop() takes no arguments at line {}", span.line));
                            }
                            let mut new_arr = arr.clone();
                            if let Some(_last) = new_arr.pop() {
                                Ok(Value::Array(new_arr))
                            } else {
                                Ok(Value::Array(new_arr))
                            }
                        }
                        "get" => {
                            if arguments.len() != 1 {
                                return Err(format!("get() expects 1 argument at line {}", span.line));
                            }
                            let arg = self.evaluate_expression(&arguments[0])?;
                            match arg {
                                Value::Int(i) => {
                                    let idx = i as usize;
                                    if idx < arr.len() {
                                        Ok(arr[idx].clone())
                                    } else {
                                        Err(format!("Index {} out of bounds at line {}", idx, span.line))
                                    }
                                }
                                _ => Err(format!("get() expects an integer index at line {}", span.line))
                            }
                        }
                        _ => Err(format!("Unknown method '{}' on array at line {}", method, span.line))
                    }
                }
                // Handle object methods
                else if let Value::Object(map) = &obj_val {
                    match method.as_str() {
                        "get" => {
                            if arguments.len() != 1 {
                                return Err(format!("get() expects 1 argument at line {}", span.line));
                            }
                            let arg = self.evaluate_expression(&arguments[0])?;
                            let key = arg.to_string();
                            map.get(&key)
                                .cloned()
                                .ok_or_else(|| format!("Key '{}' not found at line {}", key, span.line))
                        }
                        "keys" => {
                            if !arguments.is_empty() {
                                return Err(format!("keys() takes no arguments at line {}", span.line));
                            }
                            let keys: Vec<Value> = map.keys().map(|k| Value::String(k.clone())).collect();
                            Ok(Value::Array(keys))
                        }
                        "values" => {
                            if !arguments.is_empty() {
                                return Err(format!("values() takes no arguments at line {}", span.line));
                            }
                            let values: Vec<Value> = map.values().cloned().collect();
                            Ok(Value::Array(values))
                        }
                        _ => Err(format!("Unknown method '{}' on object at line {}", method, span.line))
                    }
                }
                // Handle sLLM extension methods
                else {
                    if let Expression::Extension { name, .. } = object.as_ref() {
                        if name == "sllm" {
                            match method.as_str() {
                                "from_text_file" => {
                                    if arguments.len() != 1 {
                                        return Err("from_text_file() expects 1 argument".to_string());
                                    }
                                    let arg = self.evaluate_expression(&arguments[0])?;
                                    let path = arg.to_string();
                                    Ok(Value::String(format!("Dataset from file: {}", path)))
                                }
                                "from_directory" => {
                                    if arguments.len() != 1 {
                                        return Err("from_directory() expects 1 argument".to_string());
                                    }
                                    let arg = self.evaluate_expression(&arguments[0])?;
                                    let path = arg.to_string();
                                    Ok(Value::String(format!("Dataset from directory: {}", path)))
                                }
                                _ => Err(format!("Unknown sLLM method: {}", method))
                            }
                        } else {
                            Err(format!("Method '{}' not supported on this type at line {}", method, span.line))
                        }
                    } else {
                        Err(format!("Method '{}' not supported on this type at line {}", method, span.line))
                    }
                }
            }
            
            Expression::Import { from: _, items: _, .. } => {
                Ok(Value::Null)
            }
            
            Expression::Make { name, properties, children, .. } => {
                let mut props = HashMap::new();
                for (key, expr) in properties {
                    let val = self.evaluate_expression(expr)?;
                    props.insert(key.clone(), val);
                }
                let mut child_vals = Vec::new();
                for child in children {
                    child_vals.push(self.evaluate_expression(child)?);
                }
                Ok(Value::String(format!("Make<{}> with {} props and {} children", name, props.len(), child_vals.len())))
            }
            
            Expression::Render { target, .. } => {
                let target_val = self.evaluate_expression(target)?;
                Ok(Value::String(format!("Render<{}>", target_val.to_string())))
            }
            
            Expression::UiComponent { .. } => {
                Ok(Value::String("UI Component".to_string()))
            }
            Expression::UiRender { .. } => {
                Ok(Value::Null)
            }
            Expression::Extension { name, type_name, .. } => {
                Ok(Value::String(format!("Extension: {} ({})", name, type_name)))
            }
        }
    }

    fn evaluate_literal(&self, literal: &LiteralValue) -> Result<Value, String> {
        match literal {
            LiteralValue::Number(n) => {
                if n.fract() == 0.0 && *n <= i64::MAX as f64 && *n >= i64::MIN as f64 {
                    Ok(Value::Int(*n as i64))
                } else {
                    Ok(Value::Float(*n))
                }
            }
            LiteralValue::String(s) => Ok(Value::String(s.clone())),
            LiteralValue::Boolean(b) => Ok(Value::Bool(*b)),
            LiteralValue::Null => Ok(Value::Null),
        }
    }

    fn evaluate_binary_operation(&self, left: &Value, operator: &BinaryOperator, right: &Value) -> Result<Value, String> {
        match (left, operator, right) {
            // Integer arithmetic operations
            (Value::Int(l), BinaryOperator::Add, Value::Int(r)) => Ok(Value::Int(l + r)),
            (Value::Int(l), BinaryOperator::Subtract, Value::Int(r)) => Ok(Value::Int(l - r)),
            (Value::Int(l), BinaryOperator::Multiply, Value::Int(r)) => Ok(Value::Int(l * r)),
            (Value::Int(l), BinaryOperator::Divide, Value::Int(r)) => {
                if *r == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Int(l / r))
                }
            }
            (Value::Int(l), BinaryOperator::Modulo, Value::Int(r)) => {
                if *r == 0 {
                    Err("Modulo by zero".to_string())
                } else {
                    Ok(Value::Int(l % r))
                }
            }
            
            // Float arithmetic operations
            (Value::Float(l), BinaryOperator::Add, Value::Float(r)) => Ok(Value::Float(l + r)),
            (Value::Float(l), BinaryOperator::Subtract, Value::Float(r)) => Ok(Value::Float(l - r)),
            (Value::Float(l), BinaryOperator::Multiply, Value::Float(r)) => Ok(Value::Float(l * r)),
            (Value::Float(l), BinaryOperator::Divide, Value::Float(r)) => {
                if *r == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float(l / r))
                }
            }
            
            // Mixed numeric operations
            (Value::Int(l), BinaryOperator::Add, Value::Float(r)) => Ok(Value::Float((*l as f64) + r)),
            (Value::Float(l), BinaryOperator::Add, Value::Int(r)) => Ok(Value::Float(l + (*r as f64))),
            (Value::Int(l), BinaryOperator::Subtract, Value::Float(r)) => Ok(Value::Float((*l as f64) - r)),
            (Value::Float(l), BinaryOperator::Subtract, Value::Int(r)) => Ok(Value::Float(l - (*r as f64))),
            (Value::Int(l), BinaryOperator::Multiply, Value::Float(r)) => Ok(Value::Float((*l as f64) * r)),
            (Value::Float(l), BinaryOperator::Multiply, Value::Int(r)) => Ok(Value::Float(l * (*r as f64))),
            (Value::Int(l), BinaryOperator::Divide, Value::Float(r)) => {
                if *r == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float((*l as f64) / r))
                }
            }
            (Value::Float(l), BinaryOperator::Divide, Value::Int(r)) => {
                if *r == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float(l / (*r as f64)))
                }
            }
            
            // String concatenation
            (Value::String(l), BinaryOperator::Add, Value::String(r)) => Ok(Value::String(l.to_owned() + r)),
            (Value::String(l), BinaryOperator::Add, Value::Int(r)) => Ok(Value::String(l.to_owned() + &r.to_string())),
            (Value::Int(l), BinaryOperator::Add, Value::String(r)) => Ok(Value::String(l.to_string() + r)),
            (Value::String(l), BinaryOperator::Add, Value::Float(r)) => Ok(Value::String(l.to_owned() + &r.to_string())),
            (Value::Float(l), BinaryOperator::Add, Value::String(r)) => Ok(Value::String(l.to_string() + r)),
            
            // Equality comparisons
            (Value::Int(l), BinaryOperator::Equal, Value::Int(r)) => Ok(Value::Bool(l == r)),
            (Value::Float(l), BinaryOperator::Equal, Value::Float(r)) => Ok(Value::Bool(l == r)),
            (Value::String(l), BinaryOperator::Equal, Value::String(r)) => Ok(Value::Bool(l == r)),
            (Value::Bool(l), BinaryOperator::Equal, Value::Bool(r)) => Ok(Value::Bool(l == r)),
            (Value::Null, BinaryOperator::Equal, Value::Null) => Ok(Value::Bool(true)),
            (Value::Array(l), BinaryOperator::Equal, Value::Array(r)) => Ok(Value::Bool(l == r)),
            (Value::Object(l), BinaryOperator::Equal, Value::Object(r)) => Ok(Value::Bool(l == r)),
            
            // Inequality comparisons
            (Value::Int(l), BinaryOperator::NotEqual, Value::Int(r)) => Ok(Value::Bool(l != r)),
            (Value::Float(l), BinaryOperator::NotEqual, Value::Float(r)) => Ok(Value::Bool(l != r)),
            (Value::String(l), BinaryOperator::NotEqual, Value::String(r)) => Ok(Value::Bool(l != r)),
            (Value::Bool(l), BinaryOperator::NotEqual, Value::Bool(r)) => Ok(Value::Bool(l != r)),
            (Value::Null, BinaryOperator::NotEqual, Value::Null) => Ok(Value::Bool(false)),
            
            // Numeric comparisons (int vs int)
            (Value::Int(l), BinaryOperator::LessThan, Value::Int(r)) => Ok(Value::Bool(l < r)),
            (Value::Int(l), BinaryOperator::LessThanOrEqual, Value::Int(r)) => Ok(Value::Bool(l <= r)),
            (Value::Int(l), BinaryOperator::GreaterThan, Value::Int(r)) => Ok(Value::Bool(l > r)),
            (Value::Int(l), BinaryOperator::GreaterThanOrEqual, Value::Int(r)) => Ok(Value::Bool(l >= r)),
            
            // Numeric comparisons (float vs float)
            (Value::Float(l), BinaryOperator::LessThan, Value::Float(r)) => Ok(Value::Bool(l < r)),
            (Value::Float(l), BinaryOperator::LessThanOrEqual, Value::Float(r)) => Ok(Value::Bool(l <= r)),
            (Value::Float(l), BinaryOperator::GreaterThan, Value::Float(r)) => Ok(Value::Bool(l > r)),
            (Value::Float(l), BinaryOperator::GreaterThanOrEqual, Value::Float(r)) => Ok(Value::Bool(l >= r)),
            
            // Numeric comparisons (mixed)
            (Value::Int(l), BinaryOperator::LessThan, Value::Float(r)) => Ok(Value::Bool((*l as f64) < *r)),
            (Value::Int(l), BinaryOperator::LessThanOrEqual, Value::Float(r)) => Ok(Value::Bool((*l as f64) <= *r)),
            (Value::Int(l), BinaryOperator::GreaterThan, Value::Float(r)) => Ok(Value::Bool((*l as f64) > *r)),
            (Value::Int(l), BinaryOperator::GreaterThanOrEqual, Value::Float(r)) => Ok(Value::Bool((*l as f64) >= *r)),
            (Value::Float(l), BinaryOperator::LessThan, Value::Int(r)) => Ok(Value::Bool(*l < (*r as f64))),
            (Value::Float(l), BinaryOperator::LessThanOrEqual, Value::Int(r)) => Ok(Value::Bool(*l <= (*r as f64))),
            (Value::Float(l), BinaryOperator::GreaterThan, Value::Int(r)) => Ok(Value::Bool(*l > (*r as f64))),
            (Value::Float(l), BinaryOperator::GreaterThanOrEqual, Value::Int(r)) => Ok(Value::Bool(*l >= (*r as f64))),
            
            // Logical operators
            (Value::Bool(l), BinaryOperator::And, Value::Bool(r)) => Ok(Value::Bool(*l && *r)),
            (Value::Bool(l), BinaryOperator::Or, Value::Bool(r)) => Ok(Value::Bool(*l || *r)),
            
            _ => Err(format!("Unsupported operation: {:?} {:?} {:?}", left, operator, right)),
        }
    }

    fn call_function(&mut self, name: &str, arguments: Vec<Value>, span: &Span) -> Result<Value, String> {
        // Check native functions first - case insensitive
        for (key, native_func) in &self.native_functions {
            if key.eq_ignore_ascii_case(name) {
                return native_func(&arguments);
            }
        }

        // Check user-defined functions - case insensitive
        for (key, func) in &self.functions {
            if key.eq_ignore_ascii_case(name) {
                if func.parameters.len() != arguments.len() {
                    return Err(format!(
                        "Function '{}' expects {} arguments, got {} at line {}",
                        name,
                        func.parameters.len(),
                        arguments.len(),
                        span.line
                    ));
                }

                // Create new environment for function execution
                let mut func_env = Environment::new();
                
                // Set parameters
                for (i, param_name) in func.parameters.iter().enumerate() {
                    func_env.set(param_name, arguments[i].clone());
                }

                // Copy functions to function environment
                let mut func_interpreter = AstInterpreter::new();
                func_interpreter.environment = func_env;
                for (name, func) in &self.functions {
                    func_interpreter.functions.insert(name.clone(), func.clone());
                }
                
                // Execute function body and return the result
                let result = func_interpreter.execute_block(&func.body)?;
                return match result {
                    ControlFlow::Return(v) => Ok(v),
                    _ => Ok(Value::Null),
                };
            }
        }

        Err(format!("Function '{}' not found at line {}", name, span.line))
    }

    pub fn run(&mut self, tokens: &[crate::lexer::Token]) -> Result<(), String> {
        let source = tokens.iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>().join(" ");
        let mut parser = crate::parser::Parser::new(
            tokens.to_vec(), 
            source, 
            ExtensionRegistry::new()
        );
        let program = parser.parse()
            .map_err(|e| format!("Parse error: {}", e))?;
        
        self.interpret(&program)?;
        Ok(())
    }
}

impl Default for AstInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn interpret_source(source: &str) -> Result<Value, String> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().map_err(|e| e.to_string())?;
        let mut parser = crate::parser::Parser::new(
            tokens, 
            source.to_string(), 
            ExtensionRegistry::new()
        );
        let program = parser.parse().map_err(|e| e.to_string())?;
        let mut interpreter = AstInterpreter::new();
        interpreter.interpret(&program)
    }

    #[test]
    fn test_variable_assignment() {
        let source = r#"
            make x = 5
            make y = 10
            make z = x + y
        "#;
        let result = interpret_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_call() {
        let source = r#"
            func add<a, b> {
                return a + b
            }
            make result = add<5, 3>
        "#;
        let result = interpret_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_if_elif_else() {
        let source = r#"
            make score = 85
            if <score >= 90> {
                write("A")
            } elif <score >= 80> {
                write("B")
            } elif <score >= 70> {
                write("C")
            } else {
                write("F")
            }
        "#;
        let result = interpret_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_loop() {
        let source = r#"
            loop 5 {
                write_no_newline(".")
            }
        "#;
        let result = interpret_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_null_value() {
        let source = r#"
            make x = null
            if <x == null> {
                write("null works")
            }
        "#;
        let result = interpret_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_array() {
        let source = r#"
            make arr = [1, 2, 3, 4, 5]
            make len = len(arr)
        "#;
        let result = interpret_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_string_concatenation() {
        let source = r#"
            make greeting = "Hello" + " " + "World"
        "#;
        let result = interpret_source(source);
        assert!(result.is_ok());
    }
}