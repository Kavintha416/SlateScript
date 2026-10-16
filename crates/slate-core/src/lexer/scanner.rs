//! Character-level scanner for SlateScript's core language.
//!
//! Does not know about extensions, UI components, or LLM keywords.
//! Words that aren't core keywords come out as `Token::Identifier`;
//! the parser asks the extension registry what to do with them.

use super::error::LexerError;
use super::token::Token;

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

            if ch == '\n' {
                self.line += 1;
            }

            match ch {
                // ---- Operators and delimiters ----
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
                '{' => { tokens.push(Token::LeftBrace);    self.pos += 1; }
                '}' => { tokens.push(Token::RightBrace);   self.pos += 1; }
                '(' => { tokens.push(Token::LeftParen);    self.pos += 1; }
                ')' => { tokens.push(Token::RightParen);   self.pos += 1; }
                ';' => { tokens.push(Token::Semicolon);    self.pos += 1; }
                ':' => { tokens.push(Token::Colon);        self.pos += 1; }
                ',' => { tokens.push(Token::Comma);        self.pos += 1; }
                '.' => { tokens.push(Token::Dot);          self.pos += 1; }

                // ---- '[' handling: commands, or plain bracket ----
                '[' if self.pos + 1 < self.chars.len()
                    && self.chars[self.pos + 1] == '!' => {
                    // [!Name] — command
                    self.pos += 2; // skip '[' and '!'
                    let mut name = String::new();
                    while self.pos < self.chars.len()
                        && (self.chars[self.pos].is_alphabetic()
                            || self.chars[self.pos] == '_'
                            || self.chars[self.pos].is_numeric()) {
                        name.push(self.chars[self.pos]);
                        self.pos += 1;
                    }
                    if self.pos >= self.chars.len() || self.chars[self.pos] != ']' {
                        return Err(LexerError::new(
                            "Unterminated command: expected ']'",
                            self.pos,
                            self.line,
                        ));
                    }
                    if name.is_empty() {
                        return Err(LexerError::new(
                            "Empty command: expected a name after '[!'",
                            self.pos,
                            self.line,
                        ));
                    }
                    self.pos += 1; // skip ']'
                    tokens.push(Token::Command { name });
                }
                '[' => { tokens.push(Token::LeftBracket);  self.pos += 1; }
                ']' => { tokens.push(Token::RightBracket); self.pos += 1; }

                // ---- '#' handling: annotation, hex color, or error ----
                '#' if self.pos + 1 < self.chars.len()
                    && self.chars[self.pos + 1] == '[' => {
                    // #[Name] — annotation
                    self.pos += 2; // skip '#' and '['
                    let mut name = String::new();
                    while self.pos < self.chars.len()
                        && (self.chars[self.pos].is_alphabetic()
                            || self.chars[self.pos] == '_'
                            || self.chars[self.pos].is_numeric()) {
                        name.push(self.chars[self.pos]);
                        self.pos += 1;
                    }
                    if self.pos >= self.chars.len() || self.chars[self.pos] != ']' {
                        return Err(LexerError::new(
                            "Unterminated annotation: expected ']'",
                            self.pos,
                            self.line,
                        ));
                    }
                    if name.is_empty() {
                        return Err(LexerError::new(
                            "Empty annotation: expected a name after '#['",
                            self.pos,
                            self.line,
                        ));
                    }
                    self.pos += 1; // skip ']'
                    tokens.push(Token::Annotation { name });
                }
                '#' if self.pos + 1 < self.chars.len()
                    && self.chars[self.pos + 1].is_ascii_hexdigit() => {
                    // #RRGGBB or #RGB — hex color literal
                    self.pos += 1; // skip '#'
                    let mut hex = String::new();
                    while self.pos < self.chars.len()
                        && self.chars[self.pos].is_ascii_hexdigit() {
                        hex.push(self.chars[self.pos]);
                        self.pos += 1;
                    }
                    if hex.len() != 3 && hex.len() != 6 {
                        return Err(LexerError::new(
                            &format!(
                                "Invalid hex color '#{}': expected 3 or 6 hex digits",
                                hex
                            ),
                            self.pos,
                            self.line,
                        ));
                    }
                    tokens.push(Token::HexColor(format!("#{}", hex)));
                }
                '#' => {
                    return Err(LexerError::new(
                        "Unexpected character: '#' (expected '#[Name]' or a hex color like '#FF0000')",
                        self.pos,
                        self.line,
                    ));
                }

                // ---- Whitespace ----
                ' ' | '\n' | '\t' | '\r' => {
                    self.pos += 1;
                }

                // ---- '=' and '==' ----
                '=' => {
                    self.pos += 1;
                    if self.pos < self.chars.len() && self.chars[self.pos] == '=' {
                        tokens.push(Token::EqualEqual);
                        self.pos += 1;
                    } else {
                        tokens.push(Token::Equal);
                    }
                }

                // ---- Arithmetic ----
                '+' => { tokens.push(Token::Plus);     self.pos += 1; }
                '*' => { tokens.push(Token::Multiply); self.pos += 1; }

                '-' if self.pos + 1 < self.chars.len()
                    && self.chars[self.pos + 1] == '-' => {
                    // Line comment
                    self.pos += 2;
                    while self.pos < self.chars.len() && self.chars[self.pos] != '\n' {
                        self.pos += 1;
                    }
                }
                '-' => { tokens.push(Token::Minus); self.pos += 1; }

                '/' if self.pos + 1 < self.chars.len()
                    && self.chars[self.pos + 1] == '/' => {
                    // Line comment
                    self.pos += 2;
                    while self.pos < self.chars.len() && self.chars[self.pos] != '\n' {
                        self.pos += 1;
                    }
                }
                '/' => { tokens.push(Token::Divide); self.pos += 1; }

                // ---- '!=' ----
                '!' => {
                    self.pos += 1;
                    if self.pos < self.chars.len() && self.chars[self.pos] == '=' {
                        tokens.push(Token::NotEqual);
                        self.pos += 1;
                    } else {
                        // Original behavior: silently skip a lone '!'.
                        self.pos += 1;
                    }
                }

                // ---- Numbers ----
                c if c.is_digit(10) => {
                    let mut num = String::new();
                    let mut is_float = false;

                    while self.pos < self.chars.len()
                        && (self.chars[self.pos].is_digit(10)
                            || self.chars[self.pos] == '.') {
                        if self.chars[self.pos] == '.' {
                            if is_float {
                                return Err(LexerError::new(
                                    "Invalid number: multiple decimal points",
                                    self.pos,
                                    self.line,
                                ));
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

                // ---- Strings ----
                '"' => {
                    self.pos += 1;
                    let mut content = String::new();

                    while self.pos < self.chars.len()
                        && self.chars[self.pos] != '"' {
                        content.push(self.chars[self.pos]);
                        self.pos += 1;
                    }

                    if self.pos < self.chars.len() {
                        self.pos += 1;
                    } else {
                        return Err(LexerError::new(
                            "Unterminated string: missing closing quote",
                            self.pos,
                            self.line,
                        ));
                    }

                    tokens.push(Token::String(content));
                }

                // ---- Identifiers and keywords ----
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
                        "from"   => tokens.push(Token::From),
                        "write"  => tokens.push(Token::Write),
                        "func"   => tokens.push(Token::Func),
                        "make"   => tokens.push(Token::Make),
                        "return" => tokens.push(Token::Return),
                        "string" => tokens.push(Token::StringType),
                        "int"    => tokens.push(Token::Int),
                        "bool"   => tokens.push(Token::Bool),
                        "true"   => tokens.push(Token::True),
                        "false"  => tokens.push(Token::False),
                        "if"     => tokens.push(Token::If),
                        "else"   => tokens.push(Token::Else),
                        "elif"   => tokens.push(Token::Elif),
                        "loop"   => tokens.push(Token::Loop),
                        "to"     => tokens.push(Token::To),
                        "with"   => tokens.push(Token::With),
                        "train"  => tokens.push(Token::Train),
                        "struct" => tokens.push(Token::Struct),

                        _ => tokens.push(Token::Identifier(word)),
                    }
                }

                // ---- Unknown character ----
                _ => {
                    return Err(LexerError::new(
                        &format!("Unexpected character: '{}'", ch),
                        self.pos,
                        self.line,
                    ));
                }
            }
        }

        tokens.push(Token::EOF);
        Ok(tokens)
    }
}