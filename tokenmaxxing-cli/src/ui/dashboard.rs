//! Collecting the data of the dashboard.
//!
//! The module only gathers data and prepares it for the view. Every count
//! comes from the token engine of the core crate, every suggestion from the
//! repository analysis and the prompt optimizer, and every price from the
//! pricing profile the caller supplied.

use std::path::{Path, PathBuf};
use tokenmaxxing_core::analysis::{count_files, total_of, FileTokenCount};
use tokenmaxxing_core::cost::{CostBreakdown, CostEngine, TokenUsage};
use tokenmaxxing_core::engine::{text_files, CountKind, EstimationMode, TokenCount, TokenEngine};
use tokenmaxxing_core::optimizer::PromptOptimizer;
use tokenmaxxing_core::pricing::PricingProfile;
use tokenmaxxing_core::provider::ProviderKind;
use tokenmaxxing_core::repository::{SmartRepositoryMode, WasteReport};
use tokenmaxxing_core::tokenizer::openai;

/// How many suggestions the dashboard keeps.
const MAX_SUGGESTIONS: usize = 60;

/// The model that is used when the caller does not name one.
pub const DEFAULT_MODEL: &str = "gpt-4o";

/// The settings of a scan.
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// The model the costs and the comparison refer to.
    pub model: Option<String>,
    /// The pricing profile, when the caller has one.
    pub pricing: Option<PricingProfile>,
    /// The provider the model belongs to.
    pub provider: ProviderKind,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            model: Some(DEFAULT_MODEL.to_string()),
            pricing: None,
            provider: ProviderKind::OpenAi,
        }
    }
}

/// The same text, measured with different methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonRow {
    /// Name of the method.
    pub label: String,
    /// The model the method belongs to, when it has one.
    pub model: Option<String>,
    /// The tokenizer that produced the count, when there was one.
    pub tokenizer: Option<String>,
    /// How the count was produced.
    pub kind: CountKind,
    /// The number of tokens.
    pub tokens: usize,
}

/// The change a suggestion proposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedAction {
    /// What the action does.
    pub kind: ActionKind,
    /// The file the action touches.
    pub path: PathBuf,
    /// The content of the file after the action, for a rewrite.
    pub replacement: Option<String>,
    /// The file that stays, for a removal.
    pub keep: Option<PathBuf>,
}

/// The kinds of change the dashboard can propose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    /// The file is rewritten with a shorter version of itself.
    Rewrite,
    /// The file is removed, because an identical file stays.
    Remove,
}

/// A suggestion with the reason it was produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// A short title.
    pub title: String,
    /// Why the suggestion was produced.
    pub reason: String,
    /// How many tokens the change is expected to save.
    pub estimated_tokens: usize,
    /// The change, when the dashboard can carry it out.
    pub action: Option<ProposedAction>,
}

/// Everything the dashboard shows.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// The scanned directory.
    pub root: PathBuf,
    /// The count of every file.
    pub files: Vec<FileTokenCount>,
    /// The sum of all counts.
    pub total: TokenCount,
    /// How many files could not be read as text.
    pub skipped: usize,
    /// The same text measured with different methods.
    pub comparison: Vec<ComparisonRow>,
    /// The suggestions of the scan.
    pub suggestions: Vec<Suggestion>,
    /// The cost of the scanned text, when a profile and a model are known.
    pub cost: Option<CostBreakdown>,
    /// How the tokens were counted.
    pub measurement: String,
    /// The result of the repository analysis.
    pub waste: WasteReport,
}

impl Snapshot {
    /// The share of the scanned tokens that a file holds.
    pub fn share_of(&self, file: &FileTokenCount) -> f64 {
        if self.total.tokens == 0 {
            return 0.0;
        }
        file.count.tokens as f64 / self.total.tokens as f64 * 100.0
    }
}

/// Builds the engine that the scan counts with.
fn build_engine(options: &ScanOptions) -> (TokenEngine, String) {
    let model = options.model.as_deref().unwrap_or(DEFAULT_MODEL);

    if options.provider.has_local_tokenizer() {
        match openai::tokenizer_for_model(model) {
            Ok(tokenizer) => {
                let info = tokenizer.info();
                let measurement = format!("{} ({})", info.id, info.encoding);
                return (TokenEngine::with_tokenizer(tokenizer), measurement);
            }
            Err(error) => {
                // The model has no published vocabulary, so the scan falls back
                // to the estimator and says so.
                let measurement = format!("estimate (no tokenizer for {model}: {error})");
                return (TokenEngine::new(EstimationMode::Hybrid), measurement);
            }
        }
    }

    (
        TokenEngine::new(EstimationMode::Hybrid),
        format!(
            "estimate (no local tokenizer for {})",
            options.provider.as_str()
        ),
    )
}

/// Scans a directory and gathers everything the dashboard shows.
pub fn scan(root: &Path, options: &ScanOptions) -> Snapshot {
    let (engine, measurement) = build_engine(options);

    let files = count_files(&engine, root);
    let total = total_of(&files);
    let skipped = text_files(root).len().saturating_sub(files.len());

    let waste = SmartRepositoryMode::new().analyze_waste(root);
    let comparison = compare(&engine, root, &files);
    let suggestions = suggest(&engine, root, &files, &waste);
    let cost = match (&options.pricing, &options.model) {
        (Some(profile), Some(model)) => CostEngine::new(profile.clone())
            .cost(model, TokenUsage::input(total.tokens))
            .ok(),
        _ => None,
    };

    Snapshot {
        root: root.to_path_buf(),
        files,
        total,
        skipped,
        comparison,
        suggestions,
        cost,
        measurement,
        waste,
    }
}

/// Measures the same text with every supported method.
fn compare(engine: &TokenEngine, root: &Path, files: &[FileTokenCount]) -> Vec<ComparisonRow> {
    let Some(sample) = largest_content(root, files) else {
        return Vec::new();
    };

    let mut rows = Vec::new();

    for (encoding, model) in openai::comparable_encodings() {
        let Ok(tokenizer) = openai::tokenizer_for_encoding(encoding) else {
            continue;
        };
        let info = tokenizer.info();
        rows.push(ComparisonRow {
            label: encoding.to_string(),
            model: Some(model.to_string()),
            tokenizer: Some(info.id.clone()),
            kind: CountKind::ExactLocal,
            tokens: tokenizer.count_tokens(&sample),
        });
    }

    rows.push(ComparisonRow {
        label: "ratio estimate".to_string(),
        model: None,
        tokenizer: None,
        kind: CountKind::Estimated,
        tokens: TokenEngine::new(EstimationMode::Fast)
            .count_text(&sample)
            .tokens,
    });
    rows.push(ComparisonRow {
        label: "structural approximation".to_string(),
        model: None,
        tokenizer: None,
        kind: CountKind::Approximate,
        tokens: TokenEngine::new(EstimationMode::Accurate)
            .count_text(&sample)
            .tokens,
    });

    let _ = engine;
    rows
}

/// Reads the content of the largest file.
fn largest_content(root: &Path, files: &[FileTokenCount]) -> Option<String> {
    let largest = files.iter().max_by_key(|file| file.count.bytes)?;

    std::fs::read_to_string(root.join(&largest.path)).ok()
}

/// Builds the suggestions of a scan.
fn suggest(
    engine: &TokenEngine,
    root: &Path,
    files: &[FileTokenCount],
    waste: &WasteReport,
) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();
    let optimizer = PromptOptimizer::new();

    for (first, second) in &waste.duplicates {
        let path = root.join(second);
        let tokens = files
            .iter()
            .find(|file| file.path == *second)
            .map(|file| file.count.tokens)
            .unwrap_or(0);

        suggestions.push(Suggestion {
            title: format!("Remove the duplicate {}", second.display()),
            reason: format!(
                "{} and {} have the same content, so one of them is only stored twice.",
                first.display(),
                second.display()
            ),
            estimated_tokens: tokens,
            action: Some(ProposedAction {
                kind: ActionKind::Remove,
                path,
                replacement: None,
                keep: Some(root.join(first)),
            }),
        });
    }

    for file in files {
        let Ok(content) = std::fs::read_to_string(root.join(&file.path)) else {
            continue;
        };
        let (optimized, report) = optimizer.optimize_prompt(&content);
        if optimized == content {
            continue;
        }

        // A change that only touches the trailing newline of a file is
        // formatting noise, not a suggestion.
        const WORTH_REPORTING: usize = 16;
        let removed_bytes: usize = report.removed.iter().map(|removed| removed.bytes).sum();
        if removed_bytes < WORTH_REPORTING {
            continue;
        }

        let saved = engine
            .count_text(&content)
            .tokens
            .saturating_sub(engine.count_text(&optimized).tokens);
        let repeated = report
            .removed
            .iter()
            .find(|removed| removed.category.contains("repeated"))
            .map(|removed| removed.bytes)
            .unwrap_or(0);

        let reason = if repeated > 0 {
            format!(
                "the file repeats {} bytes of instruction lines, which can be dropped",
                repeated
            )
        } else {
            "the file carries formatting whitespace that can be dropped".to_string()
        };

        suggestions.push(Suggestion {
            title: format!("Minify {}", file.path.display()),
            reason,
            estimated_tokens: saved,
            action: Some(ProposedAction {
                kind: ActionKind::Rewrite,
                path: file.path.clone(),
                replacement: Some(optimized),
                keep: None,
            }),
        });
    }

    suggestions.sort_by_key(|suggestion| std::cmp::Reverse(suggestion.estimated_tokens));
    suggestions.truncate(MAX_SUGGESTIONS);
    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenmaxxing_core::pricing::ModelPricing;

    fn fixture(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("tokenmaxxing-dashboard-{name}"));
        std::fs::create_dir_all(directory.join("sub")).unwrap();
        std::fs::write(directory.join("a.txt"), "hello world\n".repeat(20)).unwrap();
        std::fs::write(
            directory.join("sub/b.txt"),
            "Merhaba dünya! Bu bir testtir.\n".repeat(5),
        )
        .unwrap();
        directory
    }

    fn cleanup(directory: &Path) {
        std::fs::remove_dir_all(directory).ok();
    }

    #[test]
    fn test_an_empty_repository_produces_an_empty_snapshot() {
        let directory = std::env::temp_dir().join("tokenmaxxing-dashboard-empty");
        std::fs::create_dir_all(&directory).unwrap();

        let snapshot = scan(&directory, &ScanOptions::default());

        assert!(snapshot.files.is_empty());
        assert_eq!(snapshot.total.tokens, 0);
        assert!(snapshot.cost.is_none());
        assert!(snapshot.comparison.is_empty());

        cleanup(&directory);
    }

    #[test]
    fn test_a_scanned_repository_reports_its_files() {
        let directory = fixture("files");

        let snapshot = scan(&directory, &ScanOptions::default());

        assert_eq!(snapshot.files.len(), 2);
        assert!(snapshot.total.tokens > 0);
        assert_eq!(snapshot.total.chars, 240 + 155);
        assert!(snapshot.measurement.contains("tiktoken"));

        cleanup(&directory);
    }

    #[test]
    fn test_the_comparison_uses_the_same_text_for_every_method() {
        let directory = fixture("comparison");

        let snapshot = scan(&directory, &ScanOptions::default());

        assert!(snapshot.comparison.len() >= 4);
        assert!(snapshot
            .comparison
            .iter()
            .any(|row| row.kind == CountKind::ExactLocal));
        assert!(snapshot
            .comparison
            .iter()
            .any(|row| row.kind == CountKind::Estimated));

        cleanup(&directory);
    }

    #[test]
    fn test_the_cost_is_only_computed_with_a_profile_and_a_model() {
        let directory = fixture("cost");
        let profile = PricingProfile::new("USD", "2026-10-01", "test")
            .with_model("gpt-4o", ModelPricing::new("USD", 2.5, 10.0));

        let without = scan(
            &directory,
            &ScanOptions {
                model: Some("gpt-4o".to_string()),
                pricing: None,
                provider: ProviderKind::OpenAi,
            },
        );
        assert!(without.cost.is_none());

        let with = scan(
            &directory,
            &ScanOptions {
                model: Some("gpt-4o".to_string()),
                pricing: Some(profile),
                provider: ProviderKind::OpenAi,
            },
        );
        assert!(with.cost.is_some());
        assert_eq!(with.cost.unwrap().currency, "USD");

        cleanup(&directory);
    }

    #[test]
    fn test_an_unknown_model_falls_back_to_the_estimator() {
        let directory = fixture("unknown-model");

        let snapshot = scan(
            &directory,
            &ScanOptions {
                model: Some("claude-opus-5-5".to_string()),
                pricing: None,
                provider: ProviderKind::OpenAi,
            },
        );

        assert!(snapshot.measurement.contains("estimate"));
        assert!(snapshot.total.tokens > 0);

        cleanup(&directory);
    }

    #[test]
    fn test_suggestions_explain_themselves() {
        let directory = fixture("suggestions");

        let snapshot = scan(&directory, &ScanOptions::default());

        assert!(!snapshot.suggestions.is_empty());
        for suggestion in &snapshot.suggestions {
            assert!(!suggestion.reason.is_empty());
            assert!(!suggestion.title.is_empty());
        }

        cleanup(&directory);
    }
}
