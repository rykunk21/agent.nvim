use super::manager::{HealthStatus, LlmError, LlmProvider, LlmProviderType, LlmRequest, LlmResponse, LlmResult, TokenUsage};
use async_trait::async_trait;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: usize,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<AnthropicContent>,
    model: String,
    stop_reason: String,
    usage: AnthropicUsage,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicUsage {
    input_tokens: usize,
    output_tokens: usize,
}

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub async fn new(api_key: String, model: String) -> LlmResult<Self> {
        let client = reqwest::Client::new();

        let provider = AnthropicProvider {
            api_key,
            model,
            client,
        };

        // Verify connection
        provider.health_check().await?;

        Ok(provider)
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn request(&self, req: LlmRequest) -> LlmResult<LlmResponse> {
        let start = Instant::now();

        let mut messages = vec![];
        messages.push(AnthropicMessage {
            role: "user".to_string(),
            content: req.prompt,
        });

        let anthropic_req = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: req.max_tokens.unwrap_or(1024),
            messages,
            system: req.system_prompt,
            temperature: req.temperature,
        };

        let url = "https://api.anthropic.com/v1/messages";
        debug!("Sending request to Anthropic: {}", url);

        let response = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&anthropic_req)
            .send()
            .await
            .map_err(|e| LlmError::RequestError(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmError::RequestError(format!(
                "Anthropic returned status {}: {}",
                status, body
            )));
        }

        let anthropic_resp: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| LlmError::RequestError(format!("Failed to parse response: {}", e)))?;

        let elapsed = start.elapsed().as_millis() as u64;
        debug!("Anthropic request completed in {}ms", elapsed);

        let content = anthropic_resp
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        Ok(LlmResponse {
            content,
            stop_reason: anthropic_resp.stop_reason,
            usage: Some(TokenUsage {
                prompt_tokens: anthropic_resp.usage.input_tokens,
                completion_tokens: anthropic_resp.usage.output_tokens,
                total_tokens: anthropic_resp.usage.input_tokens + anthropic_resp.usage.output_tokens,
            }),
        })
    }

    async fn stream_request(
        &self,
        req: LlmRequest,
    ) -> LlmResult<Box<dyn futures::Stream<Item = LlmResult<String>> + Unpin + Send>> {
        let mut messages = vec![];
        messages.push(AnthropicMessage {
            role: "user".to_string(),
            content: req.prompt,
        });

        let anthropic_req = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: req.max_tokens.unwrap_or(1024),
            messages,
            system: req.system_prompt,
            temperature: req.temperature,
        };

        let url = "https://api.anthropic.com/v1/messages";
        debug!("Starting streaming request to Anthropic: {}", url);

        let response = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("stream", "true")
            .json(&anthropic_req)
            .send()
            .await
            .map_err(|e| LlmError::RequestError(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            return Err(LlmError::RequestError(format!(
                "Anthropic returned status: {}",
                response.status()
            )));
        }

        // For now, return an empty stream as a placeholder
        // In production, this would properly handle streaming responses
        let stream: Box<dyn futures::Stream<Item = LlmResult<String>> + Unpin + Send> =
            Box::new(futures::stream::empty());

        Ok(stream)
    }

    async fn health_check(&self) -> LlmResult<HealthStatus> {
        let start = Instant::now();

        // Anthropic doesn't have a dedicated health check endpoint,
        // so we'll do a minimal request to verify the API key works
        let test_req = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 10,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: "ok".to_string(),
            }],
            system: None,
            temperature: None,
        };

        let url = "https://api.anthropic.com/v1/messages";
        debug!("Health check: {}", url);

        match self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&test_req)
            .send()
            .await
        {
            Ok(response) => {
                let elapsed = start.elapsed().as_millis() as u64;
                if response.status().is_success() {
                    Ok(HealthStatus {
                        healthy: true,
                        message: "Anthropic is healthy".to_string(),
                        response_time_ms: elapsed,
                    })
                } else {
                    Err(LlmError::HealthCheckFailed(format!(
                        "Anthropic returned status: {}",
                        response.status()
                    )))
                }
            }
            Err(e) => {
                error!("Health check failed: {}", e);
                Err(LlmError::HealthCheckFailed(format!(
                    "Failed to connect to Anthropic: {}",
                    e
                )))
            }
        }
    }

    fn name(&self) -> &str {
        "Anthropic"
    }

    fn provider_type(&self) -> LlmProviderType {
        LlmProviderType::Anthropic {
            api_key: self.api_key.clone(),
            model: self.model.clone(),
        }
    }
}
