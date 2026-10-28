//! OpenAI tokenizer support.
//!
//! The implementation is built on [`tiktoken_rs`], a pure Rust port of
//! OpenAI's published byte-pair-encoding vocabularies. The vocabularies are
//! shipped inside the crate, so counting works offline and always produces
//! the same result for the same encoding.
//!
//! Two things are worth keeping in mind:
//!
//! * the count is the count of the *raw text*. A chat request also spends
//!   tokens on its message framing, so this number is not the token count of
//!   a request body, and
//! * these encodings are OpenAI's. They say nothing about how many tokens a
//!   Claude model uses; Anthropic's tokenizer is not published.

use crate::tokenizer::{AssetSource, LocalTokenizer, TokenizerInfo};
use std::sync::Arc;
use tiktoken_rs::tokenizer::{get_tokenizer, Tokenizer};
use tiktoken_rs::CoreBPE;

/// Version of the tokenizer implementation this module is built on.
///
/// The exact patch release is pinned in `Cargo.lock`.
const IMPLEMENTATION: &str = "tiktoken-rs 0.12";

/// Models that use the `cl100k_base` encoding.
const CL100K_MODELS: &[&str] = &[
    "gpt-4",
    "gpt-4-32k",
    "gpt-4-turbo",
    "gpt-4-0613",
    "gpt-3.5-turbo",
    "gpt-3.5-turbo-16k",
    "gpt-3.5-turbo-instruct",
    "gpt-35-turbo",
];

/// Models that use the `o200k_base` encoding.
const O200K_MODELS: &[&str] = &[
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4.1",
    "gpt-4.1-mini",
    "gpt-4.1-nano",
    "gpt-4.5-preview",
    "o1",
    "o1-mini",
    "o1-preview",
    "o3",
    "o3-mini",
    "o4-mini",
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-nano",
    "gpt-5-codex",
    "gpt-5.1-codex",
    "gpt-5.1-codex-mini",
    "gpt-5.2-codex",
];

/// Models that use the `r50k_base` encoding.
const R50K_MODELS: &[&str] = &["gpt2", "text-davinci-002", "text-davinci-003"];

/// Models that use the `p50k_base` encoding.
const P50K_MODELS: &[&str] = &["text-davinci-001", "code-davinci-001"];

/// Errors that can occur while preparing an OpenAI tokenizer.
#[derive(Debug, thiserror::Error)]
pub enum TokenizerError {
    /// No published encoding is known for the requested model.
    #[error("no OpenAI tokenizer is known for model `{model}`")]
    UnsupportedModel { model: String },
    /// The encoding is not one of the published ones.
    #[error("`{encoding}` is not a known OpenAI encoding")]
    UnsupportedEncoding { encoding: String },
    /// The vocabulary could not be loaded.
    #[error("the vocabulary for encoding `{encoding}` could not be loaded")]
    AssetUnavailable { encoding: String },
}

/// Name of an encoding family.
pub fn encoding_name(encoding: Tokenizer) -> &'static str {
    match encoding {
        Tokenizer::O200kHarmony => "o200k_harmony",
        Tokenizer::O200kBase => "o200k_base",
        Tokenizer::Cl100kBase => "cl100k_base",
        Tokenizer::P50kBase => "p50k_base",
        Tokenizer::R50kBase => "r50k_base",
        Tokenizer::P50kEdit => "p50k_edit",
        Tokenizer::Gpt2 => "gpt2",
    }
}

/// Models that are known to use an encoding.
pub fn models_for_encoding(encoding: Tokenizer) -> Vec<String> {
    let models: &[&str] = match encoding {
        Tokenizer::Cl100kBase => CL100K_MODELS,
        Tokenizer::O200kBase | Tokenizer::O200kHarmony => O200K_MODELS,
        Tokenizer::R50kBase | Tokenizer::Gpt2 => R50K_MODELS,
        Tokenizer::P50kBase | Tokenizer::P50kEdit => P50K_MODELS,
    };
    models.iter().map(|model| model.to_string()).collect()
}

/// Every encoding this crate can count with, together with the models that
/// use it.
pub fn supported_encodings() -> Vec<(&'static str, Vec<String>)> {
    vec![
        (
            encoding_name(Tokenizer::Cl100kBase),
            models_for_encoding(Tokenizer::Cl100kBase),
        ),
        (
            encoding_name(Tokenizer::O200kBase),
            models_for_encoding(Tokenizer::O200kBase),
        ),
        (
            encoding_name(Tokenizer::R50kBase),
            models_for_encoding(Tokenizer::R50kBase),
        ),
        (
            encoding_name(Tokenizer::P50kBase),
            models_for_encoding(Tokenizer::P50kBase),
        ),
    ]
}

/// Counts tokens with an OpenAI byte-pair-encoding vocabulary.
pub struct OpenAiTokenizer {
    info: TokenizerInfo,
    bpe: &'static CoreBPE,
}

impl OpenAiTokenizer {
    /// Builds the tokenizer that OpenAI uses for `model`.
    pub fn for_model(model: &str) -> Result<Self, TokenizerError> {
        let encoding = get_tokenizer(model).ok_or_else(|| TokenizerError::UnsupportedModel {
            model: model.to_string(),
        })?;
        Self::for_encoding(encoding)
    }

    /// Builds the tokenizer of an encoding family.
    pub fn for_encoding(encoding: Tokenizer) -> Result<Self, TokenizerError> {
        let name = encoding_name(encoding);
        let bpe = tiktoken_rs::bpe_for_tokenizer(encoding).map_err(|_| {
            TokenizerError::AssetUnavailable {
                encoding: name.to_string(),
            }
        })?;

        let info = TokenizerInfo {
            id: format!("tiktoken:{name}"),
            version: IMPLEMENTATION.to_string(),
            encoding: name.to_string(),
            provider: "openai".to_string(),
            models: models_for_encoding(encoding),
            supported_inputs: vec![
                "utf-8 text".to_string(),
                "chat message content (without request framing)".to_string(),
            ],
            unsupported_inputs: vec![
                "binary data".to_string(),
                "non utf-8 files".to_string(),
                "images".to_string(),
                "pdf documents".to_string(),
                "chat request framing tokens".to_string(),
            ],
            assets: AssetSource::Bundled,
        };

        Ok(Self { info, bpe })
    }

    /// The token ids of `text`.
    ///
    /// The ids are the vocabulary ids of the underlying byte-pair encoding and
    /// are stable for a given encoding.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        self.bpe.encode_ordinary(text)
    }
}

impl LocalTokenizer for OpenAiTokenizer {
    fn name(&self) -> &str {
        &self.info.id
    }

    fn count_tokens(&self, text: &str) -> usize {
        self.bpe.encode_ordinary(text).len()
    }

    fn info(&self) -> TokenizerInfo {
        self.info.clone()
    }
}

/// Builds the local tokenizer for an OpenAI model.
pub fn tokenizer_for_model(model: &str) -> Result<Arc<dyn LocalTokenizer>, TokenizerError> {
    Ok(Arc::new(OpenAiTokenizer::for_model(model)?))
}

/// The encodings that can be compared with each other, with a model that uses
/// each of them.
pub fn comparable_encodings() -> Vec<(&'static str, &'static str)> {
    vec![
        ("cl100k_base", "gpt-4"),
        ("o200k_base", "gpt-4o"),
        ("r50k_base", "gpt2"),
        ("p50k_base", "text-davinci-001"),
    ]
}

/// Builds the local tokenizer of an encoding family, by name.
pub fn tokenizer_for_encoding(name: &str) -> Result<Arc<dyn LocalTokenizer>, TokenizerError> {
    let encoding = match name {
        "cl100k_base" => Tokenizer::Cl100kBase,
        "o200k_base" => Tokenizer::O200kBase,
        "r50k_base" => Tokenizer::R50kBase,
        "p50k_base" => Tokenizer::P50kBase,
        other => {
            return Err(TokenizerError::UnsupportedEncoding {
                encoding: other.to_string(),
            })
        }
    };

    Ok(Arc::new(OpenAiTokenizer::for_encoding(encoding)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference examples from the official tiktoken README.
    #[test]
    fn test_cl100k_base_reference_token_ids() {
        let tokenizer = OpenAiTokenizer::for_model("gpt-4").unwrap();

        assert_eq!(tokenizer.encode("hello world"), vec![15339, 1917]);
        assert_eq!(
            tokenizer.encode("tiktoken is great!"),
            vec![83, 1609, 5963, 374, 2294, 0]
        );
    }

    /// The comma is a piece of its own in `cl100k_base`, so the ids are the
    /// ones of the published vocabulary: `Hello`, `,`, ` world` and `!`.
    #[test]
    fn test_cl100k_base_splits_punctuation() {
        let tokenizer = OpenAiTokenizer::for_model("gpt-4").unwrap();

        assert_eq!(tokenizer.encode("Hello, world!"), vec![9906, 11, 1917, 0]);
    }

    /// The reference example from the official tiktoken README.
    #[test]
    fn test_o200k_base_reference_token_ids() {
        let tokenizer = OpenAiTokenizer::for_model("gpt-4o").unwrap();

        assert_eq!(tokenizer.encode("hello world"), vec![24912, 2375]);
    }

    #[test]
    fn test_token_counts_match_the_reference_token_ids() {
        let tokenizer = OpenAiTokenizer::for_model("gpt-4").unwrap();

        assert_eq!(tokenizer.count_tokens("hello world"), 2);
        assert_eq!(tokenizer.count_tokens("Hello, world!"), 4);
        assert_eq!(tokenizer.count_tokens("tiktoken is great!"), 6);
        assert_eq!(tokenizer.count_tokens(""), 0);
    }

    #[test]
    fn test_models_resolve_to_the_expected_encoding() {
        assert_eq!(
            OpenAiTokenizer::for_model("gpt-4").unwrap().info().encoding,
            "cl100k_base"
        );
        assert_eq!(
            OpenAiTokenizer::for_model("gpt-3.5-turbo")
                .unwrap()
                .info()
                .encoding,
            "cl100k_base"
        );
        assert_eq!(
            OpenAiTokenizer::for_model("gpt-4o")
                .unwrap()
                .info()
                .encoding,
            "o200k_base"
        );
        assert_eq!(
            OpenAiTokenizer::for_model("gpt-5").unwrap().info().encoding,
            "o200k_base"
        );
        assert_eq!(
            OpenAiTokenizer::for_model("gpt-5-codex")
                .unwrap()
                .info()
                .encoding,
            "o200k_base"
        );
        assert_eq!(
            OpenAiTokenizer::for_model("o3-mini")
                .unwrap()
                .info()
                .encoding,
            "o200k_base"
        );
    }

    #[test]
    fn test_unknown_models_are_rejected() {
        let error = match OpenAiTokenizer::for_model("claude-opus-5-5") {
            Err(error) => error,
            Ok(_) => panic!("a Claude model must not resolve to an OpenAI tokenizer"),
        };

        assert!(matches!(error, TokenizerError::UnsupportedModel { .. }));
    }

    #[test]
    fn test_tokenizer_info_records_the_provenance() {
        let info = OpenAiTokenizer::for_model("gpt-4o").unwrap().info();

        assert_eq!(info.id, "tiktoken:o200k_base");
        assert_eq!(info.provider, "openai");
        assert_eq!(info.assets, AssetSource::Bundled);
        assert!(info.is_offline());
        assert!(info.models.iter().any(|model| model == "gpt-4o"));
        assert!(info
            .unsupported_inputs
            .iter()
            .any(|kind| kind == "chat request framing tokens"));
    }
}
