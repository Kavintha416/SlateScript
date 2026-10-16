//! Annotations — `#[name]` tags that transform how the following
//! statement is interpreted.
//!
//! Currently built-in only. Will be extensible via `LanguageExtension`
//! in a future commit.

mod trait_def;
mod registry;
mod builtin;

pub use trait_def::Annotation;
pub use registry::AnnotationRegistry;
pub use builtin::StructAnnotation;