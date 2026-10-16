//! Styles Framework Module
//! 
//! Provides CSS-like styling capabilities for Slattery UI components.
//! Supports style definitions, cascading, states, and integration with egui renderer.

pub mod style_lexer;
pub mod style_interpreter;
pub mod style_engine;
pub mod style_applier;

// Re-export main types for convenience
pub use style_engine::StyleEngine;
pub use style_interpreter::{StyleInterpreter, StyleValue, StyleRule, StyleState};
pub use style_lexer::StyleToken;
pub use style_applier::StyleApplier;