use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    Array(Vec<Value>),
    Object(HashMap<String, Value>),

    /// A named struct value.
    ///
    /// `type_name` is derived from the binding name in the `make`
    /// statement that produced it. It is used for display and error
    /// messages, not type checking.
    Struct {
        type_name: String,
        fields: HashMap<String, Value>,
    },
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
            Value::Struct { fields, .. } => !fields.is_empty(),
            Value::Null => false,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(a) => {
                let items: Vec<String> = a.iter().map(|v| v.to_string()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Object(o) => {
                let items: Vec<String> = o.iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_string()))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::Struct { type_name, fields } => {
                let mut pairs: Vec<(&String, &Value)> = fields.iter().collect();
                pairs.sort_by(|a, b| a.0.cmp(b.0));
                let items: Vec<String> = pairs.iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_string()))
                    .collect();
                format!("{} {{ {} }}", type_name, items.join(", "))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ControlFlow {
    Continue,
    Return(Value),
    Break,
}

#[derive(Debug, Clone)]
pub struct Environment {
    variables: HashMap<String, Value>,
}

impl Environment {
    pub fn new() -> Self {
        Self { variables: HashMap::new() }
    }

    pub fn set(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.variables.get_mut(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }
}

impl From<HashMap<String, Value>> for Environment {
    fn from(variables: HashMap<String, Value>) -> Self {
        Self { variables }
    }
}