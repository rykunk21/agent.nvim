use super::manager::{HealthStatus, LlmError, LlmProvider, LlmProviderType, LlmRequest, LlmResponse, LlmResult, TokenUsage};
use async_trait::async_trait;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiChoice {
    index: usize,
    message: OpenAiMessage,
    finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

pub struct OpenAiProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub async fn new(api_key: String, model: String, base_url: Option<String>) -> LlmResult<Self> {
        let base_url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let client = reqwest::Client::new();

        let provider = OpenAiProvider {
            api_key,
            model,
            base_url,
            client,
        };

        // Verify connection
        provider.health_check().await?;

        Ok(provider)
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn request(&self, req: LlmRequest) -> LlmResult<LlmResponse> {
        let start = Instant::now();

        let mut messages = vec![];

        if let Some(system) = req.system_prompt {
            messages.push(OpenAiMessage {
                role: "system".to_string(),
                content: system,
            });
        }

        messages.push(OpenAiMessage {
            role: "user".to_string(),
            content: req.prompt,
        });

        let openai_req = OpenAiRequest {
            model: self.model.clone(),
            messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            stream: false,
        };

        let url = format!("{}/chat/completions", self.base_url);
        debug!("Sending request to OpenAI: {}", url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&openai_req)
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
                "OpenAI returned status {}: {}",
                status, body
            )));
        }

        let openai_resp: OpenAiResponse = response
            .json()
            .await
            .map_err(|e| LlmError::RequestError(format!("Failed to parse response: {}", e)))?;

        let elapsed = start.elapsed().as_millis() as u64;
        debug!("OpenAI request completed in {}ms", elapsed);

        let content = openai_resp
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(LlmResponse {
            content,
            stop_reason: openai_resp
                .choices
                .first()
                .map(|c| c.finish_reason.clone())
                .unwrap_or_else(|| "stop".to_string()),
            usage: openai_resp.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }

    async fn stream_request(
        &self,
        req: LlmRequest,
    ) -> LlmResult<Box<dyn futures::Stream<Item = LlmResult<String>> + Unpin + Send>> {
        let mut messages = vec![];

        if let Some(system) = req.system_prompt {
            messages.push(OpenAiMessage {
                role: "system".to_string(),
                content: system,
            });
        }

        messages.push(OpenAiMessage {
            role: "user".to_string(),
            content: req.prompt,
        });

        let openai_req = OpenAiRequest {
            model: self.model.clone(),
            messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            stream: true,
        };

        let url = format!("{}/chat/completions", self.base_url);
        debug!("Starting streaming request to OpenAI: {}", url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&openai_req)
            .send()
            .await
            .map_err(|e| LlmError::RequestError(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            return Err(LlmError::RequestError(format!(
                "OpenAI returned status: {}",
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

        let url = format!("{}/models", self.base_url);
        debug!("Health check: {}", url);

        match self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
        {
            Ok(response) => {
                let elapsed = start.elapsed().as_millis() as u64;
                if response.status().is_success() {
                    Ok(HealthStatus {
                        healthy: true,
                        message: "OpenAI is healthy".to_string(),
                        response_time_ms: elapsed,
                    })
                } else {
                    Err(LlmError::HealthCheckFailed(format!(
                        "OpenAI returned status: {}",
                        response.status()
                    )))
                }
            }
            Err(e) => {
                error!("Health check failed: {}", e);
                Err(LlmError::HealthCheckFailed(format!(
                    "Failed to connect to OpenAI: {}",
                    e
                )))
            }
        }
    }

    fn name(&self) -> &str {
        "OpenAI"
    }

    fn provider_type(&self) -> LlmProviderType {
        LlmProviderType::OpenAI {
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            base_url: Some(self.base_url.clone()),
        }
    }
}
