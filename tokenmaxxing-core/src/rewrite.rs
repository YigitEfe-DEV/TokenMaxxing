use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewriteMode {
    Conservative,
    Balanced,
    Aggressive,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RewriteReport {
    pub original_size: usize,
    pub optimized_size: usize,
    pub reduction_percentage: f64,
}

pub struct PromptRewriter;

impl Default for PromptRewriter {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptRewriter {
    pub fn new() -> Self {
        Self
    }

    pub fn rewrite(&self, text: &str, mode: RewriteMode) -> (String, RewriteReport) {
        let original_size = text.len();
        let mut rewritten = text.to_string();

        // 1. Common politeness filters (Applied in all modes)
        let polite_patterns = [
            (r"(?i)\bplease\s+", ""),
            (r"(?i)\bkindly\s+", ""),
            (r"(?i)\bcarefully\s+", ""),
            (r"(?i)\bwould\s+you\s+mind\s+doing\s+a\s+", ""),
            (r"(?i)\bthank\s+you\s+in\s+advance\b", ""),
            (
                r"(?i)\bi\s+would\s+appreciate\s+it\s+if\s+you\s+could\s+",
                "",
            ),
        ];

        for (pattern, replacement) in &polite_patterns {
            if let Ok(re) = Regex::new(pattern) {
                rewritten = re.replace_all(&rewritten, *replacement).into_owned();
            }
        }

        // Mode specific refinements
        match mode {
            RewriteMode::Conservative => {
                // Just politeness removal
            }
            RewriteMode::Balanced => {
                // Remove helper verbs & conversational filler
                let filler_patterns = [
                    (r"(?i)\bhelp\s+me\s+to\s+analyze\b", "analyze"),
                    (r"(?i)\bi\s+want\s+you\s+to\s+check\b", "check"),
                    (r"(?i)\band\s+tell\s+me\s+all\s+the\b", "reporting all"),
                    (r"(?i)\bcan\s+you\s+please\s+explain\b", "explain"),
                ];
                for (pattern, replacement) in &filler_patterns {
                    if let Ok(re) = Regex::new(pattern) {
                        rewritten = re.replace_all(&rewritten, *replacement).into_owned();
                    }
                }
            }
            RewriteMode::Aggressive => {
                // Very direct command transformation
                let aggressive_patterns = [
                    (r"(?i)\bi\s+am\s+looking\s+for\s+a\s+way\s+to\b", "need:"),
                    (
                        r"(?i)\bmake\s+sure\s+that\s+you\s+check\s+all\b",
                        "check all",
                    ),
                    (
                        r"(?i)\banalyze\s+the\s+repository\s+and\s+tell\s+me\s+all\s+problems\b",
                        "Analyze repository. Report all issues.",
                    ),
                ];
                for (pattern, replacement) in &aggressive_patterns {
                    if let Ok(re) = Regex::new(pattern) {
                        rewritten = re.replace_all(&rewritten, *replacement).into_owned();
                    }
                }
            }
        }

        let optimized_size = rewritten.len();
        let reduction_percentage = if original_size > 0 {
            ((original_size - optimized_size) as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };

        (
            rewritten,
            RewriteReport {
                original_size,
                optimized_size,
                reduction_percentage,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_conservative() {
        let rewriter = PromptRewriter::new();
        let (output, report) = rewriter.rewrite(
            "Please carefully analyze this codebase.",
            RewriteMode::Conservative,
        );
        assert!(!output.contains("Please"));
        assert!(!output.contains("carefully"));
        assert!(report.reduction_percentage > 0.0);
    }

    #[test]
    fn test_rewrite_aggressive() {
        let rewriter = PromptRewriter::new();
        let (output, _) = rewriter.rewrite(
            "Analyze the repository and tell me all problems",
            RewriteMode::Aggressive,
        );
        assert_eq!(output, "Analyze repository. Report all issues.");
    }
}
