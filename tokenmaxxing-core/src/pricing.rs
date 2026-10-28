//! Cost estimation for token counts.
//!
//! Model prices change over time and differ per provider and per plan, so no
//! price table is shipped with this crate. A caller has to supply the rates it
//! actually pays for, as a dated [`PricingProfile`].

use std::collections::BTreeMap;

/// The usage categories a provider bills separately.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum PricingCategory {
    /// Tokens that are sent to the model and not served from a cache.
    Input,
    /// Tokens the model produces.
    Output,
    /// Tokens that are served from the prompt cache.
    CachedInput,
    /// Tokens that are written into the prompt cache.
    CacheWrite,
}

impl PricingCategory {
    /// Identifier of the category.
    pub fn as_str(self) -> &'static str {
        match self {
            PricingCategory::Input => "input",
            PricingCategory::Output => "output",
            PricingCategory::CachedInput => "cached_input",
            PricingCategory::CacheWrite => "cache_write",
        }
    }
}

/// Price schedule of a single model.
///
/// Rates are expressed per one million tokens.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ModelPricing {
    /// Currency the rates are expressed in, for example `"USD"`.
    ///
    /// An empty value means the currency of the profile is used.
    #[serde(default)]
    pub currency: String,
    /// Price of one million input tokens.
    pub input_per_million_tokens: f64,
    /// Price of one million output tokens.
    pub output_per_million_tokens: f64,
    /// Price of one million tokens that were served from the prompt cache.
    #[serde(default)]
    pub cached_input_per_million_tokens: Option<f64>,
    /// Price of one million tokens that were written into the prompt cache.
    #[serde(default)]
    pub cache_write_per_million_tokens: Option<f64>,
}

impl ModelPricing {
    /// Builds a price schedule.
    pub fn new(
        currency: impl Into<String>,
        input_per_million_tokens: f64,
        output_per_million_tokens: f64,
    ) -> Self {
        Self {
            currency: currency.into(),
            input_per_million_tokens,
            output_per_million_tokens,
            cached_input_per_million_tokens: None,
            cache_write_per_million_tokens: None,
        }
    }

    /// Sets the price of cached input tokens.
    pub fn with_cached_input(mut self, rate: f64) -> Self {
        self.cached_input_per_million_tokens = Some(rate);
        self
    }

    /// Sets the price of cache write tokens.
    pub fn with_cache_write(mut self, rate: f64) -> Self {
        self.cache_write_per_million_tokens = Some(rate);
        self
    }

    /// The rate of a usage category, if the provider prices it separately.
    pub fn rate_for(&self, category: PricingCategory) -> Option<f64> {
        match category {
            PricingCategory::Input => Some(self.input_per_million_tokens),
            PricingCategory::Output => Some(self.output_per_million_tokens),
            PricingCategory::CachedInput => self.cached_input_per_million_tokens,
            PricingCategory::CacheWrite => self.cache_write_per_million_tokens,
        }
    }

    /// Cost of `tokens` input tokens.
    pub fn input_cost(&self, tokens: usize) -> f64 {
        estimate_cost(tokens, self.input_per_million_tokens)
    }

    /// Cost of `tokens` output tokens.
    pub fn output_cost(&self, tokens: usize) -> f64 {
        estimate_cost(tokens, self.output_per_million_tokens)
    }
}

/// Supplies the price schedule of a model.
///
/// The crate ships no default implementation, so prices always come from the
/// operator rather than from a hardcoded table that silently goes stale.
pub trait PricingProvider {
    /// Returns the price schedule of `model`, if it is known.
    fn pricing_for(&self, model: &str) -> Option<ModelPricing>;
}

/// A dated set of model prices.
///
/// The profile has to say when the rates were captured and where they came
/// from, because a cost estimate without that context cannot be checked
/// later.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PricingProfile {
    /// Currency all rates are expressed in.
    pub currency: String,
    /// Date the rates were captured, as `YYYY-MM-DD`.
    pub effective_date: String,
    /// Where the rates come from.
    pub source: String,
    /// Prices per model identifier.
    pub models: BTreeMap<String, ModelPricing>,
}

impl PricingProfile {
    /// Builds an empty profile.
    pub fn new(
        currency: impl Into<String>,
        effective_date: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            currency: currency.into(),
            effective_date: effective_date.into(),
            source: source.into(),
            models: BTreeMap::new(),
        }
    }

    /// Adds the prices of a model.
    pub fn with_model(mut self, model: impl Into<String>, pricing: ModelPricing) -> Self {
        self.models.insert(model.into(), pricing);
        self
    }

    /// Parses a profile from JSON.
    pub fn from_json(json: &str) -> Result<Self, PricingError> {
        serde_json::from_str(json).map_err(|error| PricingError::InvalidProfile {
            reason: error.to_string(),
        })
    }

    /// Reads a profile from a file.
    pub fn load_from_path(path: &std::path::Path) -> Result<Self, PricingError> {
        let content = std::fs::read_to_string(path).map_err(|error| PricingError::Unreadable {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;
        Self::from_json(&content)
    }

    /// The model identifiers the profile knows.
    pub fn models(&self) -> Vec<&str> {
        self.models.keys().map(|model| model.as_str()).collect()
    }
}

impl PricingProvider for PricingProfile {
    fn pricing_for(&self, model: &str) -> Option<ModelPricing> {
        self.models.get(model).cloned()
    }
}

/// Errors of the pricing module.
#[derive(Debug, thiserror::Error)]
pub enum PricingError {
    /// The profile file could not be read.
    #[error("the pricing profile `{path}` could not be read: {reason}")]
    Unreadable { path: String, reason: String },
    /// The profile is not valid.
    #[error("the pricing profile is not valid: {reason}")]
    InvalidProfile { reason: String },
    /// The profile does not know the model.
    #[error("the pricing profile has no entry for model `{model}`")]
    ModelNotFound { model: String },
}

/// Cost of `tokens` tokens at a rate given per one million tokens.
pub fn estimate_cost(tokens: usize, rate_per_million_tokens: f64) -> f64 {
    tokens as f64 / 1_000_000.0 * rate_per_million_tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_scales_with_the_token_count() {
        let pricing = ModelPricing::new("USD", 3.0, 15.0);

        assert_eq!(pricing.input_cost(1_000_000), 3.0);
        assert_eq!(pricing.output_cost(1_000_000), 15.0);
        assert_eq!(pricing.input_cost(500_000), 1.5);
        assert_eq!(pricing.input_cost(0), 0.0);
    }

    #[test]
    fn test_pricing_provider_lookup() {
        struct FixedPricing;

        impl PricingProvider for FixedPricing {
            fn pricing_for(&self, model: &str) -> Option<ModelPricing> {
                (model == "known").then(|| ModelPricing::new("USD", 1.0, 2.0))
            }
        }

        let provider = FixedPricing;

        assert_eq!(
            provider.pricing_for("known"),
            Some(ModelPricing::new("USD", 1.0, 2.0))
        );
        assert_eq!(provider.pricing_for("unknown"), None);
    }

    #[test]
    fn test_profile_is_parsed_from_json() {
        let json = r#"{
            "currency": "USD",
            "effective_date": "2026-10-01",
            "source": "https://example.invalid/pricing",
            "models": {
                "model-a": {
                    "input_per_million_tokens": 4.0,
                    "output_per_million_tokens": 20.0,
                    "cached_input_per_million_tokens": 0.2
                }
            }
        }"#;

        let profile = PricingProfile::from_json(json).unwrap();

        assert_eq!(profile.currency, "USD");
        assert_eq!(profile.effective_date, "2026-10-01");
        assert_eq!(profile.models(), vec!["model-a"]);

        let pricing = profile.pricing_for("model-a").unwrap();
        assert_eq!(pricing.input_per_million_tokens, 4.0);
        assert_eq!(pricing.cached_input_per_million_tokens, Some(0.2));
        assert_eq!(pricing.cache_write_per_million_tokens, None);
    }

    #[test]
    fn test_broken_profiles_are_reported() {
        let error = PricingProfile::from_json("not json").unwrap_err();

        assert!(matches!(error, PricingError::InvalidProfile { .. }));
    }
}
