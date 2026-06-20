use ignore::WalkBuilder;
use rayon::prelude::*;
use regex::Regex;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimationMode {
    /// Fast regex/heuristic based counting (1 token ~= 4 chars)
    Fast,
    /// Accurate counting simulating BPE subwords
    Accurate,
    /// Fast for large files, accurate for small
    Hybrid,
}

#[derive(Debug, Clone, Default)]
pub struct TokenCount {
    pub tokens: usize,
    pub chars: usize,
}

impl std::ops::Add for TokenCount {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            tokens: self.tokens + other.tokens,
            chars: self.chars + other.chars,
        }
    }
}

pub struct TokenEngine {
    mode: EstimationMode,
}

impl TokenEngine {
    pub fn new(mode: EstimationMode) -> Self {
        Self { mode }
    }

    /// Fast token estimation (roughly 4 chars = 1 token, or word-based)
    fn count_fast(text: &str) -> TokenCount {
        // A simple heuristic: Claude uses ~3.5 to 4 characters per token for English.
        // We'll use 3.5 for better safety margin, meaning text.len() / 3.5.
        // Plus adding tokens for whitespace boundaries to be a bit more robust.
        let tokens = (text.len() as f64 / 3.5).ceil() as usize;
        TokenCount {
            tokens,
            chars: text.len(),
        }
    }

    /// Accurate token estimation mimicking a BPE tokenizer
    fn count_accurate(text: &str) -> TokenCount {
        // A simplified BPE approximation without full dictionary loading.
        // Splits by words, punctuation, etc., and counts subwords based on length.
        let re = Regex::new(r"(\w+|[^\w\s]|\s+)").unwrap();
        let mut tokens = 0;
        for cap in re.captures_iter(text) {
            let chunk = &cap[0];
            if chunk.trim().is_empty() {
                // Whitespace is usually merged or 1 token
                tokens += 1;
            } else {
                // Approximate subword chunks for long words
                tokens += (chunk.len() as f64 / 4.0).ceil() as usize;
            }
        }
        TokenCount {
            tokens,
            chars: text.len(),
        }
    }

    pub fn count_text(&self, text: &str) -> TokenCount {
        match self.mode {
            EstimationMode::Fast => Self::count_fast(text),
            EstimationMode::Accurate => Self::count_accurate(text),
            EstimationMode::Hybrid => {
                if text.len() > 100_000 {
                    Self::count_fast(text)
                } else {
                    Self::count_accurate(text)
                }
            }
        }
    }

    /// Streaming tokenization reading from any `io::Read` source.
    pub fn count_stream<R: Read>(&self, mut reader: R) -> io::Result<TokenCount> {
        let mut buffer = [0; 8192];
        let mut total = TokenCount::default();

        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            // Note: In a real implementation we need to handle UTF-8 boundaries.
            // For now, we fallback to converting lossy.
            let chunk = String::from_utf8_lossy(&buffer[..n]);
            total = total + self.count_text(&chunk);
        }

        Ok(total)
    }

    /// Batch processing of multiple text strings.
    pub fn count_batch(&self, texts: &[String]) -> TokenCount {
        texts
            .iter()
            .map(|t| self.count_text(t))
            .fold(TokenCount::default(), |a, b| a + b)
    }

    /// Parallel batch processing of multiple text strings using Rayon.
    pub fn count_batch_parallel(&self, texts: &[String]) -> TokenCount {
        texts
            .par_iter()
            .map(|t| self.count_text(t))
            .reduce(TokenCount::default, |a, b| a + b)
    }

    /// Repository-wide token counting.
    /// Analyzes all text files in the given directory and its subdirectories.
    pub fn count_repository<P: AsRef<Path>>(&self, path: P) -> TokenCount {
        let walker = WalkBuilder::new(path).build();

        let files: Vec<_> = walker
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
            .map(|e| e.path().to_path_buf())
            .collect();

        files
            .par_iter()
            .filter_map(|p| fs::read_to_string(p).ok())
            .map(|content| self.count_text(&content))
            .reduce(TokenCount::default, |a, b| a + b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_estimation() {
        let engine = TokenEngine::new(EstimationMode::Fast);
        let res = engine.count_text("Hello world! This is a test.");
        assert!(res.tokens > 0);
    }

    #[test]
    fn test_accurate_estimation() {
        let engine = TokenEngine::new(EstimationMode::Accurate);
        let res = engine.count_text("Hello world! This is a test.");
        assert!(res.tokens > 0);
    }
}
