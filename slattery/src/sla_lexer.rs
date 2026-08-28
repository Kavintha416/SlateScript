//! Slattery UI Lexer
//! 
//! Extends the SlateScript lexer to handle UI-specific syntax including
//! component declarations, property assignments, and event handlers.


#[derive(Debug, Clone, PartialEq)]
pub enum UiToken {
    // Component syntax
    Make,
    LessThan,
    GreaterThan,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Equal,
    EqualEqual,
    LessThanOrEqual,
    GreaterThanOrEqual,
    Colon,
    Comma,
    Slash,
    
    // Component relationships
    Parent,
    Child,
    
    // Component properties
    Identity,
    Rewrite,
    Get,
    
    // Layout components
    Window,
    Column,
    Row,
    Text,
    Button,
    Input,
    
    // Event handlers
    OnTap,
    OnClick,
    OnChange,
    OnInput,
    
    // Rendering
    Render,
    
    // Standard tokens (inherited from main lexer)
    Identifier(String),
    String(String),
    Number(f64),
    Boolean(bool),
    True,
    False,
    
    // Control flow
    Func,
    Import,
    From,
    
    // Delimiters
    LeftParen,
    RightParen,
    Semicolon,
    EOF,
}

pub struct UiLexer {
    source: String,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
}

impl UiLexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }
    
    pub fn tokenize(&mut self) -> Vec<UiToken> {
        let mut tokens = Vec::new();
        
        while !self.is_at_end() {
            match self.current_char() {
                '<' => {
                    if self.peek_next() == '=' {
                        self.advance(); // consume <
                        self.advance(); // consume =
                        tokens.push(UiToken::LessThanOrEqual);
                    } else {
                        tokens.push(UiToken::LessThan);
                        self.advance();
                    }
                }
                '>' => {
                    if self.peek_next() == '=' {
                        self.advance(); // consume >
                        self.advance(); // consume =
                        tokens.push(UiToken::GreaterThanOrEqual);
                    } else {
                        tokens.push(UiToken::GreaterThan);
                        self.advance();
                    }
                }
                '[' => {
                    tokens.push(UiToken::LeftBracket);
                    self.advance();
                }
                ']' => {
                    tokens.push(UiToken::RightBracket);
                    self.advance();
                }
                '/' => {
                    tokens.push(UiToken::Slash);
                    self.advance();
                }
                '{' => {
                    tokens.push(UiToken::LeftBrace);
                    self.advance();
                }
                '}' => {
                    tokens.push(UiToken::RightBrace);
                    self.advance();
                }
                '(' => {
                    tokens.push(UiToken::LeftParen);
                    self.advance();
                }
                ')' => {
                    tokens.push(UiToken::RightParen);
                    self.advance();
                }
                ':' => {
                    tokens.push(UiToken::Colon);
                    self.advance();
                }
                ';' => {
                    tokens.push(UiToken::Semicolon);
                    self.advance();
                }
                ',' => {
                    tokens.push(UiToken::Comma);
                    self.advance();
                }
                '=' => {
                    if self.peek_next() == '=' {
                        self.advance(); // consume =
                        self.advance(); // consume =
                        tokens.push(UiToken::EqualEqual);
                    } else {
                        tokens.push(UiToken::Equal);
                        self.advance();
                    }
                }
                '"' => {
                    tokens.push(self.string_literal());
                }
                '\'' => {
                    tokens.push(self.string_literal_single());
                }
                '0'..='9' => {
                    tokens.push(self.number_literal());
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    tokens.push(self.identifier());
                }
                ' ' | '\t' | '\r' => {
                    self.advance(); // skip whitespace
                }
                '\n' => {
                    self.line += 1;
                    self.column = 1;
                    self.advance();
                }
                '-' => {
                    if self.peek_next() == '-' {
                        // Skip comment
                        self.advance(); // consume -
                        self.advance(); // consume -
                        while !self.is_at_end() && self.current_char() != '\n' {
                            self.advance();
                        }
                    } else {
                        self.advance();
                    }
                }
                _ => {
                    // Skip unknown characters
                    self.advance();
                }
            }
        }
        
        tokens.push(UiToken::EOF);
        tokens
    }
    
    fn identifier(&mut self) -> UiToken {
        let start = self.pos;
        while !self.is_at_end() && (self.current_char().is_alphanumeric() || self.current_char() == '_') {
            self.advance();
        }
        
        let identifier = &self.source[start..self.pos];
        
        match identifier {
            "make" => UiToken::Make,
            "Parent" => UiToken::Parent,
            "Child" => UiToken::Child,
            "Identity" => UiToken::Identity,
            "Rewrite" => UiToken::Rewrite,
            "get" => UiToken::Get,
            "render" => UiToken::Render,
            "func" => UiToken::Func,
            "import" => UiToken::Import,
            "from" => UiToken::From,
            "true" => UiToken::True,
            "false" => UiToken::False,
            "on_tap" => UiToken::OnTap,
            "on_click" => UiToken::OnClick,  // Make sure this is here!
            "on_change" => UiToken::OnChange,
            "on_input" => UiToken::OnInput,
            "Window" => UiToken::Window,
            "Column" => UiToken::Column,
            "Row" => UiToken::Row,
            "Text" => UiToken::Text,
            "Button" => UiToken::Button,
            "Input" => UiToken::Input,
            _ => UiToken::Identifier(identifier.to_string()),
        }
    }
    
    fn string_literal(&mut self) -> UiToken {
        self.advance(); // consume opening quote
        let start = self.pos;
        
        while !self.is_at_end() && self.current_char() != '"' {
            if self.current_char() == '\\' {
                self.advance(); // consume escape character
                if !self.is_at_end() {
                    self.advance(); // consume escaped character
                }
            } else {
                self.advance();
            }
        }
        
        let end = self.pos;
        let string_value = self.source[start..end].to_string();
        self.advance(); // consume closing quote
        
        UiToken::String(string_value)
    }
    
    fn string_literal_single(&mut self) -> UiToken {
        self.advance(); // consume opening quote
        let start = self.pos;
        
        while !self.is_at_end() && self.current_char() != '\'' {
            if self.current_char() == '\\' {
                self.advance(); // consume escape character
                if !self.is_at_end() {
                    self.advance(); // consume escaped character
                }
            } else {
                self.advance();
            }
        }
        
        let end = self.pos;
        let string_value = self.source[start..end].to_string();
        self.advance(); // consume closing quote
        
        UiToken::String(string_value)
    }
    
    fn number_literal(&mut self) -> UiToken {
        let start = self.pos;
        
        while !self.is_at_end() && (self.current_char().is_numeric() || self.current_char() == '.') {
            self.advance();
        }
        
        let number_str = &self.source[start..self.pos];
        if let Ok(number) = number_str.parse::<f64>() {
            UiToken::Number(number)
        } else {
            UiToken::Number(0.0) // fallback
        }
    }
    
    fn current_char(&self) -> char {
        if self.pos < self.chars.len() {
            self.chars[self.pos]
        } else {
            '\0'
        }
    }
    
    fn peek_next(&self) -> char {
        if self.pos + 1 < self.chars.len() {
            self.chars[self.pos + 1]
        } else {
            '\0'
        }
    }
    
    fn advance(&mut self) {
        if !self.is_at_end() {
            if self.current_char() == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.pos += 1;
        }
    }
    
    fn is_at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }
}

pub fn debug_tokens(source: &str) {
    let mut lexer = UiLexer::new(source);
    let tokens = lexer.tokenize();
    
    println!("Tokens for: {:?}", source);
    for (i, token) in tokens.iter().enumerate() {
        println!("{}: {:?}", i, token);
    }
    println!("---");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_component_syntax() {
        let mut lexer = UiLexer::new("make App = Window {title: \"My App\"}");
        let tokens = lexer.tokenize();
        
        assert_eq!(tokens[0], UiToken::Make);
        assert_eq!(tokens[1], UiToken::Identifier("App".to_string()));
        assert_eq!(tokens[2], UiToken::Equal);
        assert_eq!(tokens[3], UiToken::Window);
        assert_eq!(tokens[4], UiToken::LeftBrace);
        assert_eq!(tokens[5], UiToken::Identifier("title".to_string()));
        assert_eq!(tokens[6], UiToken::Colon);
        assert_eq!(tokens[7], UiToken::String("My App".to_string()));
        assert_eq!(tokens[8], UiToken::RightBrace);
    }
    
    #[test]
    fn test_parent_child_syntax() {
        let mut lexer = UiLexer::new("Parent: <Column> Child: <Text>");
        let tokens = lexer.tokenize();
        
        assert_eq!(tokens[0], UiToken::Parent);
        assert_eq!(tokens[1], UiToken::Colon);
        assert_eq!(tokens[2], UiToken::LessThan);
        assert_eq!(tokens[3], UiToken::Column);
        assert_eq!(tokens[4], UiToken::GreaterThan);
        assert_eq!(tokens[5], UiToken::Child);
        assert_eq!(tokens[6], UiToken::Colon);
        assert_eq!(tokens[7], UiToken::LessThan);
        assert_eq!(tokens[8], UiToken::Text);
        assert_eq!(tokens[9], UiToken::GreaterThan);
    }
    
    #[test]
    fn test_child_parent_syntax() {
        let mut lexer = UiLexer::new("Child/Parent: <Button>");
        let tokens = lexer.tokenize();
        
        assert_eq!(tokens[0], UiToken::Child);
        assert_eq!(tokens[1], UiToken::Slash);
        assert_eq!(tokens[2], UiToken::Parent);
        assert_eq!(tokens[3], UiToken::Colon);
        assert_eq!(tokens[4], UiToken::LessThan);
        assert_eq!(tokens[5], UiToken::Button);
        assert_eq!(tokens[6], UiToken::GreaterThan);
    }
    
    #[test]
    fn test_component_with_identity() {
        let mut lexer = UiLexer::new("Text [Identity = my_Text] <value: \"Hello\">");
        let tokens = lexer.tokenize();
        
        assert_eq!(tokens[0], UiToken::Text);
        assert_eq!(tokens[1], UiToken::LeftBracket);
        assert_eq!(tokens[2], UiToken::Identity);
        assert_eq!(tokens[3], UiToken::Equal);
        assert_eq!(tokens[4], UiToken::Identifier("my_Text".to_string()));
        assert_eq!(tokens[5], UiToken::RightBracket);
        assert_eq!(tokens[6], UiToken::LessThan);
        assert_eq!(tokens[7], UiToken::Identifier("value".to_string()));
        assert_eq!(tokens[8], UiToken::Colon);
        assert_eq!(tokens[9], UiToken::String("Hello".to_string()));
        assert_eq!(tokens[10], UiToken::GreaterThan);
    }
}