//! Cost estimation and the optimization report.
//!
//! The cost of a request is derived from a token count and a dated pricing
//! profile. A report states where both numbers come from, because a cost
//! figure without its provenance cannot be checked.

use crate::engine::{CountKind, TokenCount};
use crate::optimizer::RemovedContent;
use crate::pricing::{ModelPricing, PricingCategory, PricingError, PricingProfile};

/// The token usage of a request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TokenUsage {
    /// Tokens that are sent to the model.
    pub input_tokens: usize,
    /// Tokens the model produces.
    pub output_tokens: usize,
    /// Tokens that are served from the prompt cache.
    pub cached_input_tokens: usize,
    /// Tokens that are written into the prompt cache.
    pub cache_write_tokens: usize,
}

impl TokenUsage {
    /// Usage of a prompt that is sent to the model.
    pub fn input(tokens: usize) -> Self {
        Self {
            input_tokens: tokens,
            ..Self::default()
        }
    }

    /// The usage categories that are actually billed.
    pub fn billed_categories(&self) -> Vec<PricingCategory> {
        let mut categories = Vec::new();
        if self.input_tokens > 0 {
            categories.push(PricingCategory::Input);
        }
        if self.output_tokens > 0 {
            categories.push(PricingCategory::Output);
        }
        if self.cached_input_tokens > 0 {
            categories.push(PricingCategory::CachedInput);
        }
        if self.cache_write_tokens > 0 {
            categories.push(PricingCategory::CacheWrite);
        }
        categories
    }
}

/// The cost of a usage breakdown.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CostBreakdown {
    /// Currency the amounts are expressed in.
    pub currency: String,
    /// Cost of the input tokens.
    pub input: f64,
    /// Cost of the output tokens.
    pub output: f64,
    /// Cost of the cached input tokens.
    pub cached_input: f64,
    /// Cost of the cache write tokens.
    pub cache_write: f64,
    /// Sum of all categories.
    pub total: f64,
    /// Date the rates were captured.
    pub pricing_effective_date: String,
    /// Where the rates come from.
    pub pricing_source: String,
}

/// Computes costs from a pricing profile.
pub struct CostEngine {
    profile: PricingProfile,
}

impl CostEngine {
    /// Builds an engine for a pricing profile.
    pub fn new(profile: PricingProfile) -> Self {
        Self { profile }
    }

    /// The pricing profile the engine uses.
    pub fn profile(&self) -> &PricingProfile {
        &self.profile
    }

    /// The prices of a model.
    pub fn pricing_for(&self, model: &str) -> Option<&ModelPricing> {
        self.profile.models.get(model)
    }

    /// Computes the cost of a usage breakdown.
    pub fn cost(&self, model: &str, usage: TokenUsage) -> Result<CostBreakdown, CostError> {
        let pricing = self
            .pricing_for(model)
            .ok_or_else(|| CostError::ModelNotFound {
                model: model.to_string(),
                available: self
                    .profile
                    .models()
                    .into_iter()
                    .map(|model| model.to_string())
                    .collect(),
            })?;

        let input = pricing.input_cost(usage.input_tokens);
        let output = pricing.output_cost(usage.output_tokens);
        let cached_input = pricing
            .cached_input_per_million_tokens
            .map(|rate| crate::pricing::estimate_cost(usage.cached_input_tokens, rate))
            .unwrap_or(0.0);
        let cache_write = pricing
            .cache_write_per_million_tokens
            .map(|rate| crate::pricing::estimate_cost(usage.cache_write_tokens, rate))
            .unwrap_or(0.0);

        Ok(CostBreakdown {
            currency: if pricing.currency.is_empty() {
                self.profile.currency.clone()
            } else {
                pricing.currency.clone()
            },
            input,
            output,
            cached_input,
            cache_write,
            total: input + output + cached_input + cache_write,
            pricing_effective_date: self.profile.effective_date.clone(),
            pricing_source: self.profile.source.clone(),
        })
    }
}

/// Errors of the cost engine.
#[derive(Debug, thiserror::Error)]
pub enum CostError {
    /// The pricing profile does not know the model.
    #[error("the pricing profile has no entry for model `{model}`; it knows: {available:?}")]
    ModelNotFound {
        model: String,
        available: Vec<String>,
    },
    /// The pricing profile could not be used.
    #[error("the pricing profile could not be used: {0}")]
    Profile(String),
}

impl From<PricingError> for CostError {
    fn from(error: PricingError) -> Self {
        match error {
            PricingError::ModelNotFound { model } => CostError::ModelNotFound {
                model,
                available: Vec::new(),
            },
            other => CostError::Profile(other.to_string()),
        }
    }
}

/// Where the numbers of a report come from.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Provenance {
    /// How the tokens were counted.
    pub count_method: CountKind,
    /// Identifier of the tokenizer, if a tokenizer was used.
    pub tokenizer: Option<String>,
    /// Version of the tokenizer implementation.
    pub tokenizer_version: Option<String>,
    /// Encoding of the tokenizer.
    pub encoding: Option<String>,
    /// Model the counts refer to.
    pub model: Option<String>,
    /// Date of the pricing profile.
    pub pricing_effective_date: Option<String>,
    /// Source of the pricing profile.
    pub pricing_source: Option<String>,
}

/// The effect of optimizing a prompt.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OptimizationReport {
    /// Token count of the original text.
    pub before: TokenCount,
    /// Token count of the optimized text.
    pub after: TokenCount,
    /// Number of bytes that were removed.
    pub removed_bytes: usize,
    /// What was removed.
    pub removed: Vec<RemovedContent>,
    /// Estimated cost of the original text.
    pub cost_before: Option<CostBreakdown>,
    /// Estimated cost of the optimized text.
    pub cost_after: Option<CostBreakdown>,
    /// Where the numbers come from.
    pub provenance: Provenance,
    /// What the report does and does not claim.
    pub notes: Vec<String>,
}

impl OptimizationReport {
    /// Difference between the costs, if both are known.
    pub fn cost_difference(&self) -> Option<f64> {
        match (&self.cost_before, &self.cost_after) {
            (Some(before), Some(after)) => Some(after.total - before.total),
            _ => None,
        }
    }

    /// Relative difference between the costs, if both are known.
    pub fn cost_difference_percentage(&self) -> Option<f64> {
        let before = self.cost_before.as_ref()?;
        if before.total == 0.0 {
            return None;
        }
        self.cost_difference()
            .map(|difference| difference / before.total * 100.0)
    }

    /// Number of tokens that were removed.
    pub fn removed_tokens(&self) -> i64 {
        self.before.tokens as i64 - self.after.tokens as i64
    }
}

/// Builds an optimization report.
///
/// `before` and `after` are the token counts of the original and the optimized
/// text, `cost` prices them for the given model. The report never claims that
/// the shorter text keeps the task quality, because nothing in this crate
/// measures that.
pub fn build_optimization_report(
    original: &str,
    optimized: &str,
    before: TokenCount,
    after: TokenCount,
    removed: Vec<RemovedContent>,
    provenance: Provenance,
    cost: Option<(&CostEngine, &str)>,
) -> OptimizationReport {
    let (cost_before, cost_after) = match cost {
        Some((engine, model)) => (
            engine.cost(model, TokenUsage::input(before.tokens)).ok(),
            engine.cost(model, TokenUsage::input(after.tokens)).ok(),
        ),
        None => (None, None),
    };

    let mut notes = vec![
        "A shorter prompt does not prove that the task quality is preserved. \
         No quality evaluation was performed."
            .to_string(),
    ];
    if cost_before.is_none() {
        notes.push(
            "No pricing profile was applied, so the report carries token counts only.".to_string(),
        );
    }

    OptimizationReport {
        before,
        after,
        removed_bytes: original.len().saturating_sub(optimized.len()),
        removed,
        cost_before,
        cost_after,
        provenance,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pricing::ModelPricing;

    fn profile() -> PricingProfile {
        PricingProfile::new("USD", "2026-10-01", "https://example.invalid/pricing").with_model(
            "model-a",
            ModelPricing::new("USD", 4.0, 20.0).with_cached_input(0.2),
        )
    }

    #[test]
    fn test_cost_of_input_tokens() {
        let engine = CostEngine::new(profile());
        let cost = engine
            .cost("model-a", TokenUsage::input(1_000_000))
            .unwrap();

        assert_eq!(cost.input, 4.0);
        assert_eq!(cost.total, 4.0);
        assert_eq!(cost.currency, "USD");
        assert_eq!(cost.pricing_effective_date, "2026-10-01");
    }

    #[test]
    fn test_cached_input_is_priced_separately() {
        let engine = CostEngine::new(profile());
        let cost = engine
            .cost(
                "model-a",
                TokenUsage {
                    input_tokens: 1_000_000,
                    cached_input_tokens: 1_000_000,
                    ..TokenUsage::default()
                },
            )
            .unwrap();

        assert_eq!(cost.input, 4.0);
        assert_eq!(cost.cached_input, 0.2);
        assert_eq!(cost.total, 4.2);
    }

    #[test]
    fn test_unknown_models_are_reported() {
        let engine = CostEngine::new(profile());
        let error = engine.cost("model-b", TokenUsage::input(10)).unwrap_err();

        assert!(matches!(error, CostError::ModelNotFound { .. }));
    }

    #[test]
    fn test_report_states_before_and_after() {
        let engine = CostEngine::new(profile());
        let counter =
            |text: &str| TokenCount::new(text.len(), text.len(), text.len(), CountKind::ExactLocal);

        let report = build_optimization_report(
            "hello world",
            "hello",
            counter("hello world"),
            counter("hello"),
            vec![RemovedContent::between(
                "repeated instruction lines",
                "hello world",
                "hello",
            )],
            Provenance {
                count_method: CountKind::ExactLocal,
                ..Provenance::default()
            },
            Some((&engine, "model-a")),
        );

        assert_eq!(report.before.tokens, 11);
        assert_eq!(report.after.tokens, 5);
        assert_eq!(report.removed_tokens(), 6);
        assert_eq!(report.removed_bytes, 6);
        assert!(report.cost_before.is_some());
        assert!(report.cost_difference().unwrap() < 0.0);
        assert!(report.cost_difference_percentage().unwrap() < 0.0);
    }

    #[test]
    fn test_report_does_not_claim_that_quality_is_preserved() {
        let counter =
            |text: &str| TokenCount::new(text.len(), text.len(), text.len(), CountKind::Estimated);
        let report = build_optimization_report(
            "a long prompt",
            "short",
            counter("a long prompt"),
            counter("short"),
            Vec::new(),
            Provenance::default(),
            None,
        );

        assert!(report
            .notes
            .iter()
            .any(|note| note.contains("does not prove that the task quality is preserved")));
        assert!(report.cost_before.is_none());
    }
}
