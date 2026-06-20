use std::collections::HashSet;

pub struct ContextCompressor;

impl ContextCompressor {
    pub fn new() -> Self {
        Self
    }

    /// Split text into semantic chunks (e.g., paragraphs or code blocks)
    pub fn semantic_chunking(text: &str) -> Vec<&str> {
        // Very basic chunking by double newlines (paragraphs)
        text.split("\n\n").filter(|s| !s.trim().is_empty()).collect()
    }

    /// Detect and remove duplicate chunks
    pub fn remove_duplicate_chunks(chunks: Vec<&str>) -> Vec<&str> {
        let mut seen = HashSet::new();
        let mut unique = Vec::new();

        for chunk in chunks {
            let normalized = chunk.trim().to_lowercase();
            // Allow small identical chunks like "}" or "end" but deduplicate large paragraphs
            if normalized.len() > 10 {
                if seen.insert(normalized) {
                    unique.push(chunk);
                }
            } else {
                unique.push(chunk);
            }
        }
        unique
    }

    /// Simple heuristic to remove dead context
    /// (e.g. chunks that look like generated licenses or generic boilerplate)
    pub fn remove_dead_context<'a>(chunks: Vec<&'a str>) -> Vec<&'a str> {
        let boilerplate_keywords = ["permission is hereby granted", "mit license", "apache license"];
        
        chunks.into_iter().filter(|chunk| {
            let lower = chunk.to_lowercase();
            !boilerplate_keywords.iter().any(|&kw| lower.contains(kw))
        }).collect()
    }

    /// Compress text by combining chunks after cleaning
    pub fn compress(&self, text: &str) -> String {
        let chunks = Self::semantic_chunking(text);
        let unique = Self::remove_duplicate_chunks(chunks);
        let alive = Self::remove_dead_context(unique);
        
        alive.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressor_deduplication() {
        let text = "Block A is long enough\n\nBlock B\n\nBlock A is long enough";
        let compressor = ContextCompressor::new();
        let compressed = compressor.compress(text);
        assert_eq!(compressed, "Block A is long enough\n\nBlock B");
    }

    #[test]
    fn test_remove_boilerplate() {
        let text = "Useful code here.\n\nMIT License Permission is hereby granted...\n\nMore code.";
        let compressor = ContextCompressor::new();
        let compressed = compressor.compress(text);
        assert_eq!(compressed, "Useful code here.\n\nMore code.");
    }
}
