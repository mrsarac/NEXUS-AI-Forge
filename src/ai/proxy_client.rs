//! NEXUS API Proxy Client
//!
//! Secure client for the NEXUS API proxy server.
//! API keys are stored on the server - never exposed to the client.

#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Default proxy server URL
const DEFAULT_PROXY_URL: &str = "https://api-nexus.mustafasarac.com";

/// Request for code generation
#[derive(Debug, Serialize)]
pub struct GenerateRequest {
    pub description: String,
    pub language: String,
}

/// Response from code generation
#[derive(Debug, Deserialize)]
pub struct GenerateResponse {
    pub success: bool,
    pub code: Option<String>,
    pub language: Option<String>,
    pub error: Option<String>,
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
}

/// Request for chat/ask
#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// Response from chat/ask
#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub success: bool,
    pub response: Option<String>,
    pub error: Option<String>,
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
}

/// Health check response
#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

/// NEXUS API Proxy Client
///
/// Communicates with the secure proxy server.
/// All AI requests go through the proxy, which holds the API keys.
pub struct ProxyClient {
    base_url: String,
    client: reqwest::Client,
}

impl ProxyClient {
    /// Create a new proxy client with the default URL
    pub fn new() -> Self {
        Self::with_url(DEFAULT_PROXY_URL)
    }

    /// Create a new proxy client with a custom URL
    pub fn with_url(url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent(format!("NEXUS-Forge/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Create client from environment variable or default
    pub fn from_env() -> Self {
        let url = std::env::var("NEXUS_PROXY_URL")
            .unwrap_or_else(|_| DEFAULT_PROXY_URL.to_string());
        Self::with_url(&url)
    }

    /// Check if the proxy server is healthy
    pub async fn health_check(&self) -> Result<HealthResponse> {
        let url = format!("{}/health", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to NEXUS proxy")?;

        if !response.status().is_success() {
            anyhow::bail!("Proxy health check failed: {}", response.status());
        }

        response
            .json::<HealthResponse>()
            .await
            .context("Failed to parse health response")
    }

    /// Generate code using the proxy
    pub async fn generate(&self, description: &str, language: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);

        let request = GenerateRequest {
            description: description.to_string(),
            language: language.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to connect to NEXUS proxy")?;

        let status = response.status();
        let body: GenerateResponse = response
            .json()
            .await
            .context("Failed to parse generation response")?;

        if !status.is_success() || !body.success {
            let error_msg = body.error.unwrap_or_else(|| "Unknown error".to_string());
            anyhow::bail!("Code generation failed: {}", error_msg);
        }

        body.code.ok_or_else(|| anyhow::anyhow!("No code in response"))
    }

    /// Send a chat/ask request
    pub async fn chat(&self, message: &str, context: Option<&str>) -> Result<String> {
        let url = format!("{}/api/chat", self.base_url);

        let request = ChatRequest {
            message: message.to_string(),
            context: context.map(|s| s.to_string()),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to connect to NEXUS proxy")?;

        let status = response.status();
        let body: ChatResponse = response
            .json()
            .await
            .context("Failed to parse chat response")?;

        if !status.is_success() || !body.success {
            let error_msg = body.error.unwrap_or_else(|| "Unknown error".to_string());
            anyhow::bail!("Chat request failed: {}", error_msg);
        }

        body.response.ok_or_else(|| anyhow::anyhow!("No response in body"))
    }
}

impl Default for ProxyClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_client_creation() {
        let client = ProxyClient::new();
        assert_eq!(client.base_url, DEFAULT_PROXY_URL);
    }

    #[test]
    fn test_custom_url() {
        let client = ProxyClient::with_url("https://custom.example.com/");
        assert_eq!(client.base_url, "https://custom.example.com");
    }
}
