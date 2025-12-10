//! Info command - show system information

use anyhow::Result;

pub fn run() -> Result<()> {
    println!("NEXUS AI Forge v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("System Information:");
    println!("  OS: {} {}", std::env::consts::OS, std::env::consts::ARCH);
    println!("  Rust: {}", rustc_version());

    println!();
    println!("Configuration:");
    println!("  Config dir: {}", config_dir());

    println!();
    println!("AI Providers:");
    check_provider("ANTHROPIC_API_KEY", "Claude");
    check_provider("OPENAI_API_KEY", "OpenAI");
    check_provider("GEMINI_API_KEY", "Gemini");

    Ok(())
}

fn rustc_version() -> &'static str {
    // Compile-time Rust version would require build script
    "1.75+"
}

fn config_dir() -> String {
    directories::ProjectDirs::from("com", "nexus", "forge")
        .map(|p| p.config_dir().to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn check_provider(env_var: &str, name: &str) {
    let status = if std::env::var(env_var).is_ok() {
        "configured"
    } else {
        "not configured"
    };
    println!("  {}: {}", name, status);
}
