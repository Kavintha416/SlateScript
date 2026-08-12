//! Style Lexer
//! 
//! Tokenizes CSS-like style syntax for Slattery UI components from .sts files.

#[derive(Debug, PartialEq, Clone)]
pub enum StyleToken {
    // Style definition keywords
    Style,
    Define,
    Import,
    From,
    
    // Selectors
    Selector(String),
    ClassSelector(String),
    IdSelector(String),
    PseudoClass(String),
    PseudoElement(String),
    ComponentSelector(String),
    
    // Properties and values
    Property(String),
    Value(String),
    
    // Delimiters
    LeftBrace,    // {
    RightBrace,   // }
    Colon,        // :
    Semicolon,    // ;
    Comma,        // ,
    
    // States
    Hover,
    Active,
    Focus,
    Disabled,
    
    // Values
    Color(String),
    Number(f64),
    Unit(String),
    String(String),
    
    // Important
    Important,
    
    // End of file
    EOF,
}

pub struct StyleLexer {
    source: String,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
}

impl StyleLexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }
    
    pub fn tokenize(&mut self) -> Vec<StyleToken> {
        let mut tokens = Vec::new();
        
        while !self.is_at_end() {
            match self.current_char() {
                // Skip whitespace and comments
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                
                // Comments
                '/' if self.peek_char() == Some('*') => {
                    self.skip_block_comment();
                }
                
                '/' if self.peek_char() == Some('/') => {
                    self.skip_line_comment();
                }
                
                // Style keywords
                c if c.is_alphabetic() => {
                    if let Some(token) = self.parse_identifier_or_keyword() {
                        tokens.push(token);
                    }
                }
                
                // Strings
                '"' => {
                    tokens.push(self.parse_string());
                }
                
                // Numbers
                c if c.is_digit(10) => {
                    tokens.push(self.parse_number());
                }
                
                // Delimiters
                '{' => {
                    tokens.push(StyleToken::LeftBrace);
                    self.advance();
                }
                
                '}' => {
                    tokens.push(StyleToken::RightBrace);
                    self.advance();
                }
                
                ':' => {
                    tokens.push(StyleToken::Colon);
                    self.advance();
                }
                
                ';' => {
                    tokens.push(StyleToken::Semicolon);
                    self.advance();
                }
                
                ',' => {
                    tokens.push(StyleToken::Comma);
                    self.advance();
                }
                
                // Selectors
                '.' => {
                    tokens.push(self.parse_class_selector());
                }
                
                '#' => {
                    // Treat as a hex color only if followed by exactly 6 hex digits
                    // (no alphabetic word characters after). e.g. #FF3B30 is a color,
                    // #ButtonOne is an id selector.
                    let is_color = {
                        let mut lookahead = self.pos + 1;
                        let mut hex_count = 0;
                        while lookahead < self.chars.len() && self.chars[lookahead].is_ascii_hexdigit() {
                            hex_count += 1;
                            lookahead += 1;
                        }
                        // It's a color if we have exactly 6 hex chars and the next char
                        // is not an alphanumeric (i.e. not part of a longer identifier)
                        hex_count == 6 && (lookahead >= self.chars.len() || !self.chars[lookahead].is_alphanumeric())
                    };
                    if is_color {
                        tokens.push(self.parse_color_literal());
                    } else {
                        tokens.push(self.parse_id_selector());
                    }
                }
                
                '@' => {
                    tokens.push(self.parse_component_selector());
                }
                
                _ => {
                    // Unknown character, skip it
                    self.advance();
                }
            }
        }
        
        tokens.push(StyleToken::EOF);
        tokens
    }
    
    fn parse_identifier_or_keyword(&mut self) -> Option<StyleToken> {
        let mut word = String::new();
        
        while !self.is_at_end() && (self.current_char().is_alphabetic() || 
                                     self.current_char() == '_' || 
                                     self.current_char().is_digit(10) ||
                                     self.current_char() == '-') {
            word.push(self.current_char());
            self.advance();
        }
        
        match word.as_str() {
            "style" => Some(StyleToken::Style),
            "define" => Some(StyleToken::Define),
            "import" => Some(StyleToken::Import),
            "from" => Some(StyleToken::From),
            "hover" => Some(StyleToken::Hover),
            "active" => Some(StyleToken::Active),
            "focus" => Some(StyleToken::Focus),
            "disabled" => Some(StyleToken::Disabled),
            "important" => Some(StyleToken::Important),
            _ => Some(StyleToken::Selector(word)),
        }
    }
    
    fn parse_string(&mut self) -> StyleToken {
        self.advance(); // Skip opening quote
        let mut content = String::new();
        
        while !self.is_at_end() && self.current_char() != '"' {
            if self.current_char() == '\\' {
                self.advance(); // Skip backslash
                if !self.is_at_end() {
                    content.push(self.current_char());
                    self.advance();
                }
            } else {
                content.push(self.current_char());
                self.advance();
            }
        }
        
        if !self.is_at_end() {
            self.advance(); // Skip closing quote
        }
        
        StyleToken::String(content)
    }
    
    fn parse_number(&mut self) -> StyleToken {
        let mut num_str = String::new();
        
        while !self.is_at_end() && (self.current_char().is_digit(10) || self.current_char() == '.') {
            num_str.push(self.current_char());
            self.advance();
        }
        
        // Check for unit
        let mut unit = String::new();
        while !self.is_at_end() && self.current_char().is_alphabetic() {
            unit.push(self.current_char());
            self.advance();
        }
        
        if let Ok(num) = num_str.parse::<f64>() {
            StyleToken::Number(num)
        } else {
            StyleToken::Value(num_str + &unit)
        }
    }
    
    fn parse_class_selector(&mut self) -> StyleToken {
        self.advance(); // Skip '.'
        let mut class_name = String::new();
        
        while !self.is_at_end() && (self.current_char().is_alphabetic() || 
                                     self.current_char() == '_' || 
                                     self.current_char().is_digit(10) ||
                                     self.current_char() == '-') {
            class_name.push(self.current_char());
            self.advance();
        }
        
        StyleToken::ClassSelector(class_name)
    }
    
    fn parse_id_selector(&mut self) -> StyleToken {
        self.advance(); // Skip '#'
        let mut id_name = String::new();
        
        while !self.is_at_end() && (self.current_char().is_alphabetic() || 
                                     self.current_char() == '_' || 
                                     self.current_char().is_digit(10) ||
                                     self.current_char() == '-') {
            id_name.push(self.current_char());
            self.advance();
        }
        
        StyleToken::IdSelector(id_name)
    }

    fn parse_color_literal(&mut self) -> StyleToken {
        self.advance(); // Skip '#'
        let mut hex = String::new();

        while !self.is_at_end() && self.current_char().is_ascii_hexdigit() {
            hex.push(self.current_char());
            self.advance();
        }

        StyleToken::Color(format!("#{}", hex))
    }
    
    fn parse_component_selector(&mut self) -> StyleToken {
        self.advance(); // Skip '@'
        let mut component_name = String::new();
        
        while !self.is_at_end() && (self.current_char().is_alphabetic() || 
                                     self.current_char() == '_' || 
                                     self.current_char().is_digit(10)) {
            component_name.push(self.current_char());
            self.advance();
        }
        
        StyleToken::ComponentSelector(component_name)
    }
    
    fn parse_pseudo_class(&mut self) -> StyleToken {
        self.advance(); // Skip ':'
        let mut pseudo_name = String::new();
        
        while !self.is_at_end() && (self.current_char().is_alphabetic() || 
                                     self.current_char() == '-') {
            pseudo_name.push(self.current_char());
            self.advance();
        }
        
        match pseudo_name.as_str() {
            "hover" => StyleToken::Hover,
            "active" => StyleToken::Active,
            "focus" => StyleToken::Focus,
            "disabled" => StyleToken::Disabled,
            _ => StyleToken::PseudoClass(pseudo_name),
        }
    }
    
    fn parse_pseudo_element(&mut self) -> StyleToken {
        self.advance(); // Skip first ':'
        self.advance(); // Skip second ':'
        let mut pseudo_name = String::new();
        
        while !self.is_at_end() && (self.current_char().is_alphabetic() || 
                                     self.current_char() == '-') {
            pseudo_name.push(self.current_char());
            self.advance();
        }
        
        StyleToken::PseudoElement(pseudo_name)
    }
    
    fn skip_block_comment(&mut self) {
        self.advance(); // Skip '/'
        self.advance(); // Skip '*'
        
        while !self.is_at_end() && !(self.current_char() == '*' && self.peek_char() == Some('/')) {
            self.advance();
        }
        
        if !self.is_at_end() {
            self.advance(); // Skip '*'
            self.advance(); // Skip '/'
        }
    }
    
    fn skip_line_comment(&mut self) {
        self.advance(); // Skip '/'
        self.advance(); // Skip '/'
        
        while !self.is_at_end() && self.current_char() != '\n' {
            self.advance();
        }
    }
    
    fn current_char(&self) -> char {
        self.chars[self.pos]
    }
    
    fn peek_char(&self) -> Option<char> {
        if self.pos + 1 < self.chars.len() {
            Some(self.chars[self.pos + 1])
        } else {
            None
        }
    }
    
    fn advance(&mut self) {
        if self.current_char() == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        
        if self.pos < self.chars.len() {
            self.pos += 1;
        }
    }
    
    fn is_at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }
}
