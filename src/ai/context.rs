//! Context management for AI operations
//!
//! Handles gathering relevant context from the codebase
//! and managing context windows efficiently.

#![allow(dead_code)]

use std::path::PathBuf;

/// Represents a piece of context
#[derive(Debug, Clone)]
pub struct ContextChunk {
    pub source: PathBuf,
    pub content: String,
    pub relevance: f32,
    pub token_count: usize,
}

/// Context manager
pub struct ContextManager {
    max_tokens: usize,
    chunks: Vec<ContextChunk>,
}

impl ContextManager {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            chunks: Vec::new(),
        }
    }

    /// Add a context chunk with relevance scoring
    pub fn add_chunk(&mut self, chunk: ContextChunk) {
        self.chunks.push(chunk);
        // Sort by relevance
        self.chunks.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());
    }

    /// Build context string within token budget
    pub fn build_context(&self) -> String {
        let mut result = String::new();
        let mut tokens_used = 0;

        for chunk in &self.chunks {
            if tokens_used + chunk.token_count > self.max_tokens {
                break;
            }
            result.push_str(&format!("\n// Source: {:?}\n", chunk.source));
            result.push_str(&chunk.content);
            result.push('\n');
            tokens_used += chunk.token_count;
        }

        result
    }

    /// Estimate tokens (rough approximation)
    pub fn estimate_tokens(text: &str) -> usize {
        text.len() / 4
    }
}
