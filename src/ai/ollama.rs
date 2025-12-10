//! Ollama Client - Local AI model support
//!
//! Enables running AI models locally via Ollama.
//! No API key needed - runs completely offline.

#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Default Ollama server URL
const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

/// Default model for code-related tasks
const DEFAULT_MODEL: &str = "codellama";

/// Request for chat completion
#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<ModelOptions>,
}

/// Chat message
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Model options
#[derive(Debug, Serialize)]
pub struct ModelOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

/// Chat response
#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub model: String,
    pub message: Message,
    pub done: bool,
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub eval_count: Option<u32>,
}

/// Generate request (simple completion)
#[derive(Debug, Serialize)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

/// Generate response
#[derive(Debug, Deserialize)]
pub struct GenerateResponse {
    pub model: String,
    pub response: String,
    pub done: bool,
}

/// List models response
#[derive(Debug, Deserialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
}

/// Model information
#[derive(Debug, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub modified_at: String,
    pub size: u64,
}

/// Ollama Client for local AI inference
pub struct OllamaClient {
    base_url: String,
    model: String,
    client: reqwest::Client,
    system_prompt: Option<String>,
}

impl OllamaClient {
    /// Create a new Ollama client with default settings
    pub fn new() -> Self {
        Self::with_model(DEFAULT_MODEL)
    }

    /// Create a new client with a specific model
    pub fn with_model(model: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 min for local inference
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: DEFAULT_OLLAMA_URL.to_string(),
            model: model.to_string(),
            client,
            system_prompt: None,
        }
    }

    /// Create client from environment or defaults
    pub fn from_env() -> Self {
        let url = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| DEFAULT_OLLAMA_URL.to_string());
        let model = std::env::var("OLLAMA_MODEL")
            .unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: url,
            model,
            client,
            system_prompt: None,
        }
    }

    /// Set the base URL
    pub fn with_url(mut self, url: &str) -> Self {
        self.base_url = url.trim_end_matches('/').to_string();
        self
    }

    /// Set a system prompt
    pub fn with_system(mut self, prompt: &str) -> Self {
        self.system_prompt = Some(prompt.to_string());
        self
    }

    /// Check if Ollama is running
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        self.client.get(&url).send().await.is_ok()
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/api/tags", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to Ollama. Is it running?")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama request failed: {}", response.status());
        }

        let models: ModelsResponse = response
            .json()
            .await
            .context("Failed to parse models response")?;

        Ok(models.models)
    }

    /// Send a chat message
    pub async fn chat(&self, message: &str) -> Result<String> {
        self.chat_with_history(message, Vec::new()).await
    }

    /// Send a chat message with conversation history
    pub async fn chat_with_history(&self, message: &str, history: Vec<Message>) -> Result<String> {
        let url = format!("{}/api/chat", self.base_url);

        let mut messages = Vec::new();

        // Add system prompt if set
        if let Some(ref system) = self.system_prompt {
            messages.push(Message {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        // Add history
        messages.extend(history);

        // Add current message
        messages.push(Message {
            role: "user".to_string(),
            content: message.to_string(),
        });

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: Some(false),
            options: Some(ModelOptions {
                temperature: Some(0.7),
                num_predict: Some(4096),
                top_p: Some(0.9),
            }),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to connect to Ollama. Is it running?")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama request failed ({}): {}", status, body);
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse chat response")?;

        Ok(chat_response.message.content)
    }

    /// Simple text generation (non-chat)
    pub async fn generate(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);

        let request = GenerateRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: Some(false),
            system: self.system_prompt.clone(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to connect to Ollama")?;

        if !response.status().is_success() {
            anyhow::bail!("Generation failed: {}", response.status());
        }

        let gen_response: GenerateResponse = response
            .json()
            .await
            .context("Failed to parse generate response")?;

        Ok(gen_response.response)
    }

    /// Get the current model name
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Set a different model
    pub fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Recommended models for different tasks
pub struct RecommendedModels;

impl RecommendedModels {
    /// Best for code generation and completion
    pub const CODE: &'static str = "codellama";

    /// Best for code with more context
    pub const CODE_INSTRUCT: &'static str = "codellama:instruct";

    /// General purpose, fast
    pub const LLAMA2: &'static str = "llama2";

    /// Smaller, faster model
    pub const MISTRAL: &'static str = "mistral";

    /// Very capable, larger
    pub const MIXTRAL: &'static str = "mixtral";

    /// Best for complex reasoning
    pub const DEEPSEEK_CODER: &'static str = "deepseek-coder";

    /// List of models good for coding tasks
    pub fn coding_models() -> Vec<&'static str> {
        vec![
            Self::CODE,
            Self::CODE_INSTRUCT,
            Self::DEEPSEEK_CODER,
            Self::MISTRAL,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = OllamaClient::new();
        assert_eq!(client.model, DEFAULT_MODEL);
        assert_eq!(client.base_url, DEFAULT_OLLAMA_URL);
    }

    #[test]
    fn test_with_model() {
        let client = OllamaClient::with_model("mistral");
        assert_eq!(client.model, "mistral");
    }

    #[test]
    fn test_with_system() {
        let client = OllamaClient::new().with_system("You are helpful.");
        assert_eq!(client.system_prompt, Some("You are helpful.".to_string()));
    }
}
