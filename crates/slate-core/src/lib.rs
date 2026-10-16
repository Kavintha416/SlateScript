// slate-core/src/lib.rs

pub mod lexer;
pub mod parser;
pub mod ast;
pub mod ast_interpreter;
pub mod value;
pub mod debug;
pub mod extension;
pub mod annotation;

pub use lexer::{Lexer, Token};
pub use parser::Parser;
pub use ast::{Program, Expression, Span, Statement};
pub use ast_interpreter::AstInterpreter;
pub use value::Value;
pub use extension::{ExtensionRegistry, LanguageExtension};
pub use annotation::{Annotation, AnnotationRegistry};