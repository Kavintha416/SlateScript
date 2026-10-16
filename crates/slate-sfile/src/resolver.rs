// slate-sfile/src/resolver.rs

use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct PathResolver {
    include_paths: Vec<PathBuf>,
}

impl PathResolver {
    pub fn new() -> Self {
        Self {
            include_paths: Vec::new(),
        }
    }

    pub fn add_include_path(&mut self, path: PathBuf) {
        self.include_paths.push(path);
    }

    pub fn resolve(&self, path: &Path, base_dir: &Path) -> Result<PathBuf, String> {
        // If path is absolute, use it directly
        if path.is_absolute() {
            if path.exists() {
                return Ok(path.to_path_buf());
            }
            return Err(format!("File not found: {}", path.display()));
        }

        // Try relative to base_dir first
        let relative_to_base = base_dir.join(path);
        if relative_to_base.exists() {
            return Ok(relative_to_base);
        }

        // Try with .st extension if not present
        if path.extension().is_none() {
            let with_ext = base_dir.join(path).with_extension("st");
            if with_ext.exists() {
                return Ok(with_ext);
            }
        }

        // Try include paths
        for include_path in &self.include_paths {
            let candidate = include_path.join(path);
            if candidate.exists() {
                return Ok(candidate);
            }
            
            // Try with .st extension
            if path.extension().is_none() {
                let candidate_with_ext = include_path.join(path).with_extension("st");
                if candidate_with_ext.exists() {
                    return Ok(candidate_with_ext);
                }
            }
        }

        Err(format!(
            "File '{}' not found. Tried:\n  - {}\n  - {}\n  - Include paths",
            path.display(),
            relative_to_base.display(),
            base_dir.join(path).with_extension("st").display()
        ))
    }
}