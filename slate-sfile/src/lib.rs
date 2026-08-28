// slate-sfile/src/lib.rs

use slate_core::extension::LanguageExtension;
use slate_core::ast::{Expression, Span, Statement, Program};
use slate_core::ast_interpreter::AstInterpreter;
use slate_core::value::Value;
use slate_core::lexer::Token;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub mod resolver;
pub mod loader;

use loader::FileLoader;
use resolver::PathResolver;

/// The sfile extension for importing SlateScript files
pub struct SFileExtension {
    loaded_files: RwLock<HashMap<PathBuf, Vec<Token>>>,
    file_cache: RwLock<HashMap<PathBuf, Program>>,
    resolver: PathResolver,
    loader: FileLoader,
    current_file: RwLock<Option<PathBuf>>,
    import_stack: RwLock<Vec<PathBuf>>,
    imported_functions: RwLock<HashMap<String, (String, Vec<String>, Vec<Statement>)>>,
}

impl SFileExtension {
    /// Get all loaded files
    pub fn get_loaded_files(&self) -> Vec<(PathBuf, Vec<Token>)> {
        self.loaded_files.read().unwrap()
            .iter()
            .map(|(path, tokens)| (path.clone(), tokens.clone()))
            .collect()
    }

    /// Get all imported functions (flattened)
    pub fn get_all_imported_functions(&self) -> Vec<(String, Vec<String>, Vec<Statement>)> {
        self.imported_functions.read().unwrap()
            .iter()
            .map(|(name, (source, params, body))| {
                (name.clone(), params.clone(), body.clone())
            })
            .collect()
    }

    pub fn new() -> Self {
        Self {
            loaded_files: RwLock::new(HashMap::new()),
            file_cache: RwLock::new(HashMap::new()),
            resolver: PathResolver::new(),
            loader: FileLoader::new(),
            current_file: RwLock::new(None),
            import_stack: RwLock::new(Vec::new()),
            imported_functions: RwLock::new(HashMap::new()),
        }
    }

    /// Set the current file being executed (for relative imports)
    pub fn set_current_file(&self, path: &Path) {
        *self.current_file.write().unwrap() = Some(path.to_path_buf());
    }

    /// Get the base directory for relative imports
    fn get_base_dir(&self) -> PathBuf {
        self.current_file.read().unwrap()
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
    }

    /// Resolve and load a file
    pub fn load_file(&self, path: &Path) -> Result<Program, String> {
        let base_dir = self.get_base_dir();
        let resolved = self.resolver.resolve(path, &base_dir)?;
        
        // Check for circular imports
        if self.import_stack.read().unwrap().contains(&resolved) {
            return Err(format!("Circular import detected: {}", resolved.display()));
        }
        
        // Check cache first
        if let Some(cached) = self.file_cache.read().unwrap().get(&resolved) {
            return Ok(cached.clone());
        }
        
        // Push to import stack
        self.import_stack.write().unwrap().push(resolved.clone());
        
        // Load and parse the file
        let source = self.loader.read_file(&resolved)?;
        let tokens = self.loader.tokenize(&source)?;
        let program = self.loader.parse(&tokens)?;
        
        // Cache it
        self.file_cache.write().unwrap().insert(resolved.clone(), program.clone());
        self.loaded_files.write().unwrap().insert(resolved.clone(), tokens);
        
        // Pop from import stack
        self.import_stack.write().unwrap().pop();
        
        Ok(program)
    }

    /// Import multiple files
    pub fn import_files(&self, file_paths: &[String]) -> Result<Vec<Statement>, String> {
        let mut all_statements = Vec::new();
        
        for path_str in file_paths {
            let path = PathBuf::from(path_str);
            let program = self.load_file(&path)?;
            
            // Extract functions from the program
            for stmt in &program.statements {
                if let Statement::FunctionDefinition { name, parameters, body, .. } = stmt {
                    // Store function in imported_functions
                    let mut imported = self.imported_functions.write().unwrap();
                    let source_file = path_str.clone();
                    imported.insert(name.clone(), (source_file, parameters.clone(), body.clone()));
                }
            }
            
            all_statements.extend(program.statements);
        }
        
        Ok(all_statements)
    }

    /// Get all imported functions from a file
    pub fn get_imported_functions(&self, file_path: &str) -> Vec<(String, Vec<String>, Vec<Statement>)> {
        self.imported_functions.read().unwrap()
            .iter()
            .filter(|(_, (source, _, _))| source == file_path)
            .map(|(name, (_, params, body))| (name.clone(), params.clone(), body.clone()))
            .collect()
    }

    /// Check if a function was imported
    pub fn has_imported_function(&self, name: &str) -> bool {
        self.imported_functions.read().unwrap().contains_key(name)
    }

    /// Get a specific imported function
    pub fn get_imported_function(&self, name: &str) -> Option<(String, Vec<String>, Vec<Statement>)> {
        self.imported_functions.read().unwrap()
            .get(name)
            .map(|(source, params, body)| (source.clone(), params.clone(), body.clone()))
    }
}

impl Clone for SFileExtension {
    fn clone(&self) -> Self {
        Self {
            loaded_files: RwLock::new(self.loaded_files.read().unwrap().clone()),
            file_cache: RwLock::new(self.file_cache.read().unwrap().clone()),
            resolver: self.resolver.clone(),
            loader: self.loader.clone(),
            current_file: RwLock::new(self.current_file.read().unwrap().clone()),
            import_stack: RwLock::new(Vec::new()),
            imported_functions: RwLock::new(self.imported_functions.read().unwrap().clone()),
        }
    }
}

impl LanguageExtension for SFileExtension {
    fn name(&self) -> &str {
        "sfile"
    }

    fn clone_box(&self) -> Box<dyn LanguageExtension> {
        Box::new(self.clone())
    }

    fn custom_tokens(&self) -> Vec<slate_core::extension::CustomToken> {
        vec![
            slate_core::extension::CustomToken { keyword: "import".to_string(), token: Token::Import },
            slate_core::extension::CustomToken { keyword: "from".to_string(), token: Token::From },
        ]
    }

    fn parse_extension(&self, tokens: &[Token], pos: usize) -> Option<(Expression, usize)> {
        // Check for: import from "sfile" {file1, file2, ...}
        if !matches!(tokens.get(pos), Some(Token::Import)) {
            return None;
        }
        
        let mut i = pos + 1;
        
        // Expect: from
        if !matches!(tokens.get(i), Some(Token::From)) {
            return None;
        }
        i += 1;
        
        // Expect: "sfile" (string - the extension name)
        match tokens.get(i) {
            Some(Token::String(s)) if s == "sfile" => {
                i += 1;
            }
            _ => return None,
        }
        
        // Expect: {
        if !matches!(tokens.get(i), Some(Token::LeftBrace)) {
            return None;
        }
        i += 1;
        
        // Parse file paths (strings inside braces)
        let mut file_paths = Vec::new();
        
        while i < tokens.len() && !matches!(tokens.get(i), Some(Token::RightBrace)) {
            match tokens.get(i) {
                Some(Token::String(path)) => {
                    file_paths.push(path.clone());
                    i += 1;
                }
                Some(Token::Comma) => {
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }
        
        // Expect: }
        if !matches!(tokens.get(i), Some(Token::RightBrace)) {
            return None;
        }
        i += 1;
        
        // Create the extension expression
        let span = Span::new(pos, i, 0, 0);
        let type_name = format!("Import:{}", file_paths.join("|"));
        
        let expr = Expression::Extension {
            name: "sfile".to_string(),
            type_name,
            span,
        };
        
        Some((expr, i))
    }

    fn interpret_extension(&self, expr: &Expression, interpreter: &mut AstInterpreter) -> Result<Value, String> {
        match expr {
            Expression::Extension { name, type_name, .. } if name == "sfile" => {
                // Parse the type_name to get file paths
                let parts: Vec<&str> = type_name.split(':').collect();
                if parts.len() >= 2 && parts[0] == "Import" {
                    let file_paths: Vec<String> = parts[1]
                        .split('|')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect();
                    
                    // Import the files
                    let statements = self.import_files(&file_paths)?;
                    
                    // Register functions in the interpreter
                    let mut registered_count = 0;
                    for stmt in &statements {
                        if let Statement::FunctionDefinition { name, parameters, body, .. } = stmt {
                            if !interpreter.has_function(name) {
                                interpreter.register_function(name, parameters.clone(), body.clone());
                                registered_count += 1;
                                println!("[sfile] Registered function: {}", name);
                            } else {
                                println!("[sfile] Function '{}' already exists, skipping", name);
                            }
                        }
                    }
                    
                    Ok(Value::String(format!(
                        "Imported {} files, registered {} functions",
                        file_paths.len(), registered_count
                    )))
                } else {
                    Err(format!("Unknown sfile expression: {}", type_name))
                }
            }
            _ => Err("Not an sfile expression".to_string()),
        }
    }

    fn handles_expression(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::Extension { name, .. } if name == "sfile")
    }

    fn post_process_ast(&mut self, program: &mut Program) -> Result<(), String> {
        // Process all import statements in the program
        let mut imports_to_process = Vec::new();
        
        // Collect all import statements first (to avoid borrow issues)
        for stmt in &program.statements {
            if let Statement::Expression(Expression::Extension { name, type_name, .. }) = stmt {
                if name == "sfile" && type_name.starts_with("Import:") {
                    let parts: Vec<&str> = type_name.split(':').collect();
                    if parts.len() >= 2 {
                        let file_paths: Vec<String> = parts[1]
                            .split('|')
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .collect();
                        imports_to_process.push(file_paths);
                    }
                }
            }
        }
        
        // Process imports
        for file_paths in imports_to_process {
            for path_str in &file_paths {
                let path = Path::new(path_str);
                let _ = self.load_file(path)?;
            }
        }
        
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Default for SFileExtension {
    fn default() -> Self {
        Self::new()
    }
}