//! Explain command - explain code files or snippets with AI
//!
//! Analyzes code using tree-sitter and provides detailed explanations.

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::path::Path;
use std::fs;
use std::io::{self, Write};

use crate::ai::{ClaudeClient, Conversation};
use crate::config::Config;
use crate::core::parser::{CodeParser, Language, SymbolKind};

// ANSI color codes from design system
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const PRIMARY: &str = "\x1b[38;2;100;181;246m";      // #64B5F6
    pub const SUCCESS: &str = "\x1b[38;2;165;214;167m";      // #A5D6A7
    pub const ERROR: &str = "\x1b[38;2;239;154;154m";        // #EF9A9A
    pub const AI_ACCENT: &str = "\x1b[38;2;255;202;40m";     // #FFCA28
    pub const MUTED: &str = "\x1b[38;2;84;110;122m";         // #546E7A
    pub const FG: &str = "\x1b[38;2;212;212;215m";           // #D4D4D7
}

mod symbols {
    pub const AI_ICON: &str = "󰌤";
    pub const FILE: &str = "󰈙";
    pub const ERROR: &str = "󰅚";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// System prompts for different explanation depths
fn get_system_prompt(depth: &str) -> &'static str {
    match depth {
        "brief" => r#"You are NEXUS AI, explaining code concisely.

Guidelines:
- Give a 2-3 sentence overview of what the code does
- Highlight the most important function/purpose
- Keep it short and to the point
- No code examples needed"#,

        "expert" => r#"You are NEXUS AI, providing expert-level code analysis.

Guidelines:
- Analyze architecture and design patterns used
- Discuss performance characteristics and trade-offs
- Identify potential improvements or issues
- Reference industry best practices
- Explain complex algorithms in detail
- Discuss edge cases and error handling
- Use technical terminology appropriate for senior developers"#,

        _ => r#"You are NEXUS AI, explaining code in detail.

Guidelines:
- Start with a high-level overview (what problem does this solve?)
- Break down the main components/functions
- Explain the flow of data and control
- Highlight important design decisions
- Note any patterns or idioms used
- Use markdown formatting for clarity
- Include brief code references when helpful"#,
    }
}

pub async fn run(_config: Config, target: &str, depth: &str) -> Result<()> {
    let path = Path::new(target);

    // Check if target exists
    if !path.exists() {
        print_error(&format!("File not found: {}", target));
        return Ok(());
    }

    // Print header
    print_header(target, depth);

    // Try to create Claude client
    let client = match ClaudeClient::from_env() {
        Ok(c) => c,
        Err(e) => {
            print_error(&format!("Could not initialize AI: {}", e));
            println!(
                "\n{}  To use explain, set your Anthropic API key:{}",
                colors::MUTED, colors::RESET
            );
            println!(
                "{}  export ANTHROPIC_API_KEY=\"your-api-key\"{}",
                colors::FG, colors::RESET
            );
            return Ok(());
        }
    };

    // Read file content
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", target))?;

    // Parse the file to get structure info
    let mut parser = CodeParser::new()
        .context("Failed to initialize parser")?;

    let language = Language::from_path(path);
    let structure_info = if language != Language::Unknown {
        if let Ok(parsed) = parser.parse_file(path) {
            let counts = parsed.symbol_counts();
            let mut info = format!(
                "Language: {}\nLines: {}\nSymbols: {} functions, {} types, {} enums\n\n",
                language.name(), parsed.line_count,
                counts.functions, counts.types, counts.enums
            );

            // Add symbol list
            if !parsed.symbols.is_empty() {
                info.push_str("Key symbols:\n");
                for symbol in parsed.symbols.iter().take(15) {
                    let kind = match symbol.kind {
                        SymbolKind::Function => "fn",
                        SymbolKind::Struct => "struct",
                        SymbolKind::Class => "class",
                        SymbolKind::Enum => "enum",
                        SymbolKind::Trait => "trait",
                        SymbolKind::Interface => "interface",
                        SymbolKind::Module => "mod",
                        SymbolKind::Constant => "const",
                        SymbolKind::Impl => "impl",
                        SymbolKind::TypeAlias => "type",
                    };
                    info.push_str(&format!("- {} {} (line {})\n", kind, symbol.name, symbol.line_start));
                }
            }
            info
        } else {
            format!("Language: {}\n", language.name())
        }
    } else {
        "Language: Unknown\n".to_string()
    };

    // Build prompt
    let prompt = format!(
        "## File: {}\n\n## Structure\n{}\n## Code\n```\n{}\n```\n\nPlease explain this code.",
        target, structure_info, content
    );

    // Send to Claude
    print_thinking();

    let mut conversation = Conversation::new(client)
        .with_system(get_system_prompt(depth));

    match conversation.send(&prompt).await {
        Ok(response) => {
            clear_line();
            print_response(&response, depth);
        }
        Err(e) => {
            clear_line();
            print_error(&format!("AI error: {}", e));
        }
    }

    Ok(())
}

/// Print the header
fn print_header(target: &str, depth: &str) {
    let depth_label = match depth {
        "brief" => "Brief Overview",
        "expert" => "Expert Analysis",
        _ => "Detailed Explanation",
    };

    println!();
    println!(
        "{}{}  {} Explaining: {}{}",
        colors::PRIMARY, colors::BOLD, symbols::FILE, target, colors::RESET
    );
    println!(
        "{}  │ Mode: {}{}{}",
        colors::MUTED, colors::FG, depth_label, colors::RESET
    );
    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );
    println!();
}

/// Print thinking indicator
fn print_thinking() {
    print!(
        "\r{}  {} Analyzing code {}{}",
        colors::AI_ACCENT,
        symbols::AI_ICON,
        symbols::SPINNER[0],
        colors::RESET
    );
    io::stdout().flush().ok();
}

/// Clear the current line
fn clear_line() {
    print!("\r{}\r", " ".repeat(60));
    io::stdout().flush().ok();
}

/// Print the AI response
fn print_response(response: &str, depth: &str) {
    let title = match depth {
        "brief" => "Brief Overview",
        "expert" => "Expert Analysis",
        _ => "Explanation",
    };

    println!();
    println!(
        "{}{}  {} {} {}",
        colors::AI_ACCENT, colors::BOLD, symbols::AI_ICON, title, colors::RESET
    );
    println!(
        "{}  ╭{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );

    for line in response.lines() {
        println!("{}  │ {}{}", colors::MUTED, colors::FG, line);
    }

    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );
    println!();
}

/// Print error message
fn print_error(message: &str) {
    println!(
        "\n{}  {} Error: {}{}",
        colors::ERROR, symbols::ERROR, message, colors::RESET
    );
}
