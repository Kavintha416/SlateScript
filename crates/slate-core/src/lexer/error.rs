//! Errors produced by the lexer.

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
        write!(
            f,
            "Lexer error at line {} (pos {}): {}",
            self.line, self.position, self.message
        )
    }
}

impl std::error::Error for LexerError {}