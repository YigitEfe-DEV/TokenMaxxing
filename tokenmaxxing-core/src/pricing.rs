//! Cost estimation for token counts.
//!
//! Model prices change over time and differ per provider and per plan, so no
//! price table is shipped with this crate. A caller has to supply the rates
//! it actually pays for; see [`PricingProvider`].

/// Price schedule of a single model.
///
/// Rates are expressed per one million tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelPricing {
    /// Currency the rates are expressed in, for example `"USD"`.
    pub currency: String,
    /// Price of one million input tokens.
    pub input_per_million_tokens: f64,
    /// Price of one million output tokens.
    pub output_per_million_tokens: f64,
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
}
