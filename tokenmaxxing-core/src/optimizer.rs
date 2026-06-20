use regex::Regex;
use std::collections::HashSet;

pub struct PromptOptimizer;

#[derive(Debug, Default)]
pub struct OptimizationReport {
    pub original_chars: usize,
    pub optimized_chars: usize,
    pub reduction_percentage: f64,
}

impl Default for PromptOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptOptimizer {
    pub fn new() -> Self {
        Self
    }

    /// Compress markdown by removing excessive whitespace and empty lines
    pub fn minify_markdown(text: &str) -> String {
        let re_empty_lines = Regex::new(r"\n{3,}").unwrap();
        let re_trailing_ws = Regex::new(r"[ \t]+\n").unwrap();

        let mut minified = re_trailing_ws.replace_all(text, "\n").into_owned();
        minified = re_empty_lines.replace_all(&minified, "\n\n").into_owned();
        minified.trim().to_string()
    }

    /// Minify JSON by removing whitespace outside of strings
    pub fn minify_json(text: &str) -> String {
        // A naive JSON minifier that uses serde_json to parse and rewrite compactly
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
            return serde_json::to_string(&value).unwrap_or_else(|_| text.to_string());
        }
        text.to_string()
    }

    /// Minify XML/HTML by removing spaces between tags
    pub fn minify_xml(text: &str) -> String {
        let re_spaces = Regex::new(r">\s+<").unwrap();
        re_spaces.replace_all(text, "><").into_owned()
    }

    /// Deduplicate repeated instruction paragraphs
    pub fn deduplicate_instructions(text: &str) -> String {
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();
            // Only deduplicate substantial lines to avoid removing empty lines or short structural parts
            if trimmed.len() > 20 {
                if seen.insert(trimmed.to_string()) {
                    result.push(line);
                }
            } else {
                result.push(line);
            }
        }

        result.join("\n")
    }

    /// Optimize a prompt by applying multiple minifications
    pub fn optimize_prompt(&self, text: &str) -> (String, OptimizationReport) {
        let original_chars = text.len();

        let mut optimized = Self::minify_markdown(text);
        optimized = Self::deduplicate_instructions(&optimized);

        // Try json/xml minification if it looks like json/xml
        if optimized.trim().starts_with('{') || optimized.trim().starts_with('[') {
            optimized = Self::minify_json(&optimized);
        } else if optimized.trim().starts_with('<') {
            optimized = Self::minify_xml(&optimized);
        }

        let optimized_chars = optimized.len();
        let reduction_percentage = if original_chars > 0 {
            (original_chars - optimized_chars) as f64 / original_chars as f64 * 100.0
        } else {
            0.0
        };

        let report = OptimizationReport {
            original_chars,
            optimized_chars,
            reduction_percentage,
        };

        (optimized, report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minify_markdown() {
        let text = "Header\n\n\n\nContent   \nFooter";
        let minified = PromptOptimizer::minify_markdown(text);
        assert_eq!(minified, "Header\n\nContent\nFooter");
    }

    #[test]
    fn test_minify_json() {
        let text = r#"{
            "key": "value"
        }"#;
        let minified = PromptOptimizer::minify_json(text);
        assert_eq!(minified, r#"{"key":"value"}"#);
    }
}
