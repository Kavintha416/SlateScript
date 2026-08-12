//
// src/debug.rs
// SlateScript Debugger - Expressive Error Messages & Diagnostics
// ASCII-only version
//

use crate::lexer::Token;

/// A diagnostic message with context and suggestions
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub context: String,
    pub suggestion: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Hint,
}

/// Smart error messages with suggestions
pub struct DiagnosticsEngine;

impl DiagnosticsEngine {
    /// Create a helpful error message for unexpected tokens
    pub fn unexpected_token(
        token: &Token,
        pos: usize,
        source: &str,
        expected: &str,
        context: &str,
    ) -> Diagnostic {
        let (line, column) = Self::get_position_info(source, pos);
        let token_str = Self::token_description(token);
        let nearby = Self::get_nearby_code(source, pos);
        
        let hint = Self::generate_suggestion(token, expected, context);
        
        Diagnostic {
            severity: Severity::Error,
            message: format!("Expected {} but found {}", expected, token_str),
            context: format!("At: {}\n\n{}", context, nearby),
            suggestion: hint,
            line,
            column,
        }
    }
    
    /// Generate specific hints based on the error context
    fn generate_suggestion(token: &Token, expected: &str, context: &str) -> String {
        let mut hint = String::new();
        
        // Function definition context
        if context.contains("function") {
            match token {
                Token::Identifier(name) if expected.contains("'func'") => {
                    hint = format!(
                        "Did you mean to write 'func {}<> {{ ... }}'?\n\
                         You wrote '{}' but 'func' starts a function definition.",
                        name, name
                    );
                }
                Token::LeftBrace => {
                    hint = format!(
                        "Missing '<>' after function name?\n\
                         Functions must be declared as: func name<> {{ body }}"
                    );
                }
                Token::Func => {
                    hint = format!(
                        "Good start! Now add a function name and '<>'\n\
                         Example: func greet<> {{ write(\"Hello\") }}"
                    );
                }
                _ => {}
            }
        }
        
        // Function call context
        if context.contains("call") {
            match token {
                Token::Identifier(name) if expected.contains("'<>") => {
                    hint = format!(
                        "To call a function, use: {}<>\n\
                         You wrote '{}' but function calls need '<>' after the name.",
                        name, name
                    );
                }
                _ => {}
            }
        }
        
        // String/write context
        if context.contains("write") {
            match token {
                Token::Identifier(word) => {
                    hint = format!(
                        "Strings must be in quotes.\n\
                         Did you mean: write(\"{}\")?", word
                    );
                }
                Token::Func => {
                    hint = format!(
                        "Cannot define a function inside write()\n\
                         write() only accepts string literals: write(\"hello\")"
                    );
                }
                _ => {}
            }
        }
        
        // Brace context
        if expected.contains("'{'") {
            hint = format!(
                "Function bodies must be wrapped in braces {{ }}\n\
                 Add '{{' before the function body"
            );
        }
        
        if expected.contains("'}'") {
            hint = format!(
                "Missing closing brace?\n\
                 Every '{{' needs a matching '}}'"
            );
        }
        
        if hint.is_empty() {
            hint = format!(
                "Unexpected {}. Expected {} in {} context.",
                Self::token_description(token),
                expected,
                context
            );
        }
        
        hint
    }
    
    /// Describe a token in user-friendly terms
    fn token_description(token: &Token) -> String {
        match token {
            Token::Func => "'func' keyword".to_string(),
            Token::Write => "'write' keyword".to_string(),
            Token::Make => "'make' keyword".to_string(),
            Token::With => "'with' keyword".to_string(),
            Token::Identifier(name) => format!("identifier '{}'", name),
            Token::String(s) => format!("string \"{}\"", s),
            Token::Number(n) => format!("number {}", n),
            Token::Float(f) => format!("float {}", f),
            Token::True => "'true'".to_string(),
            Token::False => "'false'".to_string(),
            Token::LeftParen => "'('".to_string(),
            Token::RightParen => "')'".to_string(),
            Token::LeftBracket => "'['".to_string(),
            Token::RightBracket => "']'".to_string(),
            Token::LeftBrace => "'{'".to_string(),
            Token::RightBrace => "'}'".to_string(),
            Token::LessThan => "'<'".to_string(),
            Token::GreaterThan => "'>'".to_string(),
            Token::Semicolon => "';'".to_string(),
            Token::Equal => "'='".to_string(),
            Token::Plus => "'+'".to_string(),
            Token::Minus => "'-'".to_string(),
            Token::Multiply => "'*'".to_string(),
            Token::Divide => "'/'".to_string(),
            Token::Colon => "':'".to_string(),
            Token::Int => "'int' type".to_string(),
            Token::StringType => "'string' type".to_string(),
            Token::Bool => "'bool' type".to_string(),
            Token::If => "'if' keyword".to_string(),
            Token::Elif => "'elif' keyword".to_string(),
            Token::Else => "'else' keyword".to_string(),
            Token::Loop => "'loop' keyword".to_string(),
            Token::EqualEqual => "'=='".to_string(),
            Token::NotEqual => "'!='".to_string(),
            Token::LessEqual => "'<='".to_string(),
            Token::GreaterEqual => "'>='".to_string(),
            Token::Comma => "','".to_string(),
            Token::Dot => "'.'".to_string(),
            Token::Import => "'import' keyword".to_string(),
            Token::From => "'from' keyword".to_string(),
            Token::EOF => "end of file".to_string(),
            Token::Window => "'Window' component".to_string(),
            Token::Column => "'Column' component".to_string(),
            Token::Row => "'Row' component".to_string(),
            Token::Text => "'Text' component".to_string(),
            Token::Button => "'Button' component".to_string(),
            Token::Input => "'Input' component".to_string(),
            Token::Parent => "'Parent' keyword".to_string(),
            Token::Child => "'Child' keyword".to_string(),
            Token::Slash => "'/'".to_string(),
            Token::OnTap => "'on_tap' event".to_string(),
            Token::OnClick => "'on_click' event".to_string(),
            Token::OnChange => "'on_change' event".to_string(),
            Token::OnInput => "'on_input' event".to_string(),
            Token::Render => "'render' keyword".to_string(),
            Token::Identity => "'Identity' keyword".to_string(),
            Token::Rewrite => "'Rewrite' keyword".to_string(),
        }
    }
    
    /// Get line and column from position in source
    fn get_position_info(source: &str, pos: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        
        for (i, ch) in source.chars().enumerate() {
            if i >= pos {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        
        (line, col)
    }
    
    /// Get nearby code with an arrow pointing to the error
    fn get_nearby_code(source: &str, pos: usize) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let (line_num, _) = Self::get_position_info(source, pos);
        
        let mut result = String::new();
        let start = line_num.saturating_sub(2);
        let end = (line_num + 1).min(lines.len());
        
        for i in start..end {
            let line_content = lines.get(i).unwrap_or(&"");
            let prefix = if i + 1 == line_num {
                "--> "
            } else {
                "    "
            };
            result.push_str(&format!("{}{:3} | {}\n", prefix, i + 1, line_content));
            
            if i + 1 == line_num {
                result.push_str(&format!("      | {}^ here\n", "~".repeat(30)));
            }
        }
        
        result
    }
    
    /// Print a diagnostic in a beautiful format
    pub fn print(diagnostic: &Diagnostic) {
        let (label, color_code) = match diagnostic.severity {
            Severity::Error => ("[ERROR]", "\x1b[31m"),  // Red
            Severity::Warning => ("[WARNING]", "\x1b[33m"), // Yellow
            Severity::Hint => ("[HINT]", "\x1b[36m"),   // Cyan
        };
        let reset = "\x1b[0m";
        
        println!();
        println!("{}+----------------------------------------{}" , color_code, reset);
        println!("{}| {} {} at line {}, column {}{}", 
            color_code, label, diagnostic.message, diagnostic.line, diagnostic.column, reset);
        println!("{}+----------------------------------------{}" , color_code, reset);
        
        for line in diagnostic.context.lines() {
            println!("{}|{} {}", color_code, reset, line);
        }
        
        println!("{}+----------------------------------------{}" , color_code, reset);
        println!("{}| -> {}", color_code, diagnostic.suggestion);
        println!("{}+----------------------------------------{}" , color_code, reset);
        println!();
    }
}

/// Runtime debugging support
pub struct Debugger {
    pub enabled: bool,
    pub break_on_function_calls: bool,
    pub trace_execution: bool,
    call_stack: Vec<String>,
}

impl Debugger {
    pub fn new() -> Self {
        Debugger {
            enabled: false,
            break_on_function_calls: false,
            trace_execution: false,
            call_stack: Vec::new(),
        }
    }
    
    /// Enable with flags from environment
    pub fn from_env() -> Self {
        let enabled = std::env::var("SLATE_DEBUG").is_ok();
        let trace = std::env::var("SLATE_TRACE").is_ok();
        
        Debugger {
            enabled,
            break_on_function_calls: enabled,
            trace_execution: trace,
            call_stack: Vec::new(),
        }
    }
    
    /// Trace a function call
    pub fn trace_call(&mut self, name: &str) {
        if self.trace_execution {
            let indent = "  ".repeat(self.call_stack.len());
            println!("{}-> calling {}", indent, name);
        }
        self.call_stack.push(name.to_string());
    }
    
    /// Trace function return
    pub fn trace_return(&mut self) {
        if let Some(name) = self.call_stack.pop() {
            if self.trace_execution {
                let indent = "  ".repeat(self.call_stack.len());
                println!("{}<- {} returned", indent, name);
            }
        }
    }
    
    /// Print current call stack
    pub fn print_backtrace(&self) {
        if self.call_stack.is_empty() {
            println!("  (empty call stack)");
            return;
        }
        
        println!("\n  Call Stack:");
        for (i, func) in self.call_stack.iter().enumerate() {
            let arrow = if i == self.call_stack.len() - 1 {
                ">"
            } else {
                " "
            };
            println!("    {} {}: {}", arrow, i + 1, func);
        }
    }
    
    /// Print execution location
    pub fn print_location(&self, tokens: &[Token], pos: usize) {
        if pos < tokens.len() {
            println!("\n  Current token: {:?}", tokens[pos]);
            if pos + 1 < tokens.len() {
                println!("  Next token: {:?}", tokens[pos + 1]);
            }
        }
    }
}

/// Helper function to check common mistakes and print hints
/// Returns (diagnostics, has_errors)
pub fn check_common_mistakes(source: &str) -> (Vec<Diagnostic>, bool) {
    let mut diagnostics = Vec::new();
    let mut has_errors = false;
    let lines: Vec<&str> = source.lines().collect();
    
    // Check global brace balance
    let mut total_open = 0;
    let mut total_close = 0;
    let mut last_open_line = 0;
    
    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1;
        
        for ch in line.chars() {
            if ch == '{' {
                total_open += 1;
                last_open_line = line_num;
            } else if ch == '}' {
                total_close += 1;
            }
        }
        
        // Check for missing <> in function definitions on single line
        if line.contains("func ") && !line.contains("<") && line.contains("{") {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: "Missing '<>' in function definition".to_string(),
                context: format!("Line {}: {}", line_num, line),
                suggestion: "Functions must be: func name<> { body }".to_string(),
                line: line_num,
                column: 1,
            });
            has_errors = true;
        }
    }
    
    // Report global brace imbalance as error
    if total_open > total_close {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: format!("Missing {} closing brace(s) '}}'", total_open - total_close),
            context: format!("Found {} '{{' but only {} '}}'", total_open, total_close),
            suggestion: "Every opening brace '{{' must have a matching closing brace '}}'".to_string(),
            line: last_open_line,
            column: 1,
        });
        has_errors = true;
    } else if total_close > total_open {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: format!("Too many closing brace(s) '}}' ({} extra)", total_close - total_open),
            context: format!("Found {} '}}' but only {} '{{'", total_close, total_open),
            suggestion: "Remove extra closing braces or add missing opening braces".to_string(),
            line: lines.len(),
            column: 1,
        });
        has_errors = true;
    }
    
    (diagnostics, has_errors)
}

/// Enhanced panic handler with suggestions
pub fn smart_panic(message: &str, source: &str, pos: usize) -> ! {
    eprintln!("\n");
    eprintln!("+----------------------------------------");
    eprintln!("| SLATESCRIPT ERROR");
    eprintln!("+----------------------------------------");
    eprintln!("| {}", message);
    eprintln!("+----------------------------------------");
    eprintln!("");
    
    // Show location in code
    let lines: Vec<&str> = source.lines().collect();
    let _current_pos = 0;
    let mut line_num: usize = 1;
    let mut col: usize = 1;
    
    for (i, ch) in source.chars().enumerate() {
        if i == pos {
            break;
        }
        if ch == '\n' {
            line_num += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    
    let start = line_num.saturating_sub(2);
    let end = (line_num + 1).min(lines.len());
    
    eprintln!("  Location:");
    for i in start..end {
        if let Some(line) = lines.get(i) {
            let prefix = if i + 1 == line_num { "-->" } else { "   " };
            eprintln!("  {} {:3} | {}", prefix, i + 1, line);
            if i + 1 == line_num {
                eprintln!("      | {}^", "~".repeat(col.saturating_sub(1)));
            }
        }
    }
    
    eprintln!("\n");
    eprintln!("  Tip: Run with SLATE_DEBUG=1 for detailed trace");
    eprintln!("       Run with SLATE_TRACE=1 to see execution flow");
    eprintln!("");
    
    panic!("{}", message);
}
