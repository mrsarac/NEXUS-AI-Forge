//! AI provider integrations and routing

pub mod claude;
pub mod context;
pub mod ollama;
pub mod providers;
pub mod proxy_client;
pub mod router;

pub use claude::{ClaudeClient, Conversation};
#[allow(unused_imports)]
pub use ollama::OllamaClient;
pub use proxy_client::ProxyClient;
