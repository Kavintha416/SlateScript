use crate::ast::Statement;
use crate::ast_interpreter::AstInterpreter;
use crate::value::Value;

/// An annotation that transforms the statement it precedes.
///
/// `#[name]` before a statement causes the interpreter to call
/// `Annotation::evaluate` with the following statement. The annotation
/// produces a value, which the interpreter binds to the name in the
/// inner statement.
pub trait Annotation: Send + Sync + 'static {
    /// The name used in `#[name]`. Lowercase by convention.
    fn name(&self) -> &str;

    /// Evaluate the annotated statement and produce a value.
    ///
    /// The interpreter calls this once for each `Statement::Annotated`.
    /// The `inner` statement has NOT yet been executed — the annotation
    /// owns the responsibility of evaluating it (typically via
    /// `interpreter.evaluate_expression`).
    fn evaluate(
        &self,
        inner: &Statement,
        interpreter: &mut AstInterpreter,
    ) -> Result<Value, String>;

    /// Clone plumbing. Trait objects can't derive `Clone`, so we hand
    /// it in via this method.
    fn clone_box(&self) -> Box<dyn Annotation>;

    /// Downcast plumbing.
    fn as_any(&self) -> &dyn std::any::Any;
}

impl Clone for Box<dyn Annotation> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}