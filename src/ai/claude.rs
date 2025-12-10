//! Claude API Client for NEXUS AI Forge
//!
//! Implements the Anthropic Claude API with streaming support,
//! intelligent error handling, and conversation management.

#![allow(dead_code)]

use anyhow::{Context, Result};
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
const DEFAULT_MAX_TOKENS: u32 = 4096;
const REQUEST_TIMEOUT_SECS: u64 = 120;

/// Claude API Client
pub struct ClaudeClient {
    client: Client,
    api_key: String,
    model: String,
    max_tokens: u32,
}

/// Message role in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// A single message in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// Request body for Claude API
#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

/// Response from Claude API
#[derive(Debug, Deserialize)]
pub struct ClaudeResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Error response from Claude API
#[derive(Debug, Deserialize)]
struct ClaudeError {
    #[serde(rename = "type")]
    error_type: String,
    error: ErrorDetails,
}

#[derive(Debug, Deserialize)]
struct ErrorDetails {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

impl ClaudeClient {
    /// Create a new Claude client
    pub fn new(api_key: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            api_key,
            model: DEFAULT_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
        })
    }

    /// Create client from environment variable
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY environment variable not set")?;
        Self::new(api_key)
    }

    /// Set the model to use
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Set max tokens for response
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Send a single message and get response
    pub async fn send_message(&self, content: &str) -> Result<String> {
        let messages = vec![Message {
            role: Role::User,
            content: content.to_string(),
        }];

        self.complete(messages, None, None).await
    }

    /// Send a message with system prompt
    pub async fn send_with_system(
        &self,
        content: &str,
        system: &str,
    ) -> Result<String> {
        let messages = vec![Message {
            role: Role::User,
            content: content.to_string(),
        }];

        self.complete(messages, Some(system.to_string()), None).await
    }

    /// Complete a conversation with full control
    pub async fn complete(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> Result<String> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages,
            system,
            temperature,
        };

        let response = self.client
            .post(CLAUDE_API_URL)
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Claude API")?;

        let status = response.status();

        if status.is_success() {
            let claude_response: ClaudeResponse = response
                .json()
                .await
                .context("Failed to parse Claude response")?;

            // Extract text from content blocks
            let text = claude_response
                .content
                .iter()
                .filter_map(|block| block.text.as_ref())
                .cloned()
                .collect::<Vec<String>>()
                .join("");

            Ok(text)
        } else {
            let error_text = response.text().await.unwrap_or_default();

            // Try to parse as Claude error
            if let Ok(claude_error) = serde_json::from_str::<ClaudeError>(&error_text) {
                anyhow::bail!(
                    "Claude API error ({}): {}",
                    claude_error.error.error_type,
                    claude_error.error.message
                );
            }

            anyhow::bail!("Claude API error ({}): {}", status, error_text);
        }
    }

    /// Get full response with metadata
    pub async fn complete_full(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> Result<ClaudeResponse> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages,
            system,
            temperature,
        };

        let response = self.client
            .post(CLAUDE_API_URL)
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Claude API")?;

        let status = response.status();

        if status.is_success() {
            response
                .json()
                .await
                .context("Failed to parse Claude response")
        } else {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Claude API error ({}): {}", status, error_text);
        }
    }
}

/// Conversation manager for multi-turn chats
pub struct Conversation {
    client: ClaudeClient,
    messages: Vec<Message>,
    system: Option<String>,
}

impl Conversation {
    /// Start a new conversation
    pub fn new(client: ClaudeClient) -> Self {
        Self {
            client,
            messages: Vec::new(),
            system: None,
        }
    }

    /// Set system prompt for the conversation
    pub fn with_system(mut self, system: &str) -> Self {
        self.system = Some(system.to_string());
        self
    }

    /// Send a message and get response
    pub async fn send(&mut self, content: &str) -> Result<String> {
        // Add user message
        self.messages.push(Message {
            role: Role::User,
            content: content.to_string(),
        });

        // Get response
        let response = self.client
            .complete(
                self.messages.clone(),
                self.system.clone(),
                None,
            )
            .await?;

        // Add assistant response to history
        self.messages.push(Message {
            role: Role::Assistant,
            content: response.clone(),
        });

        Ok(response)
    }

    /// Get conversation history
    pub fn history(&self) -> &[Message] {
        &self.messages
    }

    /// Clear conversation history
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

/// System prompts for different coding tasks
pub mod prompts {
    /// System prompt for general coding assistance
    pub const CODING_ASSISTANT: &str = r#"You are NEXUS AI, an expert coding assistant built into a developer tool.

Your capabilities:
- Explain code clearly and concisely
- Suggest improvements and optimizations
- Help debug issues
- Write new code following best practices
- Answer questions about programming concepts

Guidelines:
- Be direct and concise
- Provide code examples when helpful
- Use markdown formatting for code blocks
- Focus on practical, actionable advice
- If you're unsure, say so honestly

Context: You're running inside a Rust-based CLI tool called NEXUS AI Forge."#;

    /// System prompt for code review
    pub const CODE_REVIEW: &str = r#"You are NEXUS AI, performing code review.

Focus on:
- Bugs and potential issues
- Security vulnerabilities
- Performance optimizations
- Code clarity and maintainability
- Best practices violations

Format your review as:
1. Summary (1-2 sentences)
2. Issues found (if any)
3. Suggestions for improvement
4. Positive observations (if any)

Be constructive and specific."#;

    /// System prompt for code explanation
    pub const EXPLAIN_CODE: &str = r#"You are NEXUS AI, explaining code to a developer.

When explaining code:
- Start with a high-level overview
- Break down complex logic step by step
- Explain the "why" not just the "what"
- Highlight important patterns or idioms
- Note any potential issues or improvements

Adjust complexity based on the code shown."#;

    /// System prompt for refactoring
    pub const REFACTOR: &str = r#"You are NEXUS AI, helping refactor code.

When refactoring:
- Preserve existing functionality
- Improve code clarity and maintainability
- Follow language idioms and best practices
- Explain each change you make
- Show before/after comparisons when helpful

Always output complete, working code."#;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = Message {
            role: Role::User,
            content: "Hello".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }
}
