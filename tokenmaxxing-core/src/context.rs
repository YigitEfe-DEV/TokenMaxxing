use crate::compressor::ContextCompressor;
use std::collections::HashSet;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextScore {
    pub overall_score: f64,
    pub quality_score: f64,
    pub noise_ratio: f64,
    pub token_density: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RankedChunk {
    pub chunk: String,
    pub rank: usize,
    pub score: f64,
    pub original_size: usize,
    pub reason: String,
}

pub struct ContextMaxxingEngine;

impl Default for ContextMaxxingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextMaxxingEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn score_context(&self, text: &str) -> ContextScore {
        if text.is_empty() {
            return ContextScore {
                overall_score: 0.0,
                quality_score: 0.0,
                noise_ratio: 0.0,
                token_density: 0.0,
            };
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        let word_count = words.len();

        let unique_words: HashSet<&str> = words
            .iter()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .collect();
        let unique_word_count = unique_words.len();

        let repeat_ratio = if word_count > 0 {
            1.0 - (unique_word_count as f64 / word_count as f64)
        } else {
            0.0
        };

        // Boilerplate/noise check
        let lowercase_text = text.to_lowercase();
        let boilerplate_indicators = ["license", "copyright", "all rights reserved", "boilerplate"];
        let mut noise_matches = 0;
        for indicator in &boilerplate_indicators {
            if lowercase_text.contains(indicator) {
                noise_matches += 1;
            }
        }

        let noise_ratio = (noise_matches as f64 / 10.0).min(1.0);
        let quality_score = ((1.0 - repeat_ratio) * 100.0 - (noise_ratio * 20.0)).clamp(0.0, 100.0);

        let token_density = if !text.is_empty() {
            (word_count as f64 / text.len() as f64) * 10.0 // higher means shorter, denser words
        } else {
            0.0
        };

        let overall_score = (quality_score * 0.7 + token_density * 3.0).clamp(0.0, 100.0);

        ContextScore {
            overall_score,
            quality_score,
            noise_ratio,
            token_density,
        }
    }

    pub fn rank_chunks(&self, text: &str) -> Vec<RankedChunk> {
        let chunks = ContextCompressor::semantic_chunking(text);
        let mut ranked = Vec::new();

        for (idx, chunk) in chunks.iter().enumerate() {
            let score_info = self.score_context(chunk);
            let mut reason = "Standard content".to_string();
            let mut multiplier = 1.0;

            // Simple rules to prioritize chunks
            if chunk.contains("fn ")
                || chunk.contains("pub ")
                || chunk.contains("class ")
                || chunk.contains("interface ")
            {
                reason = "Contains code declarations".to_string();
                multiplier = 1.3;
            } else if chunk.to_lowercase().contains("todo") || chunk.to_lowercase().contains("fix")
            {
                reason = "Contains active tasks/TODOs".to_string();
                multiplier = 1.2;
            } else if chunk.to_lowercase().contains("license")
                || chunk.to_lowercase().contains("copyright")
            {
                reason = "Boilerplate license text".to_string();
                multiplier = 0.3;
            }

            ranked.push(RankedChunk {
                chunk: chunk.to_string(),
                rank: idx,
                score: (score_info.overall_score * multiplier).min(100.0),
                original_size: chunk.len(),
                reason,
            });
        }

        // Sort descending by score
        ranked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Re-assign ranks based on sorting
        for (new_idx, chunk) in ranked.iter_mut().enumerate() {
            chunk.rank = new_idx + 1;
        }

        ranked
    }

    pub fn detect_duplicates(&self, text: &str) -> Vec<String> {
        let chunks = ContextCompressor::semantic_chunking(text);
        let mut seen = HashSet::new();
        let mut duplicates = Vec::new();

        for chunk in chunks {
            let normalized = chunk.trim().to_lowercase();
            if normalized.len() > 15 && !seen.insert(normalized) {
                duplicates.push(chunk.to_string());
            }
        }

        duplicates
    }

    pub fn detect_dead_context(&self, text: &str) -> Vec<String> {
        let chunks = ContextCompressor::semantic_chunking(text);
        let mut dead = Vec::new();

        for chunk in chunks {
            let score_info = self.score_context(chunk);
            // Low quality + high noise = dead context
            if score_info.quality_score < 40.0 && score_info.noise_ratio > 0.2 {
                dead.push(chunk.to_string());
            }
        }

        dead
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_scoring() {
        let engine = ContextMaxxingEngine::new();
        let score = engine.score_context("Important coding patterns go here.");
        assert!(score.overall_score > 0.0);
    }

    #[test]
    fn test_chunk_ranking() {
        let engine = ContextMaxxingEngine::new();
        let text = "Boilerplate copyright license\n\npub fn important_code() {}\n\nrandom stuff";
        let ranked = engine.rank_chunks(text);
        assert!(!ranked.is_empty());
        // Code should be ranked higher than boilerplate
        let code_chunk = ranked.iter().find(|r| r.reason.contains("code")).unwrap();
        let license_chunk = ranked
            .iter()
            .find(|r| r.reason.contains("Boilerplate"))
            .unwrap();
        assert!(code_chunk.score > license_chunk.score);
    }
}
