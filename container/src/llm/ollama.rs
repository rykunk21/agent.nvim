use super::manager::{HealthStatus, LlmError, LlmProvider, LlmProviderType, LlmRequest, LlmResponse, LlmResult, TokenUsage};
use async_trait::async_trait;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaResponse {
    model: String,
    created_at: String,
    response: String,
    done: bool,
    #[serde(default)]
    context: Vec<i32>,
    #[serde(default)]
    total_duration: u64,
    #[serde(default)]
    load_duration: u64,
    #[serde(default)]
    prompt_eval_count: usize,
    #[serde(default)]
    prompt_eval_duration: u64,
    #[serde(default)]
    eval_count: usize,
    #[serde(default)]
    eval_duration: u64,
}

pub struct OllamaProvider {
    endpoint: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub async fn new(endpoint: String, model: String) -> LlmResult<Self> {
        let client = reqwest::Client::new();
        let provider = OllamaProvider {
            endpoint,
            model,
            client,
        };

        // Verify connection
        provider.health_check().await?;

        Ok(provider)
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn request(&self, req: LlmRequest) -> LlmResult<LlmResponse> {
        let start = Instant::now();

        let ollama_req = OllamaRequest {
            model: self.model.clone(),
            prompt: req.prompt,
            stream: false,
            system: req.system_prompt,
            temperature: req.temperature,
        };

        let url = format!("{}/api/generate", self.endpoint);
        debug!("Sending request to Ollama: {}", url);

        let response = self
            .client
            .post(&url)
            .json(&ollama_req)
            .send()
            .await
            .map_err(|e| LlmError::RequestError(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            return Err(LlmError::RequestError(format!(
                "Ollama returned status: {}",
                response.status()
            )));
        }

        let ollama_resp: OllamaResponse = response
            .json()
            .await
            .map_err(|e| LlmError::RequestError(format!("Failed to parse response: {}", e)))?;

        let elapsed = start.elapsed().as_millis() as u64;
        debug!("Ollama request completed in {}ms", elapsed);

        Ok(LlmResponse {
            content: ollama_resp.response,
            stop_reason: "stop".to_string(),
            usage: Some(TokenUsage {
                prompt_tokens: ollama_resp.prompt_eval_count,
                completion_tokens: ollama_resp.eval_count,
                total_tokens: ollama_resp.prompt_eval_count + ollama_resp.eval_count,
            }),
        })
    }

    async fn stream_request(
        &self,
        req: LlmRequest,
    ) -> LlmResult<Box<dyn futures::Stream<Item = LlmResult<String>> + Unpin + Send>> {
        let ollama_req = OllamaRequest {
            model: self.model.clone(),
            prompt: req.prompt,
            stream: true,
            system: req.system_prompt,
            temperature: req.temperature,
        };

        let url = format!("{}/api/generate", self.endpoint);
        debug!("Starting streaming request to Ollama: {}", url);

        let response = self
            .client
            .post(&url)
            .json(&ollama_req)
            .send()
            .await
            .map_err(|e| LlmError::RequestError(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            return Err(LlmError::RequestError(format!(
                "Ollama returned status: {}",
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

        let url = format!("{}/api/tags", self.endpoint);
        debug!("Health check: {}", url);

        match self.client.get(&url).send().await {
            Ok(response) => {
                let elapsed = start.elapsed().as_millis() as u64;
                if response.status().is_success() {
                    Ok(HealthStatus {
                        healthy: true,
                        message: "Ollama is healthy".to_string(),
                        response_time_ms: elapsed,
                    })
                } else {
                    Err(LlmError::HealthCheckFailed(format!(
                        "Ollama returned status: {}",
                        response.status()
                    )))
                }
            }
            Err(e) => {
                error!("Health check failed: {}", e);
                Err(LlmError::HealthCheckFailed(format!(
                    "Failed to connect to Ollama: {}",
                    e
                )))
            }
        }
    }

    fn name(&self) -> &str {
        "Ollama"
    }

    fn provider_type(&self) -> LlmProviderType {
        LlmProviderType::Ollama {
            endpoint: self.endpoint.clone(),
            model: self.model.clone(),
        }
    }
}
