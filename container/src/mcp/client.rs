use super::protocol::{McpAuthConfig, McpServiceConfig, McpServiceInfo, McpTool};
use anyhow::Result;
use log::{debug, error, info};
use std::time::Duration;

pub struct McpClient {
    config: McpServiceConfig,
    client: reqwest::Client,
    retry_count: u32,
    max_retries: u32,
}

impl McpClient {
    pub async fn new(config: McpServiceConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let mcp_client = McpClient {
            config,
            client,
            retry_count: 0,
            max_retries: 3,
        };

        // Verify connection
        mcp_client.health_check().await?;

        Ok(mcp_client)
    }

    /// Discover available tools from the MCP service
    pub async fn discover_tools(&self) -> Result<Vec<McpTool>> {
        debug!("Discovering tools from MCP service: {}", self.config.name);

        let url = format!("{}/tools", self.config.endpoint);
        let response = self.make_request("GET", &url, None).await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to discover tools: {}",
                response.status()
            ));
        }

        let service_info: McpServiceInfo = response.json().await?;
        info!(
            "Discovered {} tools from {}",
            service_info.tools.len(),
            self.config.name
        );

        Ok(service_info.tools)
    }

    /// Call a tool on the MCP service
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value> {
        debug!(
            "Calling tool {} on MCP service: {}",
            tool_name, self.config.name
        );

        let url = format!("{}/tools/{}/call", self.config.endpoint, tool_name);
        let body = serde_json::json!({ "arguments": arguments });

        let response = self
            .make_request("POST", &url, Some(body))
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Tool call failed: {}",
                response.status()
            ));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result)
    }

    /// Health check for the MCP service
    pub async fn health_check(&self) -> Result<()> {
        debug!("Health check for MCP service: {}", self.config.name);

        let url = format!("{}/health", self.config.endpoint);

        match self.make_request("GET", &url, None).await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("MCP service {} is healthy", self.config.name);
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "MCP service health check failed: {}",
                        response.status()
                    ))
                }
            }
            Err(e) => {
                error!("MCP service {} health check error: {}", self.config.name, e);
                Err(e)
            }
        }
    }

    /// Make an HTTP request with retry logic
    async fn make_request(
        &self,
        method: &str,
        url: &str,
        body: Option<serde_json::Value>,
    ) -> Result<reqwest::Response> {
        let mut attempt = 0;

        loop {
            attempt += 1;

            let request = match method {
                "GET" => self.client.get(url),
                "POST" => self.client.post(url),
                "PUT" => self.client.put(url),
                "DELETE" => self.client.delete(url),
                _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", method)),
            };

            let request = self.add_auth_headers(request);

            let request = if let Some(body) = &body {
                request.json(body)
            } else {
                request
            };

            match request.send().await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    if attempt >= self.max_retries {
                        error!(
                            "Request failed after {} attempts: {}",
                            self.max_retries, e
                        );
                        return Err(anyhow::anyhow!("Request failed: {}", e));
                    }

                    // Exponential backoff
                    let backoff = Duration::from_millis(100 * 2_u64.pow(attempt - 1));
                    debug!(
                        "Request failed, retrying in {:?} (attempt {}/{})",
                        backoff, attempt, self.max_retries
                    );
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }

    /// Add authentication headers to the request
    fn add_auth_headers(
        &self,
        request: reqwest::RequestBuilder,
    ) -> reqwest::RequestBuilder {
        match &self.config.auth {
            Some(McpAuthConfig::ApiKey { key }) => {
                request.header("X-API-Key", key)
            }
            Some(McpAuthConfig::Bearer { token }) => {
                request.header("Authorization", format!("Bearer {}", token))
            }
            Some(McpAuthConfig::Basic { username, password }) => {
                let credentials = format!("{}:{}", username, password);
                let encoded = base64::encode(&credentials);
                request.header("Authorization", format!("Basic {}", encoded))
            }
            Some(McpAuthConfig::OAuth2 {
                client_id,
                client_secret,
            }) => {
                // In a real implementation, you'd handle OAuth2 token exchange
                let credentials = format!("{}:{}", client_id, client_secret);
                let encoded = base64::encode(&credentials);
                request.header("Authorization", format!("Basic {}", encoded))
            }
            None => request,
        }
    }
}

// Helper function for base64 encoding (since we don't have base64 crate yet)
mod base64 {
    pub fn encode(input: &str) -> String {
        use std::fmt::Write;

        const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let bytes = input.as_bytes();
        let mut result = String::new();

        for chunk in bytes.chunks(3) {
            let b1 = chunk[0];
            let b2 = chunk.get(1).copied().unwrap_or(0);
            let b3 = chunk.get(2).copied().unwrap_or(0);

            let n = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);

            let c1 = TABLE[((n >> 18) & 63) as usize] as char;
            let c2 = TABLE[((n >> 12) & 63) as usize] as char;
            let c3 = if chunk.len() > 1 {
                TABLE[((n >> 6) & 63) as usize] as char
            } else {
                '='
            };
            let c4 = if chunk.len() > 2 {
                TABLE[(n & 63) as usize] as char
            } else {
                '='
            };

            let _ = write!(result, "{}{}{}{}", c1, c2, c3, c4);
        }

        result
    }
}
