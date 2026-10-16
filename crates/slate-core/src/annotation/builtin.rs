use super::trait_def::Annotation;
use crate::ast::Statement;
use crate::ast_interpreter::AstInterpreter;
use crate::value::Value;

/// `#[struct]` — turns
///
///     #[struct]
///     make name { field: value, ... }
///
/// into a `Value::Struct` with `type_name` derived from the binding
/// name.
///
/// If the inner statement isn't an assignment, or if the value doesn't
/// evaluate to `Value::Object`, the annotation fails.
pub struct StructAnnotation;

impl Annotation for StructAnnotation {
    fn name(&self) -> &str {
        "struct"
    }

    fn evaluate(
        &self,
        inner: &Statement,
        interpreter: &mut AstInterpreter,
    ) -> Result<Value, String> {
        let (name, value_expr) = match inner {
            Statement::Assignment { name, value, .. } => (name, value),
            _ => {
                return Err(
                    "#[struct] must precede a `make` statement.\n  \
                     Example:\n    \
                     #[struct]\n    \
                     make point { x: 0, y: 0 }"
                        .to_string(),
                );
            }
        };

        let evaluated = interpreter.evaluate_expression(value_expr)?;

        match evaluated {
            Value::Object(fields) => Ok(Value::Struct {
                type_name: name.clone(),
                fields,
            }),
            other => Err(format!(
                "#[struct] on `make {}` expects an object value, got `{}`",
                name,
                other.to_string()
            )),
        }
    }

    fn clone_box(&self) -> Box<dyn Annotation> {
        Box::new(StructAnnotation)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}