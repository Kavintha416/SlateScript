//! Abstract Syntax Tree (AST) for SlateScript
//! 
//! This module separates parsing concerns from execution, providing a clean AST
//! that can be analyzed and executed independently.

use std::collections::HashMap;

/// Source position information for better error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self { start, end, line, column }
    }
    
    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line.min(other.line),
            column: self.column.min(other.column),
        }
    }
}

/// AST node with span information
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

/// Literal values
#[derive(Debug, Clone)]
pub enum LiteralValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    
    And,
    Or,
}

/// UI Component Types
#[derive(Debug, Clone, PartialEq)]
pub enum UiComponentType {
    Window,
    Column,
    Row,
    Text,
    Button,
    Input,
}

impl UiComponentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            UiComponentType::Window => "Window",
            UiComponentType::Column => "Column",
            UiComponentType::Row => "Row",
            UiComponentType::Text => "Text",
            UiComponentType::Button => "Button",
            UiComponentType::Input => "Input",
        }
    }
}

/// Expression types in SlateScript
#[derive(Debug, Clone)]
pub enum Expression {
    /// Literal values (numbers, strings, bool, null)
    Literal(LiteralValue, Span),
    
    /// Variable references
    Variable(String, Span),
    
    /// Binary operations (a + b, a > b, etc.)
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
        span: Span,
    },
    
    /// Function calls
    FunctionCall {
        name: String,
        arguments: Vec<Expression>,
        span: Span,
    },
    
    /// Array literals
    Array(Vec<Expression>, Span),
    
    /// Object literals
    Object(HashMap<String, Expression>, Span),
    
    /// Property access (obj.property)
    PropertyAccess {
        object: Box<Expression>,
        property: String,
        span: Span,
    },
    
    /// Import expressions
    Import {
        from: String,
        items: Vec<String>,
        span: Span,
    },
    
    /// Make expressions for UI components (deprecated - use UiComponent)
    Make {
        name: String,
        properties: HashMap<String, Expression>,
        children: Vec<Expression>,
        span: Span,
    },
    
    /// Render expressions (deprecated - use UiRender)
    Render {
        target: Box<Expression>,
        span: Span,
    },
    
    /// UI Component declaration
    UiComponent {
        component_type: UiComponentType,
        identity: Option<String>,
        properties: HashMap<String, Expression>,
        children: Vec<Expression>,
        events: HashMap<String, String>,
        span: Span,
    },
    
    /// UI Render statement
    UiRender {
        target: Box<Expression>,
        span: Span,
    },
    
    /// Extension node - handled by extensions
    Extension {
        name: String,           // Extension name (e.g., "slattery")
        type_name: String,      // Type name for debugging
        span: Span,
    },
}

/// Statement types in SlateScript
#[derive(Debug, Clone)]
pub enum Statement {
    /// Expression statements (standalone expressions)
    Expression(Expression),
    
    /// Variable assignments (let x = 5)
    Assignment {
        name: String,
        value: Expression,
        span: Span,
    },
    
    /// Function definitions
    FunctionDefinition {
        name: String,
        parameters: Vec<String>,
        body: Vec<Statement>,
        span: Span,
    },
    
    /// If statements with elif support
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        elif_branches: Vec<(Expression, Vec<Statement>)>,
        else_branch: Option<Vec<Statement>>,
        span: Span,
    },
    
    /// While loops
    While {
        condition: Expression,
        body: Vec<Statement>,
        counter_var: Option<String>,
        span: Span,
    },
    
    /// Write statements (console output)
    Write {
        value: Expression,
        span: Span,
    },
    
    /// Import statements
    ImportStatement {
        from: String,
        items: Vec<String>,
        span: Span,
    },
    
    /// Return statements
    Return {
        value: Option<Expression>,
        span: Span,
    },
}

/// Complete AST program
#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
    pub span: Span,
}

impl Program {
    pub fn new(statements: Vec<Statement>) -> Self {
        let span = if statements.is_empty() {
            Span::new(0, 0, 0, 0)
        } else {
            let first = statements.first().unwrap().span();
            let last = statements.last().unwrap().span();
            first.merge(&last)
        };
        
        Self { statements, span }
    }
}

/// Trait to get span from AST nodes
pub trait SpannedNode {
    fn span(&self) -> Span;
}

impl SpannedNode for Statement {
    fn span(&self) -> Span {
        match self {
            Statement::Expression(expr) => expr.span(),
            Statement::Assignment { span, .. } => *span,
            Statement::FunctionDefinition { span, .. } => *span,
            Statement::If { span, .. } => *span,
            Statement::While { span, .. } => *span,
            Statement::Write { span, .. } => *span,
            Statement::ImportStatement { span, .. } => *span,
            Statement::Return { span, .. } => *span,
        }
    }
}

impl SpannedNode for Expression {
    fn span(&self) -> Span {
        match self {
            Expression::Literal(_, span) => *span,
            Expression::Variable(_, span) => *span,
            Expression::Binary { span, .. } => *span,
            Expression::FunctionCall { span, .. } => *span,
            Expression::Array(_, span) => *span,
            Expression::Object(_, span) => *span,
            Expression::PropertyAccess { span, .. } => *span,
            Expression::Import { span, .. } => *span,
            Expression::Make { span, .. } => *span,
            Expression::Render { span, .. } => *span,
            Expression::UiComponent { span, .. } => *span,
            Expression::UiRender { span, .. } => *span,
            Expression::Extension { span, .. } => *span,
        }
    }
}

/// Error types for AST construction
#[derive(Debug, Clone)]
pub enum AstError {
    ParseError(String, Span),
    UnexpectedToken(String, Span),
    ExpectedToken(String, Span),
    InvalidSyntax(String, Span),
}

impl std::fmt::Display for AstError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AstError::ParseError(msg, span) => {
                write!(f, "Parse error at line {}: {}", span.line, msg)
            }
            AstError::UnexpectedToken(token, span) => {
                write!(f, "Unexpected token '{}' at line {}: {}", token, span.line, token)
            }
            AstError::ExpectedToken(expected, span) => {
                write!(f, "Expected '{}' at line {}", expected, span.line)
            }
            AstError::InvalidSyntax(msg, span) => {
                write!(f, "Invalid syntax at line {}: {}", span.line, msg)
            }
        }
    }
}

impl std::error::Error for AstError {}