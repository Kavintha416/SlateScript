use std::collections::HashMap;
use super::trait_def::Annotation;

/// Holds every registered annotation.
///
/// Builtins are added by `new()`. Extensions will be able to add their
/// own in a future commit via `LanguageExtension::annotations()`.
pub struct AnnotationRegistry {
    annotations: HashMap<String, Box<dyn Annotation>>,
}

impl Default for AnnotationRegistry {
    fn default() -> Self {
        Self {
            annotations: HashMap::new(),
        }
    }
}

impl Clone for AnnotationRegistry {
    fn clone(&self) -> Self {
        let mut annotations = HashMap::new();
        for (name, a) in &self.annotations {
            annotations.insert(name.clone(), a.clone_box());
        }
        Self { annotations }
    }
}

impl AnnotationRegistry {
    /// Create a registry pre-populated with the built-in annotations.
    pub fn new() -> Self {
        let mut reg = Self::default();
        reg.register_builtins();
        reg
    }

    /// Register an annotation. If an annotation with the same name
    /// already exists, the new one replaces it.
    pub fn register(&mut self, annotation: Box<dyn Annotation>) {
        self.annotations.insert(annotation.name().to_string(), annotation);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Annotation> {
        self.annotations.get(name).map(|b| b.as_ref())
    }

    pub fn has(&self, name: &str) -> bool {
        self.annotations.contains_key(name)
    }

    pub fn names(&self) -> Vec<&str> {
        self.annotations.keys().map(|s| s.as_str()).collect()
    }

    fn register_builtins(&mut self) {
        self.register(Box::new(super::builtin::StructAnnotation));
    }
}