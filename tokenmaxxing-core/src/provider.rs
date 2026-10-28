//! Provider adapters for authoritative token counts.
//!
//! A local tokenizer can only be exact for the provider that published its
//! vocabulary. Anthropic does not publish Claude's tokenizer, so the only
//! authoritative source for Claude models is the provider's own counting
//! endpoint.
//!
//! The HTTP layer sits behind the [`HttpTransport`] trait so that the adapter
//! can be tested without a network connection.

use crate::tokenizer::{ProviderError, ProviderInfo, TokenCountProvider};
use std::sync::Arc;

/// The providers this crate knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProviderKind {
    /// OpenAI. Its byte-pair-encoding vocabularies are published, so a local
    /// tokenizer can count exactly.
    OpenAi,
    /// Anthropic. The tokenizer is not published, so counting has to go
    /// through the provider API.
    Anthropic,
}

impl ProviderKind {
    /// Identifier of the provider.
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderKind::OpenAi => "openai",
            ProviderKind::Anthropic => "anthropic",
        }
    }

    /// Parses a provider identifier.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" | "open-ai" => Some(ProviderKind::OpenAi),
            "anthropic" | "claude" => Some(ProviderKind::Anthropic),
            _ => None,
        }
    }

    /// Whether a tokenizer with a published vocabulary exists for this
    /// provider.
    pub fn has_local_tokenizer(self) -> bool {
        matches!(self, ProviderKind::OpenAi)
    }
}

/// Errors of the HTTP layer.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TransportError {
    /// The request could not be sent or the answer could not be read.
    #[error("the request could not be completed: {0}")]
    Network(String),
    /// The provider answered with an error status.
    #[error("the provider answered with status {status}: {message}")]
    Status { status: u16, message: String },
}

/// The HTTP surface the adapters need.
///
/// Keeping it this small makes the adapters testable without a network and
/// keeps the core crate free of a concrete HTTP client.
pub trait HttpTransport: Send + Sync {
    /// Sends `body` as JSON to `url` and returns the response body.
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &serde_json::Value,
    ) -> Result<String, TransportError>;
}

/// The default transport, built on the HTTP client of the `provider-api`
/// feature.
#[cfg(feature = "provider-api")]
pub struct UreqTransport {
    agent: ureq::Agent,
}

#[cfg(feature = "provider-api")]
impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "provider-api")]
impl UreqTransport {
    /// Builds a transport that reports error statuses as responses, so that
    /// the error body stays readable.
    pub fn new() -> Self {
        let config = ureq::config::Config::builder()
            .http_status_as_error(false)
            .build();
        Self {
            agent: ureq::Agent::new_with_config(config),
        }
    }
}

#[cfg(feature = "provider-api")]
impl HttpTransport for UreqTransport {
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &serde_json::Value,
    ) -> Result<String, TransportError> {
        let mut request = self.agent.post(url);
        for (name, value) in headers {
            request = request.header(name.as_str(), value.as_str());
        }

        let response = request
            .send_json(body)
            .map_err(|error| TransportError::Network(error.to_string()))?;

        let status = response.status().as_u16();
        let text = response
            .into_body()
            .read_to_string()
            .map_err(|error| TransportError::Network(error.to_string()))?;

        if (200..300).contains(&status) {
            Ok(text)
        } else {
            Err(TransportError::Status {
                status,
                message: api_error_message(&text).unwrap_or(text),
            })
        }
    }
}

/// Extracts the message of an Anthropic style error response.
fn api_error_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(|message| message.as_str())
        .map(|message| message.to_string())
}

/// Counts tokens with Anthropic's own counting endpoint.
///
/// Anthropic does not publish Claude's tokenizer, so the
/// `messages/count_tokens` endpoint is the only authoritative source for the
/// number of tokens a Claude model uses.
pub struct AnthropicTokenCounter {
    model: String,
    api_key: String,
    base_url: String,
    transport: Arc<dyn HttpTransport>,
}

impl AnthropicTokenCounter {
    /// Model that is used when the caller does not name one.
    pub const DEFAULT_MODEL: &str = "claude-opus-5-5";

    /// Default endpoint of the Anthropic API.
    pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

    /// Version header of the Anthropic API.
    pub const API_VERSION: &str = "2023-06-01";

    /// Builds a counter for `model`.
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            api_key: api_key.into(),
            base_url: Self::DEFAULT_BASE_URL.to_string(),
            transport: default_transport(),
        }
    }

    /// Replaces the transport, which is what the tests use.
    pub fn with_transport(mut self, transport: Arc<dyn HttpTransport>) -> Self {
        self.transport = transport;
        self
    }

    /// Replaces the endpoint.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// The endpoint that is called.
    pub fn endpoint(&self) -> String {
        format!(
            "{}/v1/messages/count_tokens",
            self.base_url.trim_end_matches('/')
        )
    }

    /// The endpoint of the default API.
    pub fn endpoint_for_model() -> String {
        format!("{}/v1/messages/count_tokens", Self::DEFAULT_BASE_URL)
    }

    fn request_body(&self, text: &str) -> serde_json::Value {
        serde_json::json!({
            "model": self.model,
            "messages": [{ "role": "user", "content": text }],
        })
    }

    fn request_headers(&self) -> Vec<(String, String)> {
        vec![
            ("x-api-key".to_string(), self.api_key.clone()),
            (
                "anthropic-version".to_string(),
                Self::API_VERSION.to_string(),
            ),
            ("content-type".to_string(), "application/json".to_string()),
        ]
    }

    fn map_error(&self, error: TransportError) -> ProviderError {
        match error {
            TransportError::Network(message) => ProviderError::Request(message),
            TransportError::Status { status, message } => match status {
                401 | 403 => ProviderError::Authentication {
                    provider: "anthropic".to_string(),
                },
                404 => ProviderError::UnsupportedModel {
                    provider: "anthropic".to_string(),
                    model: self.model.clone(),
                },
                _ => ProviderError::Request(format!("HTTP {status}: {message}")),
            },
        }
    }
}

#[cfg(feature = "provider-api")]
fn default_transport() -> Arc<dyn HttpTransport> {
    Arc::new(UreqTransport::new())
}

#[cfg(not(feature = "provider-api"))]
fn default_transport() -> Arc<dyn HttpTransport> {
    Arc::new(UnavailableTransport)
}

/// Transport that reports the missing feature instead of pretending to work.
#[cfg(not(feature = "provider-api"))]
pub struct UnavailableTransport;

#[cfg(not(feature = "provider-api"))]
impl HttpTransport for UnavailableTransport {
    fn post_json(
        &self,
        _url: &str,
        _headers: &[(String, String)],
        _body: &serde_json::Value,
    ) -> Result<String, TransportError> {
        Err(TransportError::Network(
            "the `provider-api` feature is not enabled".to_string(),
        ))
    }
}

impl TokenCountProvider for AnthropicTokenCounter {
    fn provider(&self) -> &str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn count_tokens(&self, text: &str) -> Result<usize, ProviderError> {
        if self.api_key.trim().is_empty() {
            return Err(ProviderError::NotConfigured(
                "the Anthropic API key is empty; set ANTHROPIC_API_KEY".to_string(),
            ));
        }

        let body = self.request_body(text);
        let response = self
            .transport
            .post_json(&self.endpoint(), &self.request_headers(), &body)
            .map_err(|error| self.map_error(error))?;

        let value: serde_json::Value = serde_json::from_str(&response)
            .map_err(|error| ProviderError::MalformedResponse(format!("{error}: {response}")))?;

        value
            .get("input_tokens")
            .and_then(serde_json::Value::as_u64)
            .map(|tokens| tokens as usize)
            .ok_or_else(|| {
                ProviderError::MalformedResponse(format!(
                    "the response has no input_tokens field: {response}"
                ))
            })
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "anthropic:messages/count_tokens".to_string(),
            provider: "anthropic".to_string(),
            model: self.model.clone(),
            endpoint: self.endpoint(),
            authoritative: true,
            caveats: vec![
                "Anthropic documents the count as an estimate: the number of input tokens \
                 actually billed can differ by a small amount."
                    .to_string(),
                "Claude 4.7 and later models use a newer tokenizer that produces roughly \
                 30 percent more tokens than the models before it."
                    .to_string(),
            ],
            unsupported_inputs: vec![
                "server tools such as web search, web fetch, code execution and tool search"
                    .to_string(),
                "the MCP connector".to_string(),
                "image and document blocks with a url or file source".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A transport that answers with a fixed body.
    struct FakeTransport {
        response: Result<String, TransportError>,
        seen: std::sync::Mutex<Option<SeenRequest>>,
    }

    /// What a transport was asked to send.
    type SeenRequest = (String, Vec<(String, String)>, serde_json::Value);

    impl FakeTransport {
        fn with_response(response: Result<String, TransportError>) -> Self {
            Self {
                response,
                seen: std::sync::Mutex::new(None),
            }
        }
    }

    impl HttpTransport for FakeTransport {
        fn post_json(
            &self,
            url: &str,
            headers: &[(String, String)],
            body: &serde_json::Value,
        ) -> Result<String, TransportError> {
            *self.seen.lock().unwrap() = Some((url.to_string(), headers.to_vec(), body.clone()));
            self.response.clone()
        }
    }

    #[test]
    fn test_count_tokens_uses_the_documented_endpoint_and_headers() {
        let transport = FakeTransport::with_response(Ok(r#"{"input_tokens": 14}"#.to_string()));
        let counter = AnthropicTokenCounter::new("claude-opus-5-5", "test-key")
            .with_transport(Arc::new(transport));

        let tokens = counter.count_tokens("Hello, Claude").unwrap();

        assert_eq!(tokens, 14);
        assert_eq!(
            counter.endpoint(),
            "https://api.anthropic.com/v1/messages/count_tokens"
        );
    }

    #[test]
    fn test_request_carries_the_model_and_the_text() {
        let transport = Arc::new(FakeTransport::with_response(Ok(
            r#"{"input_tokens": 3}"#.to_string()
        )));
        let counter = AnthropicTokenCounter::new("claude-opus-5-5", "test-key")
            .with_transport(transport.clone());

        counter.count_tokens("Merhaba").unwrap();

        let seen = transport.seen.lock().unwrap().clone().unwrap();
        assert_eq!(seen.0, "https://api.anthropic.com/v1/messages/count_tokens");
        assert_eq!(seen.1[0].0, "x-api-key");
        assert_eq!(seen.1[0].1, "test-key");
        assert_eq!(seen.1[1].0, "anthropic-version");
        assert_eq!(seen.1[1].1, "2023-06-01");
        assert_eq!(seen.2["model"], "claude-opus-5-5");
        assert_eq!(seen.2["messages"][0]["role"], "user");
        assert_eq!(seen.2["messages"][0]["content"], "Merhaba");
    }

    #[test]
    fn test_missing_api_key_is_reported() {
        let counter = AnthropicTokenCounter::new("claude-opus-5-5", "  ");
        let error = counter.count_tokens("hello").unwrap_err();

        assert!(matches!(error, ProviderError::NotConfigured(_)));
    }

    #[test]
    fn test_invalid_credentials_are_reported_as_authentication_errors() {
        let transport = FakeTransport::with_response(Err(TransportError::Status {
            status: 401,
            message: "invalid x-api-key".to_string(),
        }));
        let counter = AnthropicTokenCounter::new("claude-opus-5-5", "bad-key")
            .with_transport(Arc::new(transport));

        let error = counter.count_tokens("hello").unwrap_err();

        assert!(matches!(error, ProviderError::Authentication { .. }));
    }

    #[test]
    fn test_unknown_models_are_reported_as_unsupported() {
        let transport = FakeTransport::with_response(Err(TransportError::Status {
            status: 404,
            message: "model not found".to_string(),
        }));
        let counter = AnthropicTokenCounter::new("claude-nope", "test-key")
            .with_transport(Arc::new(transport));

        let error = counter.count_tokens("hello").unwrap_err();

        assert!(matches!(error, ProviderError::UnsupportedModel { .. }));
    }

    #[test]
    fn test_malformed_responses_are_reported() {
        let transport = FakeTransport::with_response(Ok("not json".to_string()));
        let counter = AnthropicTokenCounter::new("claude-opus-5-5", "test-key")
            .with_transport(Arc::new(transport));

        let error = counter.count_tokens("hello").unwrap_err();

        assert!(matches!(error, ProviderError::MalformedResponse(_)));
    }

    #[test]
    fn test_provider_info_records_the_caveats() {
        let counter = AnthropicTokenCounter::new("claude-opus-5-5", "test-key");
        let info = counter.info();

        assert!(info.authoritative);
        assert_eq!(info.model, "claude-opus-5-5");
        assert!(!info.caveats.is_empty());
        assert!(!info.unsupported_inputs.is_empty());
    }

    #[test]
    fn test_provider_kind_parsing() {
        assert_eq!(ProviderKind::parse("openai"), Some(ProviderKind::OpenAi));
        assert_eq!(
            ProviderKind::parse("Anthropic"),
            Some(ProviderKind::Anthropic)
        );
        assert_eq!(ProviderKind::parse("gemini"), None);
        assert!(ProviderKind::OpenAi.has_local_tokenizer());
        assert!(!ProviderKind::Anthropic.has_local_tokenizer());
    }
}
