use ignore::WalkBuilder;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct RepositoryIntelligence;

#[derive(Debug, Default)]
pub struct OptimizationReport {
    pub total_files_analyzed: usize,
    pub oversized_files: Vec<String>,
    pub suspected_duplicates: Vec<String>,
}

impl Default for RepositoryIntelligence {
    fn default() -> Self {
        Self::new()
    }
}

impl RepositoryIntelligence {
    pub fn new() -> Self {
        Self
    }

    /// Analyze a repository to detect bloated files and exact duplicates.
    pub fn analyze_repository<P: AsRef<Path>>(&self, path: P) -> OptimizationReport {
        let walker = WalkBuilder::new(path).build();
        let mut report = OptimizationReport::default();
        let mut content_hashes = HashMap::new();

        for entry in walker.flatten() {
            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                report.total_files_analyzed += 1;

                if let Ok(content) = fs::read_to_string(entry.path()) {
                    // Check for oversized files (> 50KB for context files)
                    if content.len() > 50_000 {
                        report
                            .oversized_files
                            .push(entry.path().to_string_lossy().into_owned());
                    }

                    // Check for duplicates
                    // A simple hash using length + first 100 chars (not cryptographically secure, but works as heuristic)
                    let prefix = if content.len() > 100 {
                        &content[..100]
                    } else {
                        &content
                    };
                    let hash_key = format!("{}:{}", content.len(), prefix);

                    if let Some(existing) =
                        content_hashes.insert(hash_key, entry.path().to_string_lossy().into_owned())
                    {
                        report.suspected_duplicates.push(format!(
                            "{} == {}",
                            existing,
                            entry.path().display()
                        ));
                    }
                }
            }
        }

        report
    }
}
