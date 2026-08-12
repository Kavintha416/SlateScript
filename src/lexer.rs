//
// src/lexer.rs
//

#[derive(Debug, Clone, PartialEq)]
pub struct LexerError {
    pub message: String,
    pub position: usize,
    pub line: usize,
}

impl LexerError {
    pub fn new(message: &str, position: usize, line: usize) -> Self {
        Self {
            message: message.to_string(),
            position,
            line,
        }
    }
}

impl std::fmt::Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lexer error at line {} (pos {}): {}", self.line, self.position, self.message)
    }
}

impl std::error::Error for LexerError {}

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Write,
    String(String),
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Semicolon,
    EOF,
    Func,
    Identifier(String),
    LessThan,
    GreaterThan,
    LeftBrace,
    RightBrace,
    Make,           
    Equal,          
    Plus,           
    Minus,          
    Multiply,       
    Divide,         
    Colon,         
    Number(i64),
    Float(f64),
    Int, StringType, Bool,
    True, False,
    If,
    Elif,
    Else,
    Loop, 
    With,         
    EqualEqual,    
    NotEqual,      
    LessEqual,     
    GreaterEqual,
    Comma,
    Dot,
    Import,
    From,
    // UI Component keywords
    Window,
    Column,
    Row,
    Text,
    Button,
    Input,
    // UI Relationship keywords
    Parent,
    Child,
    Slash,
    // UI Event handlers
    OnTap,
    OnClick,
    OnChange,
    OnInput,
    // UI Rendering
    Render,
    // UI Identity
    Identity,
    Rewrite,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
        }
    }
    
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        
        while self.pos < self.chars.len() {
            let ch = self.chars[self.pos];
            
            // Track line numbers
            if ch == '\n' {
                self.line += 1;
            }
            
            match ch {
                '<' => {
                    self.pos += 1;
                    if self.pos < self.chars.len() && self.chars[self.pos] == '=' {
                        tokens.push(Token::LessEqual);
                        self.pos += 1;
                    } else {
                        tokens.push(Token::LessThan);
                    }
                }
                '>' => {
                    self.pos += 1;
                    if self.pos < self.chars.len() && self.chars[self.pos] == '=' {
                        tokens.push(Token::GreaterEqual);
                        self.pos += 1;
                    } else {
                        tokens.push(Token::GreaterThan);
                    }
                }
                '{' => {
                    tokens.push(Token::LeftBrace);
                    self.pos += 1;
                }
                '}' => {
                    tokens.push(Token::RightBrace);
                    self.pos += 1;
                }
                '[' => {
                    tokens.push(Token::LeftBracket);
                    self.pos += 1;
                }
                ']' => {
                    tokens.push(Token::RightBracket);
                    self.pos += 1;
                }
                // Skip whitespace
                ' ' | '\n' | '\t' | '\r' => {
                    self.pos += 1;
                }
                
                '(' => {
                    tokens.push(Token::LeftParen);
                    self.pos += 1;
                }
                
                ')' => {
                    tokens.push(Token::RightParen);
                    self.pos += 1;
                }
                
                ';' => {
                    tokens.push(Token::Semicolon);
                    self.pos += 1;
                }

                '=' => {
                    self.pos += 1;
                    if self.pos < self.chars.len() && self.chars[self.pos] == '=' {
                        tokens.push(Token::EqualEqual);
                        self.pos += 1;
                    } else {
                        tokens.push(Token::Equal);
                    }
                }

                '+' => {
                    tokens.push(Token::Plus);
                    self.pos += 1;
                }

                '-' if self.pos + 1 < self.chars.len() 
                      && self.chars[self.pos + 1] == '-' => {
                    self.pos += 2; // skip both dashes
                    while self.pos < self.chars.len() 
                          && self.chars[self.pos] != '\n' {
                        self.pos += 1;
                    }
                }

                '-' => {
                    tokens.push(Token::Minus);
                    self.pos += 1;
                }

                '*' => {
                    tokens.push(Token::Multiply);
                    self.pos += 1;
                }

                '/' => {
                    tokens.push(Token::Divide);
                    self.pos += 1;
                }

                ':' => {
                    tokens.push(Token::Colon);
                    self.pos += 1;
                }

                ',' => {
                    tokens.push(Token::Comma);
                    self.pos += 1;
                }

                '.' => {
                    tokens.push(Token::Dot);
                    self.pos += 1;
                }

                '!' => {
                    self.pos += 1;
                    if self.pos < self.chars.len() && self.chars[self.pos] == '=' {
                        tokens.push(Token::NotEqual);
                        self.pos += 1;
                    } else {
                        return Err(LexerError::new("Expected '=' after '!', found invalid token", self.pos, self.line));
                    }
                }

                // Numbers (integers and floats)
                c if c.is_digit(10) => {
                    let mut num = String::new();
                    let mut is_float = false;
                    
                    while self.pos < self.chars.len() && 
                          (self.chars[self.pos].is_digit(10) || self.chars[self.pos] == '.') {
                        if self.chars[self.pos] == '.' {
                            if is_float {
                                return Err(LexerError::new("Invalid number: multiple decimal points", self.pos, self.line));
                            }
                            is_float = true;
                        }
                        num.push(self.chars[self.pos]);
                        self.pos += 1;
                    }
                    
                    if is_float {
                        tokens.push(Token::Float(num.parse().unwrap()));
                    } else {
                        tokens.push(Token::Number(num.parse().unwrap()));
                    }
                }
                
                // String literals
                '"' => {
                    self.pos += 1; // skip opening quote
                    let mut content = String::new();
                    
                    while self.pos < self.chars.len() 
                          && self.chars[self.pos] != '"' {
                        content.push(self.chars[self.pos]);
                        self.pos += 1;
                    }
                    
                    if self.pos < self.chars.len() {
                        self.pos += 1; // skip closing quote
                    } else {
                        return Err(LexerError::new("Unterminated string: missing closing quote", self.pos, self.line));
                    }
                    
                    tokens.push(Token::String(content));
                }
                
                // Keywords and identifiers
                c if c.is_alphabetic() || c == '_' => {
                    let mut word = String::new();
                    
                    while self.pos < self.chars.len() 
                          && (self.chars[self.pos].is_alphabetic() 
                              || self.chars[self.pos] == '_' 
                              || self.chars[self.pos].is_numeric()) {
                        word.push(self.chars[self.pos]);
                        self.pos += 1;
                    }
                    
                    match word.as_str() {
                        "import" => tokens.push(Token::Import),
                        "from" => tokens.push(Token::From),
                        "write" => tokens.push(Token::Write),
                        "func" => tokens.push(Token::Func),
                        "make" => tokens.push(Token::Make),
                        "string" => tokens.push(Token::StringType),
                        "int" => tokens.push(Token::Int),
                        "bool" => tokens.push(Token::Bool),
                        "true" => tokens.push(Token::True),
                        "false" => tokens.push(Token::False),
                        "if" => tokens.push(Token::If),
                        "else" => tokens.push(Token::Else),
                        "elif" => tokens.push(Token::Elif),
                        "loop" => tokens.push(Token::Loop),
                        "with" => tokens.push(Token::With),
                        // UI Components
                        "Window" => tokens.push(Token::Window),
                        "Column" => tokens.push(Token::Column),
                        "Row" => tokens.push(Token::Row),
                        "Text" => tokens.push(Token::Text),
                        "Button" => tokens.push(Token::Button),
                        "Input" => tokens.push(Token::Input),
                        // UI Relationships
                        "Parent" => tokens.push(Token::Parent),
                        "Child" => tokens.push(Token::Child),
                        // UI Events
                        "on_tap" => tokens.push(Token::OnTap),
                        "on_click" => tokens.push(Token::OnClick),
                        "on_change" => tokens.push(Token::OnChange),
                        "on_input" => tokens.push(Token::OnInput),
                        // UI Rendering
                        "render" => tokens.push(Token::Render),
                        // UI Identity
                        "Identity" => tokens.push(Token::Identity),
                        "Rewrite" => tokens.push(Token::Rewrite),
                        _ => tokens.push(Token::Identifier(word)),
                    }
                }
                
                _ => {
                    return Err(LexerError::new(
                        &format!("Unexpected character: '{}'", ch), 
                        self.pos, 
                        self.line
                    ));
                }
            }
        }
        
        tokens.push(Token::EOF);
        Ok(tokens)
    }
}