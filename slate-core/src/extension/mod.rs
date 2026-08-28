// slate-core/src/extension/mod.rs

mod extension_trait;
mod registry;

pub use extension_trait::{LanguageExtension, CustomToken, ExtensionToken, InterpreterContext, ExtensionParseResult};
pub use registry::ExtensionRegistry;