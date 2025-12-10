//! AI provider implementations

#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Common response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct AiResponse {
    pub content: String,
    pub model: String,
    pub tokens_used: u32,
    pub finish_reason: String,
}

/// Claude API client
pub struct ClaudeClient {
    api_key: String,
    model: String,
}

impl ClaudeClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }

    pub async fn complete(&self, _prompt: &str) -> Result<AiResponse> {
        // TODO: Implement Claude API
        todo!("Claude API implementation")
    }
}

/// OpenAI API client
pub struct OpenAiClient {
    api_key: String,
    model: String,
}

impl OpenAiClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }

    pub async fn complete(&self, _prompt: &str) -> Result<AiResponse> {
        // TODO: Implement OpenAI API
        todo!("OpenAI API implementation")
    }
}

/// Gemini API client
pub struct GeminiClient {
    api_key: String,
    model: String,
}

impl GeminiClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }

    pub async fn complete(&self, _prompt: &str) -> Result<AiResponse> {
        // TODO: Implement Gemini API
        todo!("Gemini API implementation")
    }
}

/// Local model client (Ollama/llama.cpp)
pub struct LocalClient {
    endpoint: String,
    model: String,
}

impl LocalClient {
    pub fn new(endpoint: String, model: String) -> Self {
        Self { endpoint, model }
    }

    pub async fn complete(&self, _prompt: &str) -> Result<AiResponse> {
        // TODO: Implement local model API
        todo!("Local model implementation")
    }
}
