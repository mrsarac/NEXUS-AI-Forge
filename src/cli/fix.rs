//! Fix command - AI-powered bug fixing
//!
//! Analyzes code errors and provides intelligent fixes.

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
    pub const FIX: &str = "󰁨";
    pub const AI_ICON: &str = "󰌤";
    pub const FILE: &str = "󰈙";
    pub const SUCCESS: &str = "󰄂";
    pub const ERROR: &str = "󰅚";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// System prompt for bug fixing
const FIX_PROMPT: &str = r#"You are NEXUS AI, an expert bug fixing assistant.

Your task is to analyze the provided code and error message, then provide a fix.

Guidelines:
- Identify the root cause of the bug
- Explain why the bug occurs
- Provide a minimal, targeted fix
- Don't change unrelated code
- Preserve the original code style
- Consider edge cases

Output Format:
1. **Root Cause**: Brief explanation of what's wrong
2. **Fix**: The corrected code with changes highlighted
3. **Explanation**: Why this fix works
4. **Prevention**: How to prevent similar bugs in the future

Use markdown code blocks with the appropriate language tag for code."#;

/// Determine which AI mode to use
fn determine_ai_mode() -> AiMode {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        AiMode::Claude
    } else {
        AiMode::Proxy
    }
}

pub async fn run(_config: Config, file: &str, error_msg: Option<&str>) -> Result<()> {
    print_header(file);

    let ai_mode = determine_ai_mode();
    let provider_name = match ai_mode {
        AiMode::Claude => "Claude",
        AiMode::Proxy => "NEXUS AI (Free)",
    };

    // Read the file
    let path = Path::new(file);
    if !path.exists() {
        print_error(&format!("File not found: {}", file));
        return Ok(());
    }

    let content = fs::read_to_string(path)?;
    let lang = Language::from_path(path);
    let lines = content.lines().count();

    print_file_info(file, lang, lines);

    // Build prompt
    let mut prompt = format!(
        "## Code to Fix\n\n**File:** `{}`\n**Language:** {}\n\n```{}\n{}\n```\n",
        file,
        lang,
        lang.to_string().to_lowercase(),
        content
    );

    if let Some(err) = error_msg {
        prompt.push_str(&format!(
            "\n## Error Message\n\n```\n{}\n```\n",
            err
        ));
    }

    prompt.push_str("\n## Task\n\nAnalyze the code and provide a fix for the bug.");

    // Send to AI
    print_thinking(provider_name);

    let response = match ai_mode {
        AiMode::Claude => {
            let client = ClaudeClient::from_env()?;
            let mut conversation = Conversation::new(client)
                .with_system(FIX_PROMPT);

            conversation.send(&prompt).await?
        }
        AiMode::Proxy => {
            let proxy = ProxyClient::from_env();
            let prompt_with_system = format!("{}\n\n{}", FIX_PROMPT, prompt);
            proxy.chat(&prompt_with_system, None).await?
        }
    };

    clear_line();
    print_response(&response);

    Ok(())
}

// ============================================
// UI Functions
// ============================================

fn print_header(file: &str) {
    println!();
    println!(
        "{}{}  {} AI Bug Fix{}",
        colors::PRIMARY, colors::BOLD, symbols::FIX, colors::RESET
    );
    println!(
        "{}  │ Analyzing: {}{}{}",
        colors::MUTED, colors::FG, file, colors::RESET
    );
    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );
    println!();
}

fn print_file_info(file: &str, lang: Language, lines: usize) {
    println!(
        "{}  {} {} ({}, {} lines){}",
        colors::MUTED, symbols::FILE, file, lang, lines, colors::RESET
    );
    println!();
}

fn print_thinking(provider: &str) {
    print!(
        "\r{}  {} {} is analyzing the bug {}{}",
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
        "{}{}  {} Fix Analysis{}",
        colors::SUCCESS, colors::BOLD, symbols::FIX, colors::RESET
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

fn print_error(message: &str) {
    println!(
        "\n{}  {} Error: {}{}",
        colors::ERROR, symbols::ERROR, message, colors::RESET
    );
}
