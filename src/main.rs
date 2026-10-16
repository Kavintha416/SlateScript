// slate/src/main.rs
//
// Top-level CLI. The only place that knows how to wire everything together.
//
// Responsibilities:
//   • Parse argv
//   • Build the extension registry
//   • Lex + parse the source file
//   • Decide: UI program or script program?
//   • Dispatch to slattery or the AST interpreter

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use slate_core::extension::ExtensionRegistry;
use slate_core::ast::{Expression, Program, Statement};
use slate_core::{Lexer, Parser};
use slate_sfile::SFileExtension;
use slattery::{SlatteryExtension, UiFramework};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("usage: slate <file.st>");
        return ExitCode::from(1);
    }

    match run(Path::new(&args[1])) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {}", msg);
            ExitCode::from(1)
        }
    }
}

fn run(path: &Path) -> Result<(), String> {
    // 1. Read source
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read '{}': {}", path.display(), e))?;

    // 2. Build registry
    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(SFileExtension::new()));
    registry.register(Box::new(SlatteryExtension::new()));

    // 3. Lex
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize().map_err(|e| e.to_string())?;

    // 4. Parse
    let mut parser = Parser::new(tokens, source.clone(), registry);
    let program = parser.parse().map_err(|e| e.to_string())?;

    // 5. Decide: UI or script?
    if program_has_ui(&program) {
        run_ui(program, path)
    } else {
        run_script(program, parser.take_extensions())
    }
}

/// True if the AST contains any `Expression::UiComponent` anywhere.
/// Walks every statement recursively.
fn program_has_ui(program: &Program) -> bool {
    program.statements.iter().any(statement_has_ui)
}

fn statement_has_ui(stmt: &Statement) -> bool {
    match stmt {
        Statement::Assignment { value, .. } => expression_has_ui(value),
        Statement::Expression(e) => expression_has_ui(e),
        Statement::FunctionDefinition { body, .. } => body.iter().any(statement_has_ui),
        Statement::Annotated { inner, .. } => statement_has_ui(inner),
        Statement::PropertyAssignment { target, value, .. } => {
            expression_has_ui(target) || expression_has_ui(value)
        }
        Statement::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
            ..
        } => {
            expression_has_ui(condition)
                || then_branch.iter().any(statement_has_ui)
                || elif_branches.iter().any(|(c, b)| {
                    expression_has_ui(c) || b.iter().any(statement_has_ui)
                })
                || else_branch
                    .as_ref()
                    .map_or(false, |b| b.iter().any(statement_has_ui))
        }
        Statement::While { condition, body, .. } => {
            expression_has_ui(condition) || body.iter().any(statement_has_ui)
        }
        Statement::Write { value, .. } => expression_has_ui(value),
        Statement::Return { value, .. } => {
            value.as_ref().map_or(false, expression_has_ui)
        }
        Statement::Train { model, dataset, config, .. } => {
            expression_has_ui(model)
                || expression_has_ui(dataset)
                || config.values().any(expression_has_ui)
        }
        Statement::ImportStatement { .. } => false,
    }
}

fn expression_has_ui(expr: &Expression) -> bool {
    match expr {
        Expression::UiComponent { children, .. } => {
            // Recurse into children so `<Window>{ Child: <Text>{} }` is caught.
            // The `true` short-circuits: if this node IS a UiComponent we
            // already know the answer.
            let _ = children;
            true
        }
        Expression::UiRender { .. } => true,
        Expression::Binary { left, right, .. } => {
            expression_has_ui(left) || expression_has_ui(right)
        }
        Expression::FunctionCall { arguments, .. } => {
            arguments.iter().any(expression_has_ui)
        }
        Expression::Array(elements, _) => elements.iter().any(expression_has_ui),
        Expression::Object(props, _) => props.values().any(expression_has_ui),
        Expression::PropertyAccess { object, .. } => expression_has_ui(object),
        Expression::MethodCall { object, arguments, .. } => {
            expression_has_ui(object) || arguments.iter().any(expression_has_ui)
        }
        Expression::Make { properties, children, .. } => {
            properties.values().any(expression_has_ui)
                || children.iter().any(expression_has_ui)
        }
        _ => false,
    }
}

fn run_ui(program: Program, path: &Path) -> Result<(), String> {
    let mut framework = UiFramework::new();
    framework.render_ast(&program, Some(path))
}

fn run_script(
    program: Program,
    extensions: ExtensionRegistry,
) -> Result<(), String> {
    let mut interp = slate_core::AstInterpreter::new();
    interp.set_extensions(extensions);
    interp.interpret(&program).map_err(|e| e.to_string())?;
    Ok(())
}