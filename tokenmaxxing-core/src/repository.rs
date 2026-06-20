use ignore::WalkBuilder;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize)]
pub struct WasteReport {
    pub total_waste_bytes: usize,
    pub generated_files: Vec<PathBuf>,
    pub build_artifacts: Vec<PathBuf>,
    pub cache_files: Vec<PathBuf>,
    pub vendor_directories: Vec<PathBuf>,
    pub duplicates: Vec<(PathBuf, PathBuf)>,
    pub oversized_files: Vec<PathBuf>,
    pub efficiency_score: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RepositoryMemory {
    pub architecture_summary: String,
    pub component_summary: Vec<String>,
    pub directory_summary: HashMap<String, usize>,
    pub dependency_summary: Vec<String>,
}

pub struct SmartRepositoryMode;

impl Default for SmartRepositoryMode {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartRepositoryMode {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_waste(&self, path: &Path) -> WasteReport {
        let walker = WalkBuilder::new(path).build();
        let mut generated_files = Vec::new();
        let mut build_artifacts = Vec::new();
        let mut cache_files = Vec::new();
        let mut vendor_directories = Vec::new();
        let mut oversized_files = Vec::new();
        let mut file_contents = HashMap::new();
        let mut duplicates = Vec::new();
        let mut total_waste_bytes = 0;
        let mut total_repo_bytes = 0;

        for entry in walker.flatten() {
            let path_buf = entry.path().to_path_buf();
            if path_buf.is_file() {
                let metadata = match fs::metadata(&path_buf) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let size = metadata.len() as usize;
                total_repo_bytes += size;

                let path_str = path_buf.to_string_lossy().to_lowercase();

                // Check generated files
                if path_str.contains(".min.js")
                    || path_str.contains(".min.css")
                    || path_str.contains("package-lock.json")
                    || path_str.contains("yarn.lock")
                    || path_str.contains("pnpm-lock.yaml")
                {
                    generated_files.push(path_buf.clone());
                    total_waste_bytes += size;
                }
                // Check build artifacts
                else if path_str.contains("/target/")
                    || path_str.contains("/dist/")
                    || path_str.contains("/build/")
                    || path_str.contains("/out/")
                {
                    build_artifacts.push(path_buf.clone());
                    total_waste_bytes += size;
                }
                // Check cache / temp
                else if path_str.contains("/.cache/")
                    || path_str.contains("/.tmp/")
                    || path_str.contains("/tmp/")
                    || path_str.ends_with(".tmp")
                    || path_str.ends_with('~')
                {
                    cache_files.push(path_buf.clone());
                    total_waste_bytes += size;
                }
                // Check vendor / dependency
                else if path_str.contains("/node_modules/")
                    || path_str.contains("/vendor/")
                    || path_str.contains("/.cargo/")
                {
                    vendor_directories.push(path_buf.clone());
                    total_waste_bytes += size;
                }
                // Check oversized (> 50KB)
                else if size > 50_000 {
                    oversized_files.push(path_buf.clone());
                }

                // Check duplicate files
                if let Ok(content) = fs::read_to_string(&path_buf) {
                    let hash = format!(
                        "{}:{}",
                        content.len(),
                        if content.len() > 100 {
                            &content[..100]
                        } else {
                            &content
                        }
                    );
                    if let Some(existing) = file_contents.insert(hash, path_buf.clone()) {
                        duplicates.push((existing.clone(), path_buf.clone()));
                        total_waste_bytes += size;
                    }
                }
            }
        }

        let efficiency_score = if total_repo_bytes > 0 {
            ((1.0 - (total_waste_bytes as f64 / total_repo_bytes as f64)) * 100.0).max(0.0)
        } else {
            100.0
        };

        WasteReport {
            total_waste_bytes,
            generated_files,
            build_artifacts,
            cache_files,
            vendor_directories,
            duplicates,
            oversized_files,
            efficiency_score,
        }
    }

    pub fn generate_memory(&self, path: &Path) -> RepositoryMemory {
        let walker = WalkBuilder::new(path).build();
        let mut directory_summary = HashMap::new();
        let mut components = HashSet::new();
        let mut dependencies = HashSet::new();

        for entry in walker.flatten() {
            let path_buf = entry.path().to_path_buf();
            if path_buf.is_file() {
                // Directory tally
                if let Some(parent) = path_buf.parent() {
                    let dir_name = parent.to_string_lossy().to_string();
                    let count = directory_summary.entry(dir_name).or_insert(0);
                    *count += 1;
                }

                // Component detection
                let file_name = path_buf
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if file_name.ends_with(".rs") {
                    components.insert("Rust Source Module".to_string());
                } else if file_name.ends_with(".json") {
                    components.insert("JSON Configuration".to_string());
                } else if file_name.ends_with(".js") || file_name.ends_with(".ts") {
                    components.insert("JavaScript/TypeScript Module".to_string());
                }

                // Dependency detection
                if file_name == "Cargo.toml" {
                    if let Ok(content) = fs::read_to_string(&path_buf) {
                        for line in content.lines() {
                            if line.contains(" = ") && !line.starts_with('[') {
                                let parts: Vec<&str> = line.split('=').collect();
                                dependencies.insert(parts[0].trim().to_string());
                            }
                        }
                    }
                }
            }
        }

        let mut component_summary: Vec<String> = components.into_iter().collect();
        component_summary.sort();
        let mut dependency_summary: Vec<String> = dependencies.into_iter().collect();
        dependency_summary.sort();

        RepositoryMemory {
            architecture_summary: format!(
                "Repository holds {} directories and features {} components.",
                directory_summary.len(),
                component_summary.len()
            ),
            component_summary,
            directory_summary,
            dependency_summary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waste_analysis() {
        let smart = SmartRepositoryMode::new();
        let report = smart.analyze_waste(Path::new("."));
        assert!(report.efficiency_score <= 100.0);
    }

    #[test]
    fn test_repository_memory() {
        let smart = SmartRepositoryMode::new();
        let memory = smart.generate_memory(Path::new("."));
        assert!(!memory.architecture_summary.is_empty());
    }
}
