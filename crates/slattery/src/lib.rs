pub mod install;
pub mod build;
pub mod sla_lexer;
pub mod sla_interpreter;
pub mod egui_renderer;
pub mod ui_integration;
pub mod styles;
pub mod ui_extension;
pub mod logger;

// NEW
pub mod icons;
pub mod run;

pub use ui_extension::SlatteryExtension;
pub use ui_integration::UiFramework;
pub use sla_interpreter::{UiComponent, UiValue};
pub use egui_renderer::{EguiRenderer, SlatteryApp};