//! Doc command - AI-powered documentation generation
//!
//! Generates documentation for code files or entire projects.

#![allow(dead_code)]

use anyhow::Result;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::ai::{ClaudeClient, Conversation, ProxyClient};
use crate::config::Config;
use crate::core::parser::{CodeParser, Language, SymbolKind};

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
    pub const DOC: &str = "󰈙";
    pub const AI_ICON: &str = "󰌤";
    pub const FILE: &str = "󰈙";
    pub const SUCCESS: &str = "󰄂";
    pub const ERROR: &str = "󰅚";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// System prompt for documentation generation
const DOC_PROMPT: &str = r#"You are NEXUS AI, an expert technical documentation writer.

Your task is to generate comprehensive documentation for the provided code.

## Guidelines
- Write clear, professional documentation
- Include examples where helpful
- Document public interfaces thoroughly
- Follow language-specific documentation conventions

## For Rust
- Use /// for doc comments
- Include Examples section with runnable code
- Document panic conditions and errors

## For Python
- Use docstrings with Google or NumPy style
- Include Args, Returns, Raises sections
- Add type hints in documentation

## For JavaScript/TypeScript
- Use JSDoc format
- Include @param, @returns, @throws
- Add @example sections

## Output
Generate documentation comments that can be added directly to the code.
Format as markdown with appropriate code blocks."#;

/// Determine which AI mode to use
fn determine_ai_mode() -> AiMode {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        AiMode::Claude
    } else {
        AiMode::Proxy
    }
}

pub async fn run(_config: Config, file: &str, output: Option<&str>, inline: bool) -> Result<()> {
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

    // Build symbol summary
    let symbols_summary: Vec<String> = parsed.symbols
        .iter()
        .map(|s| {
            let kind = match s.kind {
                SymbolKind::Function => "function",
                SymbolKind::Struct => "struct",
                SymbolKind::Class => "class",
                SymbolKind::Enum => "enum",
                SymbolKind::Trait => "trait",
                SymbolKind::Interface => "interface",
                SymbolKind::Module => "module",
                SymbolKind::Constant => "constant",
                SymbolKind::Impl => "impl",
                SymbolKind::TypeAlias => "type",
            };
            format!("- `{}` ({}) at line {}", s.name, kind, s.line_start)
        })
        .collect();

    print_file_info(file, lang, lines, symbols_summary.len());

    let doc_style = if inline {
        "Generate inline documentation comments to add directly to the code."
    } else {
        "Generate a comprehensive documentation file (like README or API docs)."
    };

    let prompt = format!(
        "## Code to Document\n\n**File:** `{}`\n**Language:** {}\n\n### Symbols:\n{}\n\n```{}\n{}\n```\n\n## Task\n\n{}",
        file,
        lang,
        symbols_summary.join("\n"),
        lang.to_string().to_lowercase(),
        content,
        doc_style
    );

    // Send to AI
    print_thinking(provider_name);

    let response = match ai_mode {
        AiMode::Claude => {
            let client = ClaudeClient::from_env()?;
            let mut conversation = Conversation::new(client)
                .with_system(DOC_PROMPT);

            conversation.send(&prompt).await?
        }
        AiMode::Proxy => {
            let proxy = ProxyClient::from_env();
            let prompt_with_system = format!("{}\n\n{}", DOC_PROMPT, prompt);
            proxy.chat(&prompt_with_system, None).await?
        }
    };

    clear_line();

    // Save to file if output specified
    if let Some(out_path) = output {
        fs::write(out_path, &response)?;
        print_saved(out_path);
    } else {
        print_response(&response);
    }

    Ok(())
}

// ============================================
// UI Functions
// ============================================

fn print_header(file: &str) {
    println!();
    println!(
        "{}{}  {} AI Documentation Generator{}",
        colors::PRIMARY, colors::BOLD, symbols::DOC, colors::RESET
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
        "\r{}  {} {} is generating documentation {}{}",
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
        "{}{}  {} Generated Documentation{}",
        colors::SUCCESS, colors::BOLD, symbols::DOC, colors::RESET
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

fn print_saved(path: &str) {
    println!();
    println!(
        "{}{}  {} Documentation saved to {}{}",
        colors::SUCCESS, colors::BOLD, symbols::SUCCESS, path, colors::RESET
    );
    println!();
}

fn print_error(message: &str) {
    println!(
        "\n{}  {} Error: {}{}",
        colors::ERROR, symbols::ERROR, message, colors::RESET
    );
}
