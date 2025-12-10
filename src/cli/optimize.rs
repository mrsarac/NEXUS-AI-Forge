//! Optimize command - AI-powered performance optimization suggestions
//!
//! Analyzes code for performance bottlenecks and suggests improvements.

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
    pub const PERF_HIGH: &str = "\x1b[38;2;239;83;80m";      // Red - Critical
    pub const PERF_MED: &str = "\x1b[38;2;255;167;38m";      // Orange - Medium
    pub const PERF_LOW: &str = "\x1b[38;2;102;187;106m";     // Green - Low
}

mod symbols {
    pub const OPTIMIZE: &str = "󰓅";
    pub const AI_ICON: &str = "󰌤";
    pub const FILE: &str = "󰈙";
    pub const SUCCESS: &str = "󰄂";
    pub const ERROR: &str = "󰅚";
    pub const ROCKET: &str = "🚀";
    pub const WARNING: &str = "⚠";
    pub const LIGHTNING: &str = "⚡";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// System prompt for optimization analysis
const OPTIMIZE_PROMPT: &str = r#"You are NEXUS AI, a performance optimization expert.

## Your Task
Analyze the provided code for performance bottlenecks and optimization opportunities.

## Analysis Categories

### 1. Time Complexity
- Identify O(n²) or worse algorithms
- Suggest more efficient alternatives
- Note unnecessary iterations

### 2. Memory Usage
- Identify memory leaks or excessive allocations
- Suggest memory-efficient patterns
- Note unnecessary cloning/copying

### 3. I/O Operations
- Database query optimization
- File/Network I/O patterns
- Caching opportunities

### 4. Language-Specific
- Use of slow library functions
- Better standard library alternatives
- Compiler optimization hints

## Output Format

### Summary
Brief overview of optimization opportunities found.

### Critical Issues 🔴
Issues that significantly impact performance (if any).

### Recommendations 🟡
Medium-priority optimization suggestions.

### Minor Improvements 🟢
Small tweaks for marginal gains.

### Optimized Code (if applicable)
Provide refactored code snippets for critical issues.

Be specific with line numbers and provide before/after comparisons."#;

/// Determine which AI mode to use
fn determine_ai_mode() -> AiMode {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        AiMode::Claude
    } else {
        AiMode::Proxy
    }
}

pub async fn run(_config: Config, file: &str, focus: Option<&str>) -> Result<()> {
    let path = Path::new(file);

    // Verify file exists
    if !path.exists() {
        print_error(&format!("File not found: {}", file));
        return Ok(());
    }

    // Read source file
    let content = fs::read_to_string(path)?;
    let lang = Language::from_path(path);
    let lines = content.lines().count();

    print_header(file);

    // Parse code to get symbols
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

    let ai_mode = determine_ai_mode();
    let provider_name = match ai_mode {
        AiMode::Claude => "Claude",
        AiMode::Proxy => "NEXUS AI (Free)",
    };

    // Build focus area context
    let focus_context = match focus {
        Some("time") | Some("speed") => "\n\nFocus primarily on TIME COMPLEXITY optimizations.",
        Some("memory") | Some("mem") => "\n\nFocus primarily on MEMORY USAGE optimizations.",
        Some("io") | Some("network") => "\n\nFocus primarily on I/O and NETWORK optimizations.",
        Some("all") | None => "",
        Some(other) => &format!("\n\nFocus on: {}", other),
    };

    // Prepare prompt
    let prompt = format!(
        "## Code to Optimize\n\n**File:** `{}`\n**Language:** {}\n**Lines:** {}\n\n### Symbols Found:\n{}\n\n```{}\n{}\n```{}",
        file,
        lang,
        lines,
        symbols_summary.join("\n"),
        lang.to_string().to_lowercase(),
        content,
        focus_context
    );

    // Send to AI
    print_thinking(provider_name);

    let response = match ai_mode {
        AiMode::Claude => {
            let client = ClaudeClient::from_env()?;
            let mut conversation = Conversation::new(client)
                .with_system(OPTIMIZE_PROMPT);

            conversation.send(&prompt).await?
        }
        AiMode::Proxy => {
            let proxy = ProxyClient::from_env();
            let prompt_with_system = format!("{}\n\n{}", OPTIMIZE_PROMPT, prompt);
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
        "{}{}  {} Performance Optimizer{}",
        colors::PRIMARY, colors::BOLD, symbols::OPTIMIZE, colors::RESET
    );
    println!(
        "{}  │ Target: {}{}",
        colors::MUTED, file, colors::RESET
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
        "\r{}  {} {} is analyzing performance {}{}",
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
        "{}{}  {} Optimization Analysis{}",
        colors::SUCCESS, colors::BOLD, symbols::LIGHTNING, colors::RESET
    );
    println!(
        "{}  ╭{}─{}",
        colors::MUTED, "─".repeat(60), colors::RESET
    );

    for line in response.lines() {
        // Colorize severity indicators
        let colored_line = if line.contains("🔴") || line.contains("Critical") {
            format!("{}{}", colors::PERF_HIGH, line)
        } else if line.contains("🟡") || line.contains("Recommendation") {
            format!("{}{}", colors::PERF_MED, line)
        } else if line.contains("🟢") || line.contains("Minor") {
            format!("{}{}", colors::PERF_LOW, line)
        } else {
            format!("{}", line)
        };

        println!("{}  │ {}{}", colors::MUTED, colored_line, colors::RESET);
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
