//! Refactor command - AI-powered code refactoring
//!
//! Reads code files and uses AI to suggest or apply refactorings.

#![allow(dead_code)]

use anyhow::Result;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::ai::{ClaudeClient, Conversation, ProxyClient};
use crate::config::Config;
use crate::core::parser::Language;

/// AI Provider mode
#[derive(Debug, Clone, Copy, PartialEq)]
enum AiMode {
    Claude,
    Proxy,
}

// ANSI color codes
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const PRIMARY: &str = "\x1b[38;2;100;181;246m";      // #64B5F6
    pub const SUCCESS: &str = "\x1b[38;2;165;214;167m";      // #A5D6A7
    pub const ERROR: &str = "\x1b[38;2;239;154;154m";        // #EF9A9A
    pub const WARNING: &str = "\x1b[38;2;255;202;40m";       // #FFCA28
    pub const MUTED: &str = "\x1b[38;2;84;110;122m";         // #546E7A
    pub const FG: &str = "\x1b[38;2;212;212;215m";           // #D4D4D7
}

mod symbols {
    pub const REFACTOR: &str = "󰑕";
    pub const AI_ICON: &str = "󰌤";
    pub const FILE: &str = "󰈙";
    pub const SUCCESS: &str = "󰄂";
    pub const ERROR: &str = "󰅚";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// System prompt for refactoring
const REFACTOR_PROMPT: &str = r#"You are NEXUS AI, an expert code refactoring assistant.

Your task is to refactor the provided code according to the user's description.

Guidelines:
- Preserve the original functionality exactly
- Improve code quality, readability, and maintainability
- Follow language-specific best practices and idioms
- Add or improve comments where helpful
- Keep the same public API unless explicitly asked to change it
- Explain each significant change you make

Output Format:
1. First, briefly explain the refactoring changes you're making
2. Then provide the complete refactored code
3. Use markdown code blocks with the appropriate language tag

Be thorough but focused - only make changes that improve the code according to the description."#;

/// Determine which AI mode to use
fn determine_ai_mode() -> AiMode {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        AiMode::Claude
    } else {
        AiMode::Proxy
    }
}

pub async fn run(_config: Config, paths: &[String], description: &str) -> Result<()> {
    print_header(description);

    let ai_mode = determine_ai_mode();
    let provider_name = match ai_mode {
        AiMode::Claude => "Claude",
        AiMode::Proxy => "NEXUS AI (Free)",
    };

    // Collect all files to refactor
    let mut files_content: Vec<(String, String, Language)> = Vec::new();

    for path_str in paths {
        let path = Path::new(path_str);

        if path.is_file() {
            if let Some(content) = read_file_if_supported(path) {
                let lang = Language::from_path(path);
                files_content.push((path_str.clone(), content, lang));
            }
        } else if path.is_dir() {
            // Walk directory and collect supported files
            for entry in walkdir::WalkDir::new(path)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| {
                    let name = e.file_name().to_string_lossy();
                    !name.starts_with('.') &&
                    name != "node_modules" &&
                    name != "target" &&
                    name != "build" &&
                    name != "dist"
                })
            {
                if let Ok(entry) = entry {
                    let file_path = entry.path();
                    if file_path.is_file() {
                        if let Some(content) = read_file_if_supported(file_path) {
                            let lang = Language::from_path(file_path);
                            files_content.push((
                                file_path.display().to_string(),
                                content,
                                lang
                            ));
                        }
                    }
                }
            }
        }
    }

    if files_content.is_empty() {
        print_error("No supported files found in the specified paths");
        return Ok(());
    }

    // Show files to be refactored
    print_files_summary(&files_content);

    // Build the prompt with all file contents
    let mut code_context = String::new();
    for (path, content, lang) in &files_content {
        let lang_str = lang.to_string().to_lowercase();
        code_context.push_str(&format!(
            "\n### File: `{}`\n```{}\n{}\n```\n",
            path, lang_str, content
        ));
    }

    let full_prompt = format!(
        "## Refactoring Request\n\n{}\n\n## Code to Refactor\n{}",
        description, code_context
    );

    // Send to AI
    print_thinking(provider_name);

    let response = match ai_mode {
        AiMode::Claude => {
            let client = ClaudeClient::from_env()?;
            let mut conversation = Conversation::new(client)
                .with_system(REFACTOR_PROMPT);

            conversation.send(&full_prompt).await?
        }
        AiMode::Proxy => {
            let proxy = ProxyClient::from_env();
            let prompt_with_system = format!("{}\n\n{}", REFACTOR_PROMPT, full_prompt);
            proxy.chat(&prompt_with_system, None).await?
        }
    };

    clear_line();
    print_response(&response);

    // Ask if user wants to apply changes
    print_apply_hint();

    Ok(())
}

/// Read file if it's a supported language
fn read_file_if_supported(path: &Path) -> Option<String> {
    let lang = Language::from_path(path);
    if lang == Language::Unknown {
        return None;
    }

    fs::read_to_string(path).ok()
}

// ============================================
// UI Functions
// ============================================

fn print_header(description: &str) {
    println!();
    println!(
        "{}{}  {} Code Refactoring{}",
        colors::PRIMARY, colors::BOLD, symbols::REFACTOR, colors::RESET
    );
    println!(
        "{}  │ {}{}{}",
        colors::MUTED, colors::FG, description, colors::RESET
    );
    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );
    println!();
}

fn print_files_summary(files: &[(String, String, Language)]) {
    println!(
        "{}  {} Files to refactor ({}):{}",
        colors::MUTED, symbols::FILE, files.len(), colors::RESET
    );

    for (path, content, lang) in files.iter().take(10) {
        let lines = content.lines().count();
        println!(
            "{}     {} {} ({}, {} lines){}",
            colors::MUTED, "•", path, lang, lines, colors::RESET
        );
    }

    if files.len() > 10 {
        println!(
            "{}     ... and {} more files{}",
            colors::MUTED, files.len() - 10, colors::RESET
        );
    }
    println!();
}

fn print_thinking(provider: &str) {
    print!(
        "\r{}  {} {} is analyzing and refactoring {}{}",
        colors::WARNING,
        symbols::AI_ICON,
        provider,
        symbols::SPINNER[0],
        colors::RESET
    );
    io::stdout().flush().ok();
}

fn clear_line() {
    print!("\r{}\r", " ".repeat(70));
    io::stdout().flush().ok();
}

fn print_response(response: &str) {
    println!();
    println!(
        "{}{}  {} Refactoring Suggestions{}",
        colors::SUCCESS, colors::BOLD, symbols::REFACTOR, colors::RESET
    );
    println!(
        "{}  ╭{}─{}",
        colors::MUTED, "─".repeat(60), colors::RESET
    );

    for line in response.lines() {
        println!("{}  │ {}{}", colors::MUTED, colors::FG, line);
    }

    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(60), colors::RESET
    );
    println!();
}

fn print_apply_hint() {
    println!(
        "{}  💡 To apply changes: Copy the refactored code and replace the original files.{}",
        colors::MUTED, colors::RESET
    );
    println!(
        "{}     Future versions will support automatic application with --apply flag.{}",
        colors::MUTED, colors::RESET
    );
    println!();
}

fn print_error(message: &str) {
    println!(
        "\n{}  {} Error: {}{}",
        colors::ERROR, symbols::ERROR, message, colors::RESET
    );
}
