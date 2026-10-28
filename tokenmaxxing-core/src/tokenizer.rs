//! Interfaces for tokenizers and token counting providers.
//!
//! The core crate deliberately depends on neither an HTTP client nor on a
//! vendor SDK. It only describes *what* a tokenizer or a provider has to
//! offer, so that Anthropic and OpenAI adapters can be added later without
//! touching the engine.

use crate::engine::{CountKind, TokenCount};

pub mod openai;

/// Where the vocabulary of a tokenizer came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AssetSource {
    /// The vocabulary is shipped with the binary, so counting works offline.
    Bundled,
    /// The vocabulary is fetched at runtime.
    Remote,
    /// The vocabulary could not be loaded.
    Missing,
}

impl AssetSource {
    /// Human readable label.
    pub fn label(self) -> &'static str {
        match self {
            AssetSource::Bundled => "bundled",
            AssetSource::Remote => "remote",
            AssetSource::Missing => "missing",
        }
    }
}

/// Identity and capabilities of a tokenizer.
///
/// Every tokenizer states which model it is the authoritative tokenizer for
/// and which input types it cannot handle, so that a caller never has to
/// guess what a count refers to.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TokenizerInfo {
    /// Stable identifier, for example `tiktoken:cl100k_base`.
    pub id: String,
    /// Version of the tokenizer implementation.
    pub version: String,
    /// Encoding family, for example `cl100k_base`.
    pub encoding: String,
    /// Provider that owns the tokenizer.
    pub provider: String,
    /// Models this tokenizer is the authoritative tokenizer for.
    pub models: Vec<String>,
    /// Input types the tokenizer accepts.
    pub supported_inputs: Vec<String>,
    /// Input types the tokenizer cannot handle.
    pub unsupported_inputs: Vec<String>,
    /// Where the vocabulary came from.
    pub assets: AssetSource,
}

impl Default for TokenizerInfo {
    fn default() -> Self {
        Self {
            id: "unknown".to_string(),
            version: "unknown".to_string(),
            encoding: "unknown".to_string(),
            provider: "unknown".to_string(),
            models: Vec::new(),
            supported_inputs: vec!["utf-8 text".to_string()],
            unsupported_inputs: vec![
                "binary data".to_string(),
                "non utf-8 files".to_string(),
                "images".to_string(),
                "pdf documents".to_string(),
            ],
            assets: AssetSource::Missing,
        }
    }
}

impl TokenizerInfo {
    /// Returns `true` when the tokenizer can count text without any network
    /// access.
    pub fn is_offline(&self) -> bool {
        self.assets == AssetSource::Bundled
    }
}

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

    /// Identity and capabilities of the tokenizer.
    ///
    /// The default implementation only knows the name, so implementations
    /// should override it whenever they can describe themselves fully.
    fn info(&self) -> TokenizerInfo {
        TokenizerInfo {
            id: self.name().to_string(),
            ..TokenizerInfo::default()
        }
    }
}

/// Identity and capabilities of a counting provider.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProviderInfo {
    /// Stable identifier, for example `anthropic:messages/count_tokens`.
    pub id: String,
    /// Provider that owns the tokenizer.
    pub provider: String,
    /// Model the counts refer to.
    pub model: String,
    /// Endpoint that produces the counts.
    pub endpoint: String,
    /// Whether the count is the provider's own count for its own model.
    pub authoritative: bool,
    /// Caveats the provider documents about the count.
    pub caveats: Vec<String>,
    /// Input types the endpoint rejects.
    pub unsupported_inputs: Vec<String>,
}

/// Errors a token counting provider can report.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// The provider does not know the requested model.
    #[error("provider `{provider}` does not support model `{model}`")]
    UnsupportedModel { provider: String, model: String },
    /// The provider rejected the credentials.
    #[error("provider `{provider}` rejected the credentials")]
    Authentication { provider: String },
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

    /// Identity and capabilities of the provider.
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: format!("{}:{}", self.provider(), self.model()),
            provider: self.provider().to_string(),
            model: self.model().to_string(),
            endpoint: String::new(),
            authoritative: true,
            caveats: Vec::new(),
            unsupported_inputs: Vec::new(),
        }
    }

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

    #[test]
    fn test_default_tokenizer_info_reports_its_limits() {
        let info = FakeTokenizer.info();

        assert_eq!(info.id, "fake");
        assert_eq!(info.assets, AssetSource::Missing);
        assert!(!info.is_offline());
        assert!(info
            .unsupported_inputs
            .iter()
            .any(|kind| kind == "binary data"));
    }
}
