//! Configuration management for NEXUS AI Forge

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub ai: AiConfig,
    pub privacy: PrivacyConfig,
    pub index: IndexConfig,
    #[serde(skip)]
    pub verbose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub theme: String,
    pub telemetry: bool,
    pub auto_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub default_provider: String,
    pub local_fallback: bool,
    pub providers: AiProviders,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProviders {
    pub claude: Option<ProviderConfig>,
    pub openai: Option<ProviderConfig>,
    pub gemini: Option<ProviderConfig>,
    pub local: Option<LocalProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key_env: String,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalProviderConfig {
    pub enabled: bool,
    pub backend: String,
    pub model: String,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub send_code_to_cloud: bool,
    pub local_embeddings: bool,
    pub anonymize_telemetry: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    pub auto_index: bool,
    pub exclude_patterns: Vec<String>,
    pub max_file_size_mb: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                theme: "dark".to_string(),
                telemetry: false,
                auto_update: true,
            },
            ai: AiConfig {
                default_provider: "claude".to_string(),
                local_fallback: true,
                providers: AiProviders {
                    claude: Some(ProviderConfig {
                        api_key_env: "ANTHROPIC_API_KEY".to_string(),
                        model: "claude-3-opus-20240229".to_string(),
                        max_tokens: Some(4096),
                        temperature: Some(0.7),
                    }),
                    openai: Some(ProviderConfig {
                        api_key_env: "OPENAI_API_KEY".to_string(),
                        model: "gpt-4o".to_string(),
                        max_tokens: Some(4096),
                        temperature: Some(0.7),
                    }),
                    gemini: Some(ProviderConfig {
                        api_key_env: "GEMINI_API_KEY".to_string(),
                        model: "gemini-pro".to_string(),
                        max_tokens: Some(8192),
                        temperature: Some(0.7),
                    }),
                    local: Some(LocalProviderConfig {
                        enabled: true,
                        backend: "ollama".to_string(),
                        model: "codellama".to_string(),
                        endpoint: Some("http://localhost:11434".to_string()),
                    }),
                },
            },
            privacy: PrivacyConfig {
                send_code_to_cloud: false,
                local_embeddings: true,
                anonymize_telemetry: true,
            },
            index: IndexConfig {
                auto_index: true,
                exclude_patterns: vec![
                    "node_modules".to_string(),
                    ".git".to_string(),
                    "target".to_string(),
                    "__pycache__".to_string(),
                    "*.lock".to_string(),
                ],
                max_file_size_mb: 10,
            },
            verbose: false,
        }
    }
}

/// Get the configuration file path
fn config_path() -> Result<PathBuf> {
    let config_dir = directories::ProjectDirs::from("com", "nexus", "forge")
        .context("Failed to determine config directory")?
        .config_dir()
        .to_path_buf();

    Ok(config_dir.join("config.toml"))
}

/// Load configuration from file or use defaults
pub fn load_config(custom_path: Option<&str>) -> Result<Config> {
    let path = if let Some(p) = custom_path {
        PathBuf::from(p)
    } else {
        config_path()?
    };

    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config from {:?}", path))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config from {:?}", path))?;
        Ok(config)
    } else {
        Ok(Config::default())
    }
}

/// Initialize configuration file with defaults
pub fn init_config() -> Result<()> {
    let path = config_path()?;

    if path.exists() {
        println!("Configuration file already exists at {:?}", path);
        return Ok(());
    }

    // Create directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory {:?}", parent))?;
    }

    // Write default config
    let default_config = Config::default();
    let content = toml::to_string_pretty(&default_config)
        .context("Failed to serialize default config")?;

    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write config to {:?}", path))?;

    println!("Configuration initialized at {:?}", path);
    Ok(())
}

/// Show current configuration
pub fn show_config(config: &Config) -> Result<()> {
    let content = toml::to_string_pretty(config)
        .context("Failed to serialize config")?;
    println!("{}", content);
    Ok(())
}
