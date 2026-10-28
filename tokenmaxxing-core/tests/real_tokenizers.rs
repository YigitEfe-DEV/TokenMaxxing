//! Regression tests for the real tokenizers.
//!
//! None of these tests touch the network. The live provider tests live at the
//! bottom of the file and are marked as ignored, so they only run when they
//! are asked for explicitly.

use std::io::Read;
use std::sync::Arc;
use tokenmaxxing_core::engine::{CountKind, EstimationMode, TokenEngine};
use tokenmaxxing_core::provider::{
    AnthropicTokenCounter, HttpTransport, ProviderKind, TransportError,
};
use tokenmaxxing_core::tokenizer::openai::{OpenAiTokenizer, TokenizerError};
use tokenmaxxing_core::tokenizer::{AssetSource, LocalTokenizer, TokenCountProvider};

/// A reader that hands out at most `chunk` bytes per read.
struct ChunkedReader<'a> {
    data: &'a [u8],
    chunk: usize,
    position: usize,
}

impl<'a> ChunkedReader<'a> {
    fn new(data: &'a [u8], chunk: usize) -> Self {
        Self {
            data,
            chunk: chunk.max(1),
            position: 0,
        }
    }
}

impl Read for ChunkedReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let remaining = &self.data[self.position..];
        let take = remaining.len().min(self.chunk).min(buffer.len());
        buffer[..take].copy_from_slice(&remaining[..take]);
        self.position += take;
        Ok(take)
    }
}

fn tokenizer(model: &str) -> OpenAiTokenizer {
    OpenAiTokenizer::for_model(model).expect("the model has a published tokenizer")
}

fn engine(model: &str) -> TokenEngine {
    TokenEngine::with_tokenizer(Arc::new(tokenizer(model)))
}

/// Texts that cover the shapes a repository is made of.
fn corpus() -> Vec<&'static str> {
    vec![
        "",
        " ",
        "\n\n\n",
        "Hello world!",
        "Merhaba dünya! Bu bir testtir.",
        "Türkçe karakterler: ğüşiöç âîû",
        "日本語のテキストです。",
        "emoji 🚀🚀 and symbols ∑∫ ≈ ±",
        "fn main() {\n    println!(\"hello\");\n}",
        "```rust\nlet x = 1;\n```",
        "# Başlık\n\n- madde bir\n- madde iki\n",
        r#"{"anahtar":"değer","sayı":42,"liste":[1,2,3]}"#,
        "!@#$%^&*()_+-=[]{}|;':\",./<>?",
        "supercalifragilisticexpialidocious",
        "https://example.com/path?query=value&other=1",
        "CamelCaseIdentifier snake_case_ident SCREAMING_CASE",
        "tab\tseparated\tvalues\n",
    ]
}

#[test]
fn test_cl100k_base_reference_token_ids() {
    let tokenizer = tokenizer("gpt-4");

    assert_eq!(tokenizer.encode("hello world"), vec![15339, 1917]);
    assert_eq!(
        tokenizer.encode("tiktoken is great!"),
        vec![83, 1609, 5963, 374, 2294, 0]
    );
}

#[test]
fn test_o200k_base_reference_token_ids() {
    let tokenizer = tokenizer("gpt-4o");

    assert_eq!(tokenizer.encode("hello world"), vec![24912, 2375]);
}

#[test]
fn test_counts_are_deterministic() {
    for text in corpus() {
        let tokenizer = tokenizer("gpt-4o");
        assert_eq!(
            tokenizer.count_tokens(text),
            tokenizer.count_tokens(text),
            "the count has to be stable for {text:?}"
        );
    }
}

#[test]
fn test_corpus_is_counted_by_the_tokenizer() {
    let tokenizer = tokenizer("gpt-4o");

    for text in corpus() {
        let count = tokenizer.count_tokens(text);

        assert!(
            count > 0 || text.is_empty(),
            "no tokens counted for {text:?}"
        );
        assert_eq!(count, tokenizer.encode(text).len());
    }
}

#[test]
fn test_turkish_text_is_counted_exactly() {
    let tokenizer = tokenizer("gpt-4o");
    let text = "Merhaba dünya! Bu bir testtir.";

    assert!(tokenizer.count_tokens(text) > 0);
    // The engine reports the same number and labels it exact.
    let count = engine("gpt-4o").count_text(text);
    assert_eq!(count.tokens, tokenizer.count_tokens(text));
    assert_eq!(count.kind, CountKind::ExactLocal);
    assert_eq!(count.chars, text.chars().count());
}

#[test]
fn test_unicode_text_is_counted_exactly() {
    let tokenizer = tokenizer("gpt-4o");
    let text = "日本語のテキストです。🚀🚀";

    let count = engine("gpt-4o").count_text(text);

    assert_eq!(count.tokens, tokenizer.count_tokens(text));
    assert_eq!(count.bytes, text.len());
    assert!(
        count.chars < count.bytes,
        "multibyte text has fewer characters than bytes"
    );
}

#[test]
fn test_source_code_is_counted_exactly() {
    let code = "fn main() {\n    let value = 42;\n    println!(\"{value}\");\n}\n";
    let tokenizer = tokenizer("gpt-4o");

    let count = engine("gpt-4o").count_text(code);

    assert_eq!(count.tokens, tokenizer.count_tokens(code));
    assert!(count.tokens > 10);
}

#[test]
fn test_markdown_and_json_are_counted_exactly() {
    let markdown = "# Başlık\n\n- madde bir\n- madde iki\n";
    let json = r#"{"anahtar":"değer","sayı":42}"#;
    let tokenizer = tokenizer("gpt-4o");

    assert_eq!(
        engine("gpt-4o").count_text(markdown).tokens,
        tokenizer.count_tokens(markdown)
    );
    assert_eq!(
        engine("gpt-4o").count_text(json).tokens,
        tokenizer.count_tokens(json)
    );
}

#[test]
fn test_empty_and_punctuation_only_input() {
    let tokenizer = tokenizer("gpt-4o");

    assert_eq!(tokenizer.count_tokens(""), 0);
    assert_eq!(engine("gpt-4o").count_text("").tokens, 0);
    assert!(tokenizer.count_tokens("!@#$%^&*()") > 0);
}

#[test]
fn test_long_input_is_counted_exactly() {
    let text = "The quick brown fox jumps over the lazy dog. 日本語のテキスト。\n".repeat(2000);
    let tokenizer = tokenizer("gpt-4o");

    let count = engine("gpt-4o").count_text(&text);

    assert_eq!(count.tokens, tokenizer.count_tokens(&text));
    assert!(count.tokens > 10_000);
}

#[test]
fn test_streaming_equals_the_full_input_for_a_local_tokenizer() {
    for text in corpus() {
        let engine = engine("gpt-4o");
        let expected = engine.count_text(text);

        for chunk in [1usize, 2, 3, 7, 64, 4096] {
            let reader = ChunkedReader::new(text.as_bytes(), chunk);
            let streamed = engine.count_stream(reader).unwrap();

            assert_eq!(streamed, expected, "chunk size {chunk}, text {text:?}");
        }
    }
}

#[test]
fn test_streaming_survives_split_utf8_sequences() {
    let text = "日本語のテキスト、 Türkçe karakterler, and emoji 🚀🚀🚀.";
    let engine = engine("gpt-4o");
    let expected = engine.count_text(text);

    for chunk in [1usize, 2, 3, 5] {
        let reader = ChunkedReader::new(text.as_bytes(), chunk);
        let streamed = engine.count_stream(reader).unwrap();

        assert_eq!(streamed, expected, "chunk size {chunk}");
        assert_eq!(streamed.chars, text.chars().count());
    }
}

#[test]
fn test_the_estimate_is_never_reported_as_exact() {
    let estimate = TokenEngine::new(EstimationMode::Fast).count_text("Hello world!");

    assert_eq!(estimate.kind, CountKind::Estimated);
    assert!(!estimate.kind.is_exact());
}

#[test]
fn test_unsupported_models_are_rejected() {
    let error = match OpenAiTokenizer::for_model("claude-opus-5-5") {
        Err(error) => error,
        Ok(_) => panic!("a Claude model must not resolve to an OpenAI tokenizer"),
    };

    assert!(matches!(error, TokenizerError::UnsupportedModel { .. }));
}

#[test]
fn test_the_vocabulary_is_bundled_and_works_offline() {
    let info = tokenizer("gpt-4o").info();

    assert_eq!(info.assets, AssetSource::Bundled);
    assert!(info.is_offline());
    assert_eq!(info.provider, "openai");
    assert_eq!(info.encoding, "o200k_base");
    assert!(info.models.iter().any(|model| model == "gpt-4o"));
}

#[test]
fn test_the_tokenizer_records_the_input_types_it_cannot_handle() {
    let info = tokenizer("gpt-4o").info();

    assert!(info
        .unsupported_inputs
        .iter()
        .any(|kind| kind.contains("binary")));
    assert!(info
        .unsupported_inputs
        .iter()
        .any(|kind| kind.contains("framing")));
}

/// A transport that answers with a fixed result.
struct FakeTransport {
    result: Result<String, TransportError>,
}

impl FakeTransport {
    fn ok(body: &str) -> Self {
        Self {
            result: Ok(body.to_string()),
        }
    }

    fn failing(error: TransportError) -> Self {
        Self { result: Err(error) }
    }
}

impl HttpTransport for FakeTransport {
    fn post_json(
        &self,
        _url: &str,
        _headers: &[(String, String)],
        _body: &serde_json::Value,
    ) -> Result<String, TransportError> {
        self.result.clone()
    }
}

#[test]
fn test_provider_reports_invalid_credentials() {
    let counter = AnthropicTokenCounter::new("claude-opus-5-5", "key").with_transport(Arc::new(
        FakeTransport::failing(TransportError::Status {
            status: 401,
            message: "invalid x-api-key".to_string(),
        }),
    ));

    let error = counter.count_tokens("hello").unwrap_err();

    assert!(matches!(
        error,
        tokenmaxxing_core::ProviderError::Authentication { .. }
    ));
}

#[test]
fn test_provider_reports_unsupported_models() {
    let counter = AnthropicTokenCounter::new("claude-nope", "key").with_transport(Arc::new(
        FakeTransport::failing(TransportError::Status {
            status: 404,
            message: "model not found".to_string(),
        }),
    ));

    let error = counter.count_tokens("hello").unwrap_err();

    assert!(matches!(
        error,
        tokenmaxxing_core::ProviderError::UnsupportedModel { .. }
    ));
}

#[test]
fn test_provider_reports_transport_failures() {
    let counter = AnthropicTokenCounter::new("claude-opus-5-5", "key").with_transport(Arc::new(
        FakeTransport::failing(TransportError::Network("connection refused".to_string())),
    ));

    let error = counter.count_tokens("hello").unwrap_err();

    assert!(matches!(
        error,
        tokenmaxxing_core::ProviderError::Request(_)
    ));
}

#[test]
fn test_provider_counts_are_labelled_as_provider_authoritative() {
    let counter = AnthropicTokenCounter::new("claude-opus-5-5", "key")
        .with_transport(Arc::new(FakeTransport::ok(r#"{"input_tokens": 14}"#)));

    let count = counter.count("Hello, Claude").unwrap();

    assert_eq!(count.tokens, 14);
    assert_eq!(count.kind, CountKind::ProviderApi);
    assert!(count.kind.is_exact());
}

#[test]
fn test_anthropic_has_no_local_tokenizer() {
    assert!(!ProviderKind::Anthropic.has_local_tokenizer());
    assert!(ProviderKind::OpenAi.has_local_tokenizer());
}

#[test]
fn test_the_count_of_a_provider_is_not_the_count_of_a_local_tokenizer() {
    // The OpenAI vocabulary says nothing about Claude, so the numbers may
    // differ and the report must not claim they are the same.
    let text = "Merhaba dünya! Bu bir testtir.";
    let local = tokenizer("gpt-4o").count_tokens(text);
    let provider = AnthropicTokenCounter::new("claude-opus-5-5", "key")
        .with_transport(Arc::new(FakeTransport::ok(&format!(
            r#"{{"input_tokens": {}}}"#,
            local + 7
        ))))
        .count_tokens(text)
        .unwrap();

    assert_ne!(local, provider);
}

/// Live provider test. It needs network access and a real API key, so it is
/// ignored by default.
///
/// Run it with `cargo test --test real_tokenizers -- --ignored`.
#[test]
#[ignore = "needs network access and ANTHROPIC_API_KEY"]
fn test_live_anthropic_count_tokens() {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY has to be set for the live test");
    let counter = AnthropicTokenCounter::new("claude-opus-5-5", api_key);

    let count = counter.count("Merhaba dünya!").unwrap();

    assert!(count.tokens > 0);
}
