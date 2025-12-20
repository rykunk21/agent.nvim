use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmProviderType {
    Ollama {
        endpoint: String,
        model: String,
    },
    OpenAI {
        api_key: String,
        model: String,
        base_url: Option<String>,
    },
    Anthropic {
        api_key: String,
        model: String,
    },
}

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Request error: {0}")]
    RequestError(String),
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),
}

pub type LlmResult<T> = Result<T, LlmError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub prompt: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub stop_reason: String,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub message: String,
    pub response_time_ms: u64,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a request to the LLM provider
    async fn request(&self, req: LlmRequest) -> LlmResult<LlmResponse>;

    /// Stream a request to the LLM provider
    async fn stream_request(
        &self,
        req: LlmRequest,
    ) -> LlmResult<Box<dyn futures::Stream<Item = LlmResult<String>> + Unpin + Send>>;

    /// Check health of the provider
    async fn health_check(&self) -> LlmResult<HealthStatus>;

    /// Get provider name
    fn name(&self) -> &str;

    /// Get provider type
    fn provider_type(&self) -> LlmProviderType;
}

pub struct LlmProviderManager {
    provider: Arc<dyn LlmProvider>,
}

impl LlmProviderManager {
    /// Create a new LLM provider manager
    pub async fn new(provider_type: LlmProviderType) -> LlmResult<Self> {
        let provider: Arc<dyn LlmProvider> = match provider_type {
            LlmProviderType::Ollama { endpoint, model } => {
                Arc::new(crate::llm::ollama::OllamaProvider::new(endpoint, model).await?)
            }
            LlmProviderType::OpenAI {
                api_key,
                model,
                base_url,
            } => {
                Arc::new(crate::llm::openai::OpenAiProvider::new(api_key, model, base_url).await?)
            }
            LlmProviderType::Anthropic { api_key, model } => {
                Arc::new(crate::llm::anthropic::AnthropicProvider::new(api_key, model).await?)
            }
        };

        Ok(LlmProviderManager { provider })
    }

    /// Send a request to the LLM
    pub async fn request(&self, req: LlmRequest) -> LlmResult<LlmResponse> {
        self.provider.request(req).await
    }

    /// Stream a request to the LLM
    pub async fn stream_request(
        &self,
        req: LlmRequest,
    ) -> LlmResult<Box<dyn futures::Stream<Item = LlmResult<String>> + Unpin + Send>> {
        self.provider.stream_request(req).await
    }

    /// Check health of the provider
    pub async fn health_check(&self) -> LlmResult<HealthStatus> {
        self.provider.health_check().await
    }

    /// Get provider name
    pub fn name(&self) -> &str {
        self.provider.name()
    }

    /// Switch to a different provider
    pub async fn switch_provider(&mut self, provider_type: LlmProviderType) -> LlmResult<()> {
        let new_provider: Arc<dyn LlmProvider> = match provider_type {
            LlmProviderType::Ollama { endpoint, model } => {
                Arc::new(crate::llm::ollama::OllamaProvider::new(endpoint, model).await?)
            }
            LlmProviderType::OpenAI {
                api_key,
                model,
                base_url,
            } => {
                Arc::new(crate::llm::openai::OpenAiProvider::new(api_key, model, base_url).await?)
            }
            LlmProviderType::Anthropic { api_key, model } => {
                Arc::new(crate::llm::anthropic::AnthropicProvider::new(api_key, model).await?)
            }
        };

        // Verify the new provider is healthy before switching
        new_provider.health_check().await?;

        // SAFETY: We're in a single-threaded async context, so this is safe
        // In production, use Arc<Mutex<>> for thread-safe switching
        unsafe {
            let self_mut = self as *mut Self;
            (*self_mut).provider = new_provider;
        }

        Ok(())
    }
}
