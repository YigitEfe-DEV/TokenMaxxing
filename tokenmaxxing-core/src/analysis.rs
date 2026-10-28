//! Repository level token analysis.
//!
//! The token engine counts a whole repository in one go. A dashboard needs to
//! know how the total is spread over the files, so the per file counts live
//! here instead of in the user interface.

use crate::engine::{text_files, TokenCount, TokenEngine};
use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// The token count of a single file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileTokenCount {
    /// Path of the file, relative to the scanned root when possible.
    pub path: PathBuf,
    /// The count of the file.
    pub count: TokenCount,
}

impl FileTokenCount {
    /// The size of the file in bytes.
    pub fn bytes(&self) -> usize {
        self.count.bytes
    }
}

/// Counts every readable text file of a tree.
///
/// Files that cannot be read as UTF-8 are skipped, because no tokenizer of
/// this crate can count them.
pub fn count_files<P: AsRef<Path>>(engine: &TokenEngine, root: P) -> Vec<FileTokenCount> {
    let root = root.as_ref();

    text_files(root)
        .par_iter()
        .filter_map(|path| {
            let content = std::fs::read_to_string(path).ok()?;
            Some(FileTokenCount {
                path: relative_path(root, path),
                count: engine.count_text(&content),
            })
        })
        .collect()
}

/// The path of a file relative to the scanned root.
fn relative_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

/// Sums the counts of a list of files.
pub fn total_of(files: &[FileTokenCount]) -> TokenCount {
    files
        .iter()
        .fold(TokenCount::default(), |sum, file| sum + file.count.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EstimationMode;

    #[test]
    fn test_empty_repository_has_no_files() {
        let directory = std::env::temp_dir().join("tokenmaxxing-analysis-empty");
        std::fs::create_dir_all(&directory).unwrap();

        let files = count_files(&TokenEngine::new(EstimationMode::Fast), &directory);

        assert!(files.is_empty());
        assert_eq!(total_of(&files).tokens, 0);
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn test_files_are_counted_relative_to_the_root() {
        let directory = std::env::temp_dir().join("tokenmaxxing-analysis-relative");
        std::fs::create_dir_all(directory.join("sub")).unwrap();
        std::fs::write(directory.join("sub/one.txt"), "hello world").unwrap();

        let files = count_files(&TokenEngine::new(EstimationMode::Fast), &directory);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, PathBuf::from("sub/one.txt"));
        assert!(files[0].count.tokens > 0);

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn test_total_matches_the_sum_of_the_files() {
        let directory = std::env::temp_dir().join("tokenmaxxing-analysis-total");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("a.txt"), "one two three").unwrap();
        std::fs::write(directory.join("b.txt"), "four five").unwrap();

        let engine = TokenEngine::new(EstimationMode::Fast);
        let files = count_files(&engine, &directory);
        let total = total_of(&files);

        assert_eq!(total.tokens, engine.count_repository(&directory).tokens);
        assert_eq!(total.chars, 13 + 9);

        std::fs::remove_dir_all(&directory).ok();
    }
}
