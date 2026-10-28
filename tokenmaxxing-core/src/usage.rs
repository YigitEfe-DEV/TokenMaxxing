//! Provider neutral usage records.
//!
//! A dashboard shows two very different things:
//!
//! * the tokens of the files in a repository, which is what the token engine
//!   measures, and
//! * the tokens a provider actually billed for a request, which is usage.
//!
//! Only the second one is usage. This module describes the interface for it
//! without pretending that an integration exists: the default store has no
//! records at all, and a file store keeps only the minimum a chart needs.
//!
//! Nothing here stores API keys, prompts or file contents.

use std::path::{Path, PathBuf};

/// A usage record of a provider request.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UsageRecord {
    /// Unix timestamp in seconds.
    pub timestamp: i64,
    /// Provider that reported the record.
    pub provider: String,
    /// Model the request went to.
    pub model: String,
    /// Tokens that were sent to the model.
    pub input_tokens: usize,
    /// Tokens the model produced.
    pub output_tokens: usize,
    /// Tokens that were served from the prompt cache.
    #[serde(default)]
    pub cached_input_tokens: usize,
    /// Tokens that were written into the prompt cache.
    #[serde(default)]
    pub cache_write_tokens: usize,
    /// Cost the provider reported, in the currency of the record.
    #[serde(default)]
    pub cost: Option<f64>,
    /// Currency of the reported cost.
    #[serde(default)]
    pub currency: Option<String>,
}

impl UsageRecord {
    /// The total number of tokens of the record.
    pub fn total_tokens(&self) -> usize {
        self.input_tokens + self.output_tokens
    }
}

/// Errors of a usage store.
#[derive(Debug, thiserror::Error)]
pub enum UsageError {
    /// The records could not be read.
    #[error("the usage history could not be read: {0}")]
    Unreadable(String),
    /// The records could not be written.
    #[error("the usage history could not be written: {0}")]
    Unwritable(String),
}

/// A source of usage records.
pub trait UsageStore: Send + Sync {
    /// Identifier of the store, for example `file` or `none`.
    fn name(&self) -> &str;
    /// The records, oldest first.
    fn records(&self) -> Result<Vec<UsageRecord>, UsageError>;
    /// Whether the store can hold new records.
    fn is_writable(&self) -> bool {
        false
    }
}

/// A store without any records.
///
/// This is the default, so that a dashboard never invents usage numbers.
pub struct EmptyUsageStore;

impl UsageStore for EmptyUsageStore {
    fn name(&self) -> &str {
        "none"
    }

    fn records(&self) -> Result<Vec<UsageRecord>, UsageError> {
        Ok(Vec::new())
    }
}

/// A store that keeps the records in a JSON lines file.
///
/// Only the fields of a [`UsageRecord`] are written, so no prompt, no file
/// content and no credential ends up on disk.
pub struct FileUsageStore {
    path: PathBuf,
}

impl FileUsageStore {
    /// Builds a store for a file.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The file the records are kept in.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The default location of the usage history.
    ///
    /// It follows the platform conventions: `XDG_DATA_HOME` on Linux and
    /// `Library/Application Support` on macOS.
    pub fn default_path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;

        if cfg!(target_os = "macos") {
            Some(
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("tokenmaxxing")
                    .join("usage.jsonl"),
            )
        } else {
            let data = std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(&home).join(".local").join("share"));
            Some(data.join("tokenmaxxing").join("usage.jsonl"))
        }
    }

    /// Appends a record to the file.
    pub fn append(&self, record: &UsageRecord) -> Result<(), UsageError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                UsageError::Unwritable(format!("{}: {error}", parent.display()))
            })?;
        }

        let mut line = serde_json::to_string(record)
            .map_err(|error| UsageError::Unwritable(error.to_string()))?;
        line.push('\n');

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| UsageError::Unwritable(format!("{}: {error}", self.path.display())))?;
        file.write_all(line.as_bytes())
            .map_err(|error| UsageError::Unwritable(format!("{}: {error}", self.path.display())))
    }
}

impl UsageStore for FileUsageStore {
    fn name(&self) -> &str {
        "file"
    }

    fn records(&self) -> Result<Vec<UsageRecord>, UsageError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&self.path)
            .map_err(|error| UsageError::Unreadable(format!("{}: {error}", self.path.display())))?;

        let mut records = Vec::new();
        for (number, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let record: UsageRecord = serde_json::from_str(line).map_err(|error| {
                UsageError::Unreadable(format!(
                    "{}: line {}: {error}",
                    self.path.display(),
                    number + 1
                ))
            })?;
            records.push(record);
        }

        records.sort_by_key(|record| record.timestamp);
        Ok(records)
    }

    fn is_writable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(timestamp: i64, input: usize) -> UsageRecord {
        UsageRecord {
            timestamp,
            provider: "anthropic".to_string(),
            model: "claude-opus-5-5".to_string(),
            input_tokens: input,
            output_tokens: 10,
            cached_input_tokens: 0,
            cache_write_tokens: 0,
            cost: Some(0.01),
            currency: Some("USD".to_string()),
        }
    }

    #[test]
    fn test_the_empty_store_has_no_records() {
        let store = EmptyUsageStore;

        assert_eq!(store.name(), "none");
        assert!(store.records().unwrap().is_empty());
        assert!(!store.is_writable());
    }

    #[test]
    fn test_records_are_read_back_in_order() {
        let path = std::env::temp_dir().join("tokenmaxxing-usage-order.jsonl");
        std::fs::remove_file(&path).ok();

        let store = FileUsageStore::new(&path);
        store.append(&record(200, 20)).unwrap();
        store.append(&record(100, 10)).unwrap();

        let records = store.records().unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].timestamp, 100);
        assert_eq!(records[1].timestamp, 200);
        assert_eq!(records[0].total_tokens(), 20);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_a_missing_file_is_an_empty_history() {
        let path = std::env::temp_dir().join("tokenmaxxing-usage-missing.jsonl");
        std::fs::remove_file(&path).ok();

        let store = FileUsageStore::new(&path);

        assert!(store.records().unwrap().is_empty());
    }

    #[test]
    fn test_broken_lines_are_reported() {
        let path = std::env::temp_dir().join("tokenmaxxing-usage-broken.jsonl");
        std::fs::write(&path, "not json\n").unwrap();

        let store = FileUsageStore::new(&path);
        let error = store.records().unwrap_err();

        assert!(matches!(error, UsageError::Unreadable(_)));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_the_default_path_follows_the_platform_convention() {
        let path = FileUsageStore::default_path();

        // Only meaningful when a home directory is available.
        if let Some(path) = path {
            assert!(path.ends_with("tokenmaxxing/usage.jsonl"));
        }
    }
}
