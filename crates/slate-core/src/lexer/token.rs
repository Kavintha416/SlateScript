//! Token definitions for SlateScript's core language.
//!
//! Extensions (slattery, slate-sllm, ...) register their own keywords
//! via `LanguageExtension::custom_tokens()`. Core tokens below are the
//! language primitives only.

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // === Core keywords ===
    Make,
    Func,
    If,
    Elif,
    Else,
    Loop,
    With,
    To,
    Write,
    Return,
    Import,
    From,
    Train,
    Struct,

    // === Type keywords ===
    Int,
    StringType,
    Bool,

    // === Literals ===
    Number(i64),
    Float(f64),
    String(String),
    Identifier(String),
    True,
    False,

    /// Hex colour literal, e.g. `#FF3B30` or `#FFF`.
    ///
    /// Emitted by the lexer when it sees `#` followed by 3 or 6 hex
    /// digits. Used by the Slattery extension's `style` blocks, but
    /// the token itself is a core language construct so any extension
    /// can consume it.
    HexColor(String),

    // === Operators ===
    Plus,
    Minus,
    Multiply,
    Divide,
    Equal,
    EqualEqual,
    NotEqual,
    LessThan,
    GreaterThan,
    LessEqual,
    GreaterEqual,

    // === Delimiters ===
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Comma,
    Colon,
    Dot,
    Semicolon,
    Slash,

    // === Metadata ===
    //
    // `#[Name]` — an annotation that tags the following statement.
    // `[!Name]` — a command that runs at parse time and sets program
    //             state. (Reserved; not yet wired into the parser.)
    Annotation { name: String },
    Command { name: String },

    // === Extension escape hatch ===
    Extension { name: String, keyword: String },

    // === End of input ===
    EOF,
}