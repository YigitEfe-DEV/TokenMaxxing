//! Interfaces for tokenizers and token counting providers.
//!
//! The core crate deliberately depends on neither an HTTP client nor on a
//! vendor SDK. It only describes *what* a tokenizer or a provider has to
//! offer, so that Anthropic and OpenAI adapters can be added later without
//! touching the engine.

use crate::engine::{CountKind, TokenCount};

/// A tokenizer that runs locally and ships a real vocabulary.
///
/// Implementations have to return the number of tokens their tokenizer
/// produces for a piece of text. Because the vocabulary is real, counts
/// produced through this trait are reported as [`CountKind::ExactLocal`].
pub trait LocalTokenizer: Send + Sync {
    /// Stable identifier of the tokenizer, for example `"cl100k_base"`.
    fn name(&self) -> &str;

    /// Number of tokens the tokenizer produces for `text`.
    fn count_tokens(&self, text: &str) -> usize;
}

/// Errors a token counting provider can report.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// The provider does not know the requested model.
    #[error("provider `{provider}` does not support model `{model}`")]
    UnsupportedModel { provider: String, model: String },
    /// The provider could not be reached or refused the request.
    #[error("provider request failed: {0}")]
    Request(String),
    /// The provider answered with something that could not be understood.
    #[error("provider returned a malformed response: {0}")]
    MalformedResponse(String),
    /// The provider is missing credentials or an endpoint.
    #[error("provider is not configured: {0}")]
    NotConfigured(String),
}

/// A service that reports the authoritative token count for a model.
///
/// The interface is transport agnostic on purpose: an Anthropic adapter and
/// an OpenAI adapter only have to differ in how they talk to their backend.
///
/// A successful count is always reported as [`CountKind::ProviderApi`] because
/// the provider owns the tokenizer that the count refers to.
pub trait TokenCountProvider: Send + Sync {
    /// Identifier of the provider, for example `"anthropic"`.
    fn provider(&self) -> &str;

    /// Model the provider counts tokens for.
    fn model(&self) -> &str;

    /// Asks the provider how many tokens `text` consumes.
    fn count_tokens(&self, text: &str) -> Result<usize, ProviderError>;

    /// Counts `text` and labels the result as provider authoritative.
    ///
    /// Adapters should implement [`TokenCountProvider::count_tokens`] and use
    /// this helper so that the provenance cannot be mislabelled.
    fn count(&self, text: &str) -> Result<TokenCount, ProviderError> {
        let tokens = self.count_tokens(text)?;
        Ok(TokenCount::new(
            tokens,
            text.chars().count(),
            text.len(),
            CountKind::ProviderApi,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// A stand-in for a real tokenizer vocabulary.
    struct FakeTokenizer;

    impl LocalTokenizer for FakeTokenizer {
        fn name(&self) -> &str {
            "fake"
        }

        fn count_tokens(&self, text: &str) -> usize {
            text.split_whitespace().count()
        }
    }

    /// A stand-in for a remote provider.
    struct FakeProvider;

    impl TokenCountProvider for FakeProvider {
        fn provider(&self) -> &str {
            "fake"
        }

        fn model(&self) -> &str {
            "fake-model"
        }

        fn count_tokens(&self, text: &str) -> Result<usize, ProviderError> {
            Ok(text.split_whitespace().count())
        }
    }

    #[test]
    fn test_local_tokenizer_produces_exact_counts() {
        let engine = crate::engine::TokenEngine::with_tokenizer(Arc::new(FakeTokenizer));
        let count = engine.count_text("one two three");

        assert_eq!(count.tokens, 3);
        assert_eq!(count.kind, CountKind::ExactLocal);
        assert!(count.kind.is_exact());
    }

    #[test]
    fn test_provider_counts_are_labelled_as_authoritative() {
        let count = FakeProvider.count("one two three").unwrap();

        assert_eq!(count.tokens, 3);
        assert_eq!(count.kind, CountKind::ProviderApi);
        assert!(count.kind.is_exact());
    }
}
