//! Test command - AI-powered test generation
//!
//! Generates unit tests for code using AI.

#![allow(dead_code)]

use anyhow::Result;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::ai::{ClaudeClient, Conversation, ProxyClient};
use crate::config::Config;
use crate::core::parser::{CodeParser, Language};

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
    pub const TEST: &str = "󰙨";
    pub const AI_ICON: &str = "󰌤";
    pub const FILE: &str = "󰈙";
    pub const SUCCESS: &str = "󰄂";
    pub const ERROR: &str = "󰅚";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// System prompt for test generation
const TEST_PROMPT: &str = r#"You are NEXUS AI, an expert test generation assistant.

Your task is to generate comprehensive unit tests for the provided code.

Guidelines:
- Follow the testing conventions of the language
- Test all public functions and methods
- Include edge cases and error cases
- Use meaningful test names that describe what's being tested
- Add comments explaining what each test verifies
- Use appropriate mocking where needed
- Aim for high code coverage

For Rust: Use #[cfg(test)] and #[test] attributes
For Python: Use pytest or unittest conventions
For JavaScript/TypeScript: Use Jest/Vitest conventions

Output Format:
1. Brief analysis of what needs testing
2. Complete test code in a markdown code block
3. Explanation of test coverage

Use markdown code blocks with the appropriate language tag."#;

/// Determine which AI mode to use
fn determine_ai_mode() -> AiMode {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        AiMode::Claude
    } else {
        AiMode::Proxy
    }
}

pub async fn run(_config: Config, file: &str, output: Option<&str>) -> Result<()> {
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

    // Parse to get symbols
    let mut parser = CodeParser::new()?;
    let parsed = parser.parse_file(path)?;
    let symbol_count = parsed.symbols.len();

    print_file_info(file, lang, lines, symbol_count);

    // Build symbol list for context
    let symbol_list: Vec<String> = parsed.symbols
        .iter()
        .map(|s| format!("- {} ({})", s.name, format!("{:?}", s.kind).to_lowercase()))
        .collect();

    let prompt = format!(
        "## Code to Test\n\n**File:** `{}`\n**Language:** {}\n\n### Symbols found:\n{}\n\n```{}\n{}\n```\n\n## Task\n\nGenerate comprehensive unit tests for this code.",
        file,
        lang,
        symbol_list.join("\n"),
        lang.to_string().to_lowercase(),
        content
    );

    // Send to AI
    print_thinking(provider_name);

    let response = match ai_mode {
        AiMode::Claude => {
            let client = ClaudeClient::from_env()?;
            let mut conversation = Conversation::new(client)
                .with_system(TEST_PROMPT);

            conversation.send(&prompt).await?
        }
        AiMode::Proxy => {
            let proxy = ProxyClient::from_env();
            let prompt_with_system = format!("{}\n\n{}", TEST_PROMPT, prompt);
            proxy.chat(&prompt_with_system, None).await?
        }
    };

    clear_line();

    // Extract code from response if output file specified
    if let Some(out_path) = output {
        if let Some(code) = extract_code_block(&response, lang) {
            fs::write(out_path, &code)?;
            print_saved(out_path, &code);
        } else {
            print_response(&response);
            print_warning("Could not extract test code. Showing full response.");
        }
    } else {
        print_response(&response);
    }

    Ok(())
}

/// Extract code block from markdown response
fn extract_code_block(response: &str, lang: Language) -> Option<String> {
    let lang_str = lang.to_string().to_lowercase();
    let patterns = vec![
        format!("```{}", lang_str),
        "```rust".to_string(),
        "```python".to_string(),
        "```javascript".to_string(),
        "```typescript".to_string(),
        "```".to_string(),
    ];

    for pattern in patterns {
        if let Some(start_idx) = response.find(&pattern) {
            let code_start = start_idx + pattern.len();
            if let Some(end_idx) = response[code_start..].find("```") {
                let code = response[code_start..code_start + end_idx].trim();
                if !code.is_empty() {
                    return Some(code.to_string());
                }
            }
        }
    }

    None
}

// ============================================
// UI Functions
// ============================================

fn print_header(file: &str) {
    println!();
    println!(
        "{}{}  {} AI Test Generator{}",
        colors::PRIMARY, colors::BOLD, symbols::TEST, colors::RESET
    );
    println!(
        "{}  │ Source: {}{}{}",
        colors::MUTED, colors::FG, file, colors::RESET
    );
    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );
    println!();
}

fn print_file_info(file: &str, lang: Language, lines: usize, symbols: usize) {
    println!(
        "{}  {} {} ({}, {} lines, {} symbols){}",
        colors::MUTED, symbols::FILE, file, lang, lines, symbols, colors::RESET
    );
    println!();
}

fn print_thinking(provider: &str) {
    print!(
        "\r{}  {} {} is generating tests {}{}",
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
        "{}{}  {} Generated Tests{}",
        colors::SUCCESS, colors::BOLD, symbols::TEST, colors::RESET
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

fn print_saved(path: &str, code: &str) {
    let lines = code.lines().count();
    println!();
    println!(
        "{}{}  {} Tests saved to {}{}",
        colors::SUCCESS, colors::BOLD, symbols::SUCCESS, path, colors::RESET
    );
    println!(
        "{}  {} lines of test code generated{}",
        colors::MUTED, lines, colors::RESET
    );
    println!();
}

fn print_error(message: &str) {
    println!(
        "\n{}  {} Error: {}{}",
        colors::ERROR, symbols::ERROR, message, colors::RESET
    );
}

fn print_warning(message: &str) {
    println!(
        "{}  {} {}{}",
        colors::WARNING, symbols::TEST, message, colors::RESET
    );
}
