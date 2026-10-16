// slattery/src/ui_integration.rs
//
// Glue between a parsed SlateScript AST and the egui renderer.
//
// Entry point: `UiFramework::render_ast(&program, path)`.
//
// Responsibilities:
//   1. Walk the AST and collect every `Expression::UiComponent` into a
//      component map keyed by identity (or a synthetic name for anonymous ones).
//   2. Walk the AST and register every `Statement::FunctionDefinition`
//      with the renderer so button handlers can be found by name.
//   3. Hand the populated renderer off to eframe.

use crate::egui_renderer::EguiRenderer;
use crate::sla_interpreter::{UiComponent, UiValue};
use slate_core::ast::{
    Expression, LiteralValue, Program, Statement, UiComponentType,
};
use slate_sfile::SFileExtension;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

pub struct UiFramework {
    sfile_extension: SFileExtension,
}

impl UiFramework {
    pub fn new() -> Self {
        Self {
            sfile_extension: SFileExtension::new(),
        }
    }

    // =================================================================
    //  Public entry point
    // =================================================================

    /// Render a SlateScript program that contains UI components.
    ///
    /// Caller is expected to have already parsed the source into `program`.
    /// `script_path` is used to resolve relative imports via sfile.
    pub fn render_ast(
        &mut self,
        program: &Program,
        script_path: Option<&Path>,
        icon: Option<egui::IconData>,
    ) -> Result<(), String> {
        if let Some(path) = script_path {
            self.sfile_extension.set_current_file(path);
        }

        // 1. Collect components.
        let mut components: HashMap<String, Rc<RefCell<UiComponent>>> = HashMap::new();
        for stmt in &program.statements {
            self.collect_components_from_statement(stmt, &mut components)?;
        }

        if components.is_empty() {
            return Err("Program parsed but contains no UI components".to_string());
        }

        // 2. Build renderer and register everything.
        let mut renderer = EguiRenderer::new();
        renderer.set_components(components);
        renderer.icon = icon;   // NEW

        for stmt in &program.statements {
            self.collect_styles(stmt, &mut renderer);
        }
        for stmt in &program.statements {
            self.collect_functions(stmt, &mut renderer);
        }

        // 3. Launch the app.
        crate::egui_renderer::run_egui_app_with_renderer(renderer)
            .map_err(|e| format!("render failed: {}", e))
    }

    // =================================================================
    //  Component collection
    // =================================================================

    fn collect_components_from_statement(
        &mut self,
        stmt: &Statement,
        out: &mut HashMap<String, Rc<RefCell<UiComponent>>>,
    ) -> Result<(), String> {
        match stmt {
            Statement::Assignment { name, value, .. } => {
                if let Some(comp) = self.convert_ast_expression(value)? {
                    out.insert(name.clone(), comp);
                }
            }
            Statement::Expression(e) => {
                if let Some(comp) = self.convert_ast_expression(e)? {
                    out.insert(format!("__anon_{}", out.len()), comp);
                }
            }
            Statement::FunctionDefinition { body, .. } => {
                for s in body {
                    self.collect_components_from_statement(s, out)?;
                }
            }
            Statement::If {
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                for s in then_branch {
                    self.collect_components_from_statement(s, out)?;
                }
                for (_, body) in elif_branches {
                    for s in body {
                        self.collect_components_from_statement(s, out)?;
                    }
                }
                if let Some(body) = else_branch {
                    for s in body {
                        self.collect_components_from_statement(s, out)?;
                    }
                }
            }
            Statement::While { body, .. } => {
                for s in body {
                    self.collect_components_from_statement(s, out)?;
                }
            }
            // Other statement kinds (Write, Return, Import, Train) don't
            // carry UI components at the statement level.
            _ => {}
        }
        Ok(())
    }

    /// Convert a `slate_core::ast::Expression::UiComponent` into a runtime
    /// `slattery::sla_interpreter::UiComponent`. Recurses into children.
    fn convert_ast_expression(
        &self,
        expr: &Expression,
    ) -> Result<Option<Rc<RefCell<UiComponent>>>, String> {
        let Expression::UiComponent {
            component_type,
            properties,
            children,
            events,
            ..
        } = expr
        else {
            return Ok(None);
        };

        let type_str = match component_type {
            UiComponentType::Window => "Window",
            UiComponentType::Column => "Column",
            UiComponentType::Row    => "Row",
            UiComponentType::Text   => "Text",
            UiComponentType::Button => "Button",
            UiComponentType::Input  => "Input",
        };

        let comp = Rc::new(RefCell::new(UiComponent::new(type_str.to_string())));

        // Properties
        for (k, v) in properties {
            let ui_val = convert_value(v);
            comp.borrow_mut().set_property(k.clone(), ui_val);
        }

        // Events
        for (k, handler) in events {
            comp.borrow_mut().set_event(k.clone(), handler.clone());
        }

        // Children (recursive)
        for child in children {
            if let Some(child_comp) = self.convert_ast_expression(child)? {
                comp.borrow_mut().add_child(child_comp);
            }
        }

        Ok(Some(comp))
    }

    // =================================================================
    //  Function registration
    // =================================================================

    /// Walk the AST and register every `func` definition with the renderer
    /// so that button `on_click` handlers can find them by name.
    fn collect_functions(&self, stmt: &Statement, renderer: &mut EguiRenderer) {
        match stmt {
            Statement::FunctionDefinition { name, body, .. } => {
                renderer.register_function(name, body.clone());
            }
            Statement::If {
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                for s in then_branch {
                    self.collect_functions(s, renderer);
                }
                for (_, body) in elif_branches {
                    for s in body {
                        self.collect_functions(s, renderer);
                    }
                }
                if let Some(body) = else_branch {
                    for s in body {
                        self.collect_functions(s, renderer);
                    }
                }
            }
            Statement::While { body, .. } => {
                for s in body {
                    self.collect_functions(s, renderer);
                }
            }
            _ => {}
        }
    }

    /// Walk the AST and feed every `Style:` extension node to the
    /// renderer's style engine.
    fn collect_styles(&self, stmt: &Statement, renderer: &mut EguiRenderer) {
        match stmt {
            Statement::Expression(Expression::Extension { name, type_name, .. }) => {
                if name == "slattery" && type_name.starts_with("Style:") {
                    if let Err(e) = renderer.load_style_from_encoded(type_name) {
                        renderer.logger.log_warn(&format!(
                            "Failed to load style: {}", e
                        ));
                    }
                }
            }
            Statement::If {
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                for s in then_branch {
                    self.collect_styles(s, renderer);
                }
                for (_, body) in elif_branches {
                    for s in body {
                        self.collect_styles(s, renderer);
                    }
                }
                if let Some(body) = else_branch {
                    for s in body {
                        self.collect_styles(s, renderer);
                    }
                }
            }
            Statement::While { body, .. } => {
                for s in body {
                    self.collect_styles(s, renderer);
                }
            }
            Statement::FunctionDefinition { body, .. } => {
                for s in body {
                    self.collect_styles(s, renderer);
                }
            }
            _ => {}
        }
    }
}

impl Default for UiFramework {
    fn default() -> Self {
        Self::new()
    }
}

// =====================================================================
//  Value conversion
// =====================================================================

/// Convert an AST expression into a `UiValue` for the runtime renderer.
///
/// Only literal and identifier values are supported. Anything more
/// complex (function calls, binary ops) resolves to `Null` for now —
/// those would require evaluating the expression, which the UI renderer
/// doesn't do. If you need computed property values later, run them
/// through `AstInterpreter` first.
fn convert_value(expr: &Expression) -> UiValue {
    match expr {
        Expression::Literal(LiteralValue::String(s), _)  => UiValue::String(s.clone()),
        Expression::Literal(LiteralValue::Number(n), _)  => UiValue::Number(*n),
        Expression::Literal(LiteralValue::Boolean(b), _) => UiValue::Boolean(*b),
        Expression::Literal(LiteralValue::Null, _)       => UiValue::Null,
        Expression::Variable(name, _)                    => UiValue::String(name.clone()),
        _ => UiValue::Null,
    }
}

// =====================================================================
//  Style file discovery
// =====================================================================

/// Collect `.sts` style files from the current directory, a `styles/`
/// subdirectory, and `src/`.
pub fn collect_style_files(_base_dir: Option<&Path>) -> Vec<String> {
    let mut files = Vec::new();

    let search_dirs = [".", "styles", "src"];
    for dir in search_dirs {
        let path = Path::new(dir);
        if !path.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("sts") {
                    files.push(p.to_string_lossy().to_string());
                }
            }
        }
    }

    files
}