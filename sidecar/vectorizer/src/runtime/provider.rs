//! Async embedding-provider clients.
//!
//! Each provider implements [`AsyncEmbeddingProvider::embed`] for a batch of
//! input strings. Concrete clients use [`reqwest`] with the rustls-tls feature
//! so the binary stays portable across glibc and musl bases.

// FEATURE: A3

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

/// Result of an embedding call returned by every provider.
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddingResponse {
    pub embeddings: Vec<Vec<f32>>,
    pub prompt_tokens: u64,
    pub total_tokens: u64,
}

#[async_trait]
pub trait AsyncEmbeddingProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn embed(
        &self,
        model: &str,
        inputs: &[String],
    ) -> Result<EmbeddingResponse, EmbeddingError>;
}

/// A registry that owns provider implementations keyed by their stable name.
#[derive(Clone, Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn AsyncEmbeddingProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, provider: Arc<dyn AsyncEmbeddingProvider>) {
        self.providers.insert(provider.name().to_string(), provider);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn AsyncEmbeddingProvider>> {
        self.providers.get(name).cloned()
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.providers.keys().cloned().collect();
        names.sort();
        names
    }
}

/// Deterministic in-process provider used for tests, the smoke script, and the
/// `AI_BLAISE_VECTORIZER_PROVIDER_MODE=mock` runtime profile.
#[derive(Debug, Clone)]
pub struct MockProvider {
    name: String,
    dimensions: usize,
    cost_micros_per_token: u64,
}

impl MockProvider {
    pub fn new(name: impl Into<String>, dimensions: usize, cost_micros_per_token: u64) -> Self {
        Self {
            name: name.into(),
            dimensions,
            cost_micros_per_token,
        }
    }

    pub fn cost_micros_per_token(&self) -> u64 {
        self.cost_micros_per_token
    }
}

#[async_trait]
impl AsyncEmbeddingProvider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn embed(
        &self,
        _model: &str,
        inputs: &[String],
    ) -> Result<EmbeddingResponse, EmbeddingError> {
        if inputs.is_empty() {
            return Err(EmbeddingError::EmptyInput);
        }
        let mut embeddings = Vec::with_capacity(inputs.len());
        let mut total_tokens = 0u64;
        for input in inputs {
            let token_count = estimate_tokens(input);
            total_tokens += token_count;
            let mut vector = Vec::with_capacity(self.dimensions);
            for index in 0..self.dimensions {
                let component = (token_count as f32 + index as f32) / 1000.0;
                vector.push(component);
            }
            embeddings.push(vector);
        }
        Ok(EmbeddingResponse {
            embeddings,
            prompt_tokens: total_tokens,
            total_tokens,
        })
    }
}

fn estimate_tokens(input: &str) -> u64 {
    // Mirrors the tiktoken byte-pair approximation used by the SQL extension:
    // four bytes of UTF-8 input correspond to roughly one token.
    let bytes = input.len();
    (bytes.div_ceil(4)) as u64
}

/// Configuration shared by every HTTP provider.
#[derive(Debug, Clone)]
pub struct HttpProviderConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub user_agent: String,
    pub timeout: Duration,
}

impl HttpProviderConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: None,
            user_agent: "ai-blaise-citus-sidecar-vectorizer/0.1.0".to_string(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

fn build_http_client(config: &HttpProviderConfig) -> Result<Client, EmbeddingError> {
    Client::builder()
        .user_agent(config.user_agent.clone())
        .timeout(config.timeout)
        .build()
        .map_err(|error| EmbeddingError::Transport(error.to_string()))
}

/// OpenAI / Azure OpenAI / vLLM embedding client (they share the JSON shape).
#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    name: String,
    client: Client,
    base_url: String,
    api_key: Option<String>,
    cost_micros_per_token: u64,
}

impl OpenAiProvider {
    pub const DEFAULT_BASE_URL: &'static str = "https://api.openai.com/v1";

    pub fn new(
        name: impl Into<String>,
        config: HttpProviderConfig,
        cost_micros_per_token: u64,
    ) -> Result<Self, EmbeddingError> {
        let client = build_http_client(&config)?;
        Ok(Self {
            name: name.into(),
            client,
            base_url: trim_trailing_slash(&config.base_url),
            api_key: config.api_key,
            cost_micros_per_token,
        })
    }

    pub fn cost_micros_per_token(&self) -> u64 {
        self.cost_micros_per_token
    }
}

#[async_trait]
impl AsyncEmbeddingProvider for OpenAiProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn embed(
        &self,
        model: &str,
        inputs: &[String],
    ) -> Result<EmbeddingResponse, EmbeddingError> {
        if inputs.is_empty() {
            return Err(EmbeddingError::EmptyInput);
        }
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(api_key) = &self.api_key {
            let value = HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|error| EmbeddingError::Configuration(error.to_string()))?;
            headers.insert(AUTHORIZATION, value);
        }

        let url = format!("{}/embeddings", self.base_url);
        let body = OpenAiRequest {
            model: model.to_string(),
            input: inputs.to_vec(),
        };

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|error| EmbeddingError::Transport(error.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let detail = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::Provider {
                status: status.as_u16(),
                detail,
            });
        }
        let parsed: OpenAiResponse = response
            .json()
            .await
            .map_err(|error| EmbeddingError::Decode(error.to_string()))?;
        Ok(parsed.into_response())
    }
}

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    data: Vec<OpenAiEmbedding>,
    usage: OpenAiUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAiEmbedding {
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u64,
    total_tokens: u64,
}

impl OpenAiResponse {
    fn into_response(self) -> EmbeddingResponse {
        EmbeddingResponse {
            embeddings: self.data.into_iter().map(|entry| entry.embedding).collect(),
            prompt_tokens: self.usage.prompt_tokens,
            total_tokens: self.usage.total_tokens,
        }
    }
}

/// Voyage AI client.
#[derive(Debug, Clone)]
pub struct VoyageProvider {
    name: String,
    client: Client,
    base_url: String,
    api_key: Option<String>,
    cost_micros_per_token: u64,
}

impl VoyageProvider {
    pub const DEFAULT_BASE_URL: &'static str = "https://api.voyageai.com/v1";

    pub fn new(
        name: impl Into<String>,
        config: HttpProviderConfig,
        cost_micros_per_token: u64,
    ) -> Result<Self, EmbeddingError> {
        let client = build_http_client(&config)?;
        Ok(Self {
            name: name.into(),
            client,
            base_url: trim_trailing_slash(&config.base_url),
            api_key: config.api_key,
            cost_micros_per_token,
        })
    }

    pub fn cost_micros_per_token(&self) -> u64 {
        self.cost_micros_per_token
    }
}

#[async_trait]
impl AsyncEmbeddingProvider for VoyageProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn embed(
        &self,
        model: &str,
        inputs: &[String],
    ) -> Result<EmbeddingResponse, EmbeddingError> {
        if inputs.is_empty() {
            return Err(EmbeddingError::EmptyInput);
        }
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(api_key) = &self.api_key {
            let value = HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|error| EmbeddingError::Configuration(error.to_string()))?;
            headers.insert(AUTHORIZATION, value);
        }

        let url = format!("{}/embeddings", self.base_url);
        let body = VoyageRequest {
            model: model.to_string(),
            input: inputs.to_vec(),
            input_type: "document".to_string(),
        };

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|error| EmbeddingError::Transport(error.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let detail = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::Provider {
                status: status.as_u16(),
                detail,
            });
        }
        let parsed: VoyageResponse = response
            .json()
            .await
            .map_err(|error| EmbeddingError::Decode(error.to_string()))?;
        Ok(parsed.into_response())
    }
}

#[derive(Debug, Serialize)]
struct VoyageRequest {
    model: String,
    input: Vec<String>,
    input_type: String,
}

#[derive(Debug, Deserialize)]
struct VoyageResponse {
    data: Vec<VoyageEmbedding>,
    usage: VoyageUsage,
}

#[derive(Debug, Deserialize)]
struct VoyageEmbedding {
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct VoyageUsage {
    total_tokens: u64,
}

impl VoyageResponse {
    fn into_response(self) -> EmbeddingResponse {
        EmbeddingResponse {
            embeddings: self.data.into_iter().map(|entry| entry.embedding).collect(),
            prompt_tokens: self.usage.total_tokens,
            total_tokens: self.usage.total_tokens,
        }
    }
}

/// Cohere client.
#[derive(Debug, Clone)]
pub struct CohereProvider {
    name: String,
    client: Client,
    base_url: String,
    api_key: Option<String>,
    cost_micros_per_token: u64,
}

impl CohereProvider {
    pub const DEFAULT_BASE_URL: &'static str = "https://api.cohere.com/v1";

    pub fn new(
        name: impl Into<String>,
        config: HttpProviderConfig,
        cost_micros_per_token: u64,
    ) -> Result<Self, EmbeddingError> {
        let client = build_http_client(&config)?;
        Ok(Self {
            name: name.into(),
            client,
            base_url: trim_trailing_slash(&config.base_url),
            api_key: config.api_key,
            cost_micros_per_token,
        })
    }

    pub fn cost_micros_per_token(&self) -> u64 {
        self.cost_micros_per_token
    }
}

#[async_trait]
impl AsyncEmbeddingProvider for CohereProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn embed(
        &self,
        model: &str,
        inputs: &[String],
    ) -> Result<EmbeddingResponse, EmbeddingError> {
        if inputs.is_empty() {
            return Err(EmbeddingError::EmptyInput);
        }
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(api_key) = &self.api_key {
            let value = HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|error| EmbeddingError::Configuration(error.to_string()))?;
            headers.insert(AUTHORIZATION, value);
        }

        let url = format!("{}/embed", self.base_url);
        let body = CohereRequest {
            model: model.to_string(),
            texts: inputs.to_vec(),
            input_type: "search_document".to_string(),
        };

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|error| EmbeddingError::Transport(error.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let detail = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::Provider {
                status: status.as_u16(),
                detail,
            });
        }
        let parsed: CohereResponse = response
            .json()
            .await
            .map_err(|error| EmbeddingError::Decode(error.to_string()))?;
        Ok(parsed.into_response())
    }
}

#[derive(Debug, Serialize)]
struct CohereRequest {
    model: String,
    texts: Vec<String>,
    input_type: String,
}

#[derive(Debug, Deserialize)]
struct CohereResponse {
    embeddings: Vec<Vec<f32>>,
    meta: CohereMeta,
}

#[derive(Debug, Deserialize)]
struct CohereMeta {
    billed_units: CohereBilledUnits,
}

#[derive(Debug, Deserialize)]
struct CohereBilledUnits {
    input_tokens: u64,
}

impl CohereResponse {
    fn into_response(self) -> EmbeddingResponse {
        EmbeddingResponse {
            embeddings: self.embeddings,
            prompt_tokens: self.meta.billed_units.input_tokens,
            total_tokens: self.meta.billed_units.input_tokens,
        }
    }
}

/// Ollama client (talks to a local `/api/embeddings` endpoint).
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    name: String,
    client: Client,
    base_url: String,
    cost_micros_per_token: u64,
}

impl OllamaProvider {
    pub const DEFAULT_BASE_URL: &'static str = "http://localhost:11434";

    pub fn new(
        name: impl Into<String>,
        config: HttpProviderConfig,
        cost_micros_per_token: u64,
    ) -> Result<Self, EmbeddingError> {
        let client = build_http_client(&config)?;
        Ok(Self {
            name: name.into(),
            client,
            base_url: trim_trailing_slash(&config.base_url),
            cost_micros_per_token,
        })
    }

    pub fn cost_micros_per_token(&self) -> u64 {
        self.cost_micros_per_token
    }
}

#[async_trait]
impl AsyncEmbeddingProvider for OllamaProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn embed(
        &self,
        model: &str,
        inputs: &[String],
    ) -> Result<EmbeddingResponse, EmbeddingError> {
        if inputs.is_empty() {
            return Err(EmbeddingError::EmptyInput);
        }

        let mut embeddings = Vec::with_capacity(inputs.len());
        let mut total_tokens = 0u64;
        for input in inputs {
            let url = format!("{}/api/embeddings", self.base_url);
            let body = OllamaRequest {
                model: model.to_string(),
                prompt: input.clone(),
            };
            let response = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|error| EmbeddingError::Transport(error.to_string()))?;
            let status = response.status();
            if !status.is_success() {
                let detail = response.text().await.unwrap_or_default();
                return Err(EmbeddingError::Provider {
                    status: status.as_u16(),
                    detail,
                });
            }
            let parsed: OllamaResponse = response
                .json()
                .await
                .map_err(|error| EmbeddingError::Decode(error.to_string()))?;
            total_tokens += estimate_tokens(input);
            embeddings.push(parsed.embedding);
        }

        Ok(EmbeddingResponse {
            embeddings,
            prompt_tokens: total_tokens,
            total_tokens,
        })
    }
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    embedding: Vec<f32>,
}

fn trim_trailing_slash(value: &str) -> String {
    value.trim_end_matches('/').to_string()
}

/// Errors emitted by every async provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddingError {
    Configuration(String),
    Transport(String),
    Provider { status: u16, detail: String },
    Decode(String),
    EmptyInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingErrorKind {
    Retryable,
    Permanent,
}

impl EmbeddingError {
    pub fn kind(&self) -> EmbeddingErrorKind {
        match self {
            Self::Transport(_) => EmbeddingErrorKind::Retryable,
            Self::Provider { status, .. } if *status == 408 || *status == 409 || *status == 425 => {
                EmbeddingErrorKind::Retryable
            }
            Self::Provider { status, .. } if *status == 429 || *status >= 500 => {
                EmbeddingErrorKind::Retryable
            }
            Self::Configuration(_) | Self::Decode(_) | Self::EmptyInput => {
                EmbeddingErrorKind::Permanent
            }
            Self::Provider { .. } => EmbeddingErrorKind::Permanent,
        }
    }

    pub fn is_retryable(&self) -> bool {
        self.kind() == EmbeddingErrorKind::Retryable
    }
}

impl fmt::Display for EmbeddingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(detail) => {
                write!(formatter, "embedding provider misconfigured: {detail}")
            }
            Self::Transport(detail) => write!(formatter, "embedding provider transport: {detail}"),
            Self::Provider { status, detail } => {
                write!(formatter, "embedding provider returned {status}: {detail}")
            }
            Self::Decode(detail) => write!(formatter, "embedding provider decode failed: {detail}"),
            Self::EmptyInput => write!(formatter, "embedding provider requires at least one input"),
        }
    }
}

impl Error for EmbeddingError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_provider_returns_deterministic_embeddings() {
        let provider = MockProvider::new("mock", 4, 7);
        let response = provider
            .embed("any-model", &["hello".to_string(), "world!".to_string()])
            .await
            .expect("response");

        assert_eq!(response.embeddings.len(), 2);
        assert_eq!(response.embeddings[0].len(), 4);
        assert_eq!(response.prompt_tokens, response.total_tokens);
        assert!(response.total_tokens > 0);
    }

    #[tokio::test]
    async fn mock_provider_rejects_empty_input() {
        let provider = MockProvider::new("mock", 4, 7);
        let error = provider
            .embed("model", &[])
            .await
            .expect_err("empty input should fail");
        assert_eq!(error, EmbeddingError::EmptyInput);
    }

    #[test]
    fn registry_lists_providers_sorted() {
        let mut registry = ProviderRegistry::new();
        registry.insert(Arc::new(MockProvider::new("zeta", 2, 1)));
        registry.insert(Arc::new(MockProvider::new("alpha", 2, 1)));
        assert_eq!(
            registry.names(),
            vec!["alpha".to_string(), "zeta".to_string()]
        );
        assert!(registry.get("alpha").is_some());
        assert!(registry.get("missing").is_none());
    }

    #[test]
    fn classifies_retryable_and_permanent_errors() {
        assert!(EmbeddingError::Transport("timeout".into()).is_retryable());
        assert!(EmbeddingError::Provider {
            status: 429,
            detail: "rate limited".into(),
        }
        .is_retryable());
        assert!(!EmbeddingError::Provider {
            status: 401,
            detail: "bad key".into(),
        }
        .is_retryable());
        assert!(!EmbeddingError::Decode("shape changed".into()).is_retryable());
    }

    #[test]
    fn estimate_tokens_handles_ascii_and_unicode() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("hi"), 1);
        assert_eq!(estimate_tokens("hello"), 2);
        // Each char is 3 bytes; "三人" => 6 bytes => ceil(6/4) = 2.
        assert_eq!(estimate_tokens("三人"), 2);
    }
}
