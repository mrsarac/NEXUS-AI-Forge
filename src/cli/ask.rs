//! Ask command - query your codebase with AI assistance
//!
//! Indexes the codebase, finds relevant context, and uses Claude to answer questions.

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::path::Path;
use std::io::{self, Write};

use crate::ai::{ClaudeClient, Conversation, ProxyClient};
use crate::config::Config;
use crate::core::parser::{CodeParser, Language, ParsedFile, Symbol, SymbolKind};

/// AI Provider mode
#[derive(Debug, Clone, Copy, PartialEq)]
enum AiMode {
    Claude,
    Proxy,
}

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
    pub const SEARCH: &str = "󰍉";
    pub const SUCCESS: &str = "󰄂";
    pub const ERROR: &str = "󰅚";
    pub const CODE: &str = "";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// System prompt for codebase questions
const CODEBASE_ASSISTANT: &str = r#"You are NEXUS AI, an expert coding assistant with deep knowledge of the user's codebase.

You have been given context about the codebase including:
- File structure and symbols (functions, structs, enums, etc.)
- Relevant code snippets

Guidelines:
- Answer questions based on the provided context
- Be specific and reference actual code when possible
- If you're not sure, say so honestly
- Use markdown formatting for code examples
- Keep responses concise but complete

When explaining code:
- Start with a high-level overview
- Reference specific functions/structs by name
- Explain the "why" not just the "what"
"#;

/// Determine which AI mode to use
fn determine_ai_mode() -> AiMode {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        AiMode::Claude
    } else {
        AiMode::Proxy
    }
}

pub async fn run(_config: Config, question: &str) -> Result<()> {
    // Print header
    print_header(question);

    // Determine AI mode
    let ai_mode = determine_ai_mode();
    let provider_name = match ai_mode {
        AiMode::Claude => "Claude",
        AiMode::Proxy => "NEXUS AI (Free)",
    };

    // Index codebase
    print_status("Scanning codebase...");
    let parsed_files = index_codebase(Path::new("."))?;

    if parsed_files.is_empty() {
        print_warning("No supported files found in current directory");
        return Ok(());
    }

    // Find relevant context based on question
    print_status("Finding relevant context...");
    let context = build_context(&parsed_files, question);

    // Build prompt with context
    let full_prompt = format!(
        "{}\n\n## Codebase Context\n\n{}\n\n## Question\n\n{}",
        CODEBASE_ASSISTANT, context, question
    );

    // Send to AI
    print_thinking_with_provider(provider_name);

    match ai_mode {
        AiMode::Claude => {
            let client = ClaudeClient::from_env()?;
            let mut conversation = Conversation::new(client)
                .with_system(CODEBASE_ASSISTANT);

            let prompt = format!(
                "## Codebase Context\n\n{}\n\n## Question\n\n{}",
                context, question
            );

            match conversation.send(&prompt).await {
                Ok(response) => {
                    clear_line();
                    print_response(&response);
                }
                Err(e) => {
                    clear_line();
                    print_error(&format!("AI error: {}", e));
                }
            }
        }
        AiMode::Proxy => {
            let proxy = ProxyClient::from_env();

            match proxy.chat(&full_prompt, None).await {
                Ok(response) => {
                    clear_line();
                    print_response(&response);
                }
                Err(e) => {
                    clear_line();
                    print_error(&format!("AI error: {}", e));
                }
            }
        }
    }

    Ok(())
}

/// Index all supported files in the codebase
fn index_codebase(path: &Path) -> Result<Vec<ParsedFile>> {
    let mut parser = CodeParser::new()
        .context("Failed to initialize code parser")?;

    let mut parsed_files = Vec::new();

    // Walk directory
    for entry in walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Skip hidden and common non-source dirs
            !name.starts_with('.') &&
            name != "node_modules" &&
            name != "target" &&
            name != "build" &&
            name != "dist" &&
            name != "__pycache__" &&
            name != "vendor"
        })
    {
        let entry = entry?;
        let file_path = entry.path();

        if file_path.is_file() {
            let language = Language::from_path(file_path);
            if language != Language::Unknown {
                if let Ok(parsed) = parser.parse_file(file_path) {
                    parsed_files.push(parsed);
                }
            }
        }
    }

    Ok(parsed_files)
}

/// Build context string from parsed files based on the question
fn build_context(files: &[ParsedFile], question: &str) -> String {
    let question_lower = question.to_lowercase();
    let mut context_parts = Vec::new();

    // Extract keywords from question
    let keywords: Vec<&str> = question_lower
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .collect();

    // File summary
    context_parts.push(format!(
        "### Codebase Overview\n- {} files indexed\n- Languages: Rust, Python, JavaScript, TypeScript\n",
        files.len()
    ));

    // Find relevant symbols
    let mut relevant_symbols: Vec<(&ParsedFile, &Symbol)> = Vec::new();

    for file in files {
        for symbol in &file.symbols {
            let symbol_lower = symbol.name.to_lowercase();

            // Check if symbol name matches any keyword
            let is_relevant = keywords.iter().any(|kw| {
                symbol_lower.contains(kw) || kw.contains(&symbol_lower)
            });

            if is_relevant {
                relevant_symbols.push((file, symbol));
            }
        }
    }

    // Add relevant symbols to context
    if !relevant_symbols.is_empty() {
        context_parts.push("### Relevant Symbols\n".to_string());

        for (file, symbol) in relevant_symbols.iter().take(10) {
            let rel_path = file.path.strip_prefix(".").unwrap_or(&file.path);
            let kind_str = match symbol.kind {
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

            context_parts.push(format!(
                "- `{}` ({}) in `{}` (lines {}-{})",
                symbol.name, kind_str, rel_path.display(),
                symbol.line_start, symbol.line_end
            ));

            // Add signature if available
            if let Some(sig) = &symbol.signature {
                context_parts.push(format!("  ```\n  {}\n  ```", sig));
            }
        }
    }

    // Add file structure summary
    context_parts.push("\n### File Structure\n".to_string());

    // Group by directory
    let mut dirs: std::collections::HashMap<String, Vec<&ParsedFile>> = std::collections::HashMap::new();
    for file in files {
        let dir = file.path.parent()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| ".".to_string());
        dirs.entry(dir).or_default().push(file);
    }

    for (dir, dir_files) in dirs.iter().take(5) {
        context_parts.push(format!("- `{}/`", dir));
        for file in dir_files.iter().take(3) {
            let filename = file.path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let counts = file.symbol_counts();
            context_parts.push(format!(
                "  - `{}` ({} functions, {} types)",
                filename, counts.functions, counts.types
            ));
        }
        if dir_files.len() > 3 {
            context_parts.push(format!("  - ... and {} more", dir_files.len() - 3));
        }
    }

    context_parts.join("\n")
}

/// Print the header
fn print_header(question: &str) {
    println!();
    println!(
        "{}{}  {} Asking about your codebase...{}",
        colors::PRIMARY, colors::BOLD, symbols::SEARCH, colors::RESET
    );
    println!(
        "{}  │ {}{}{}",
        colors::MUTED, colors::FG, question, colors::RESET
    );
    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );
    println!();
}

/// Print a status message
fn print_status(message: &str) {
    println!(
        "{}  {} {}{}",
        colors::MUTED, symbols::SPINNER[0], message, colors::RESET
    );
}

/// Print thinking indicator
fn print_thinking() {
    print!(
        "\r{}  {} Nexus AI is thinking {}{}",
        colors::AI_ACCENT,
        symbols::AI_ICON,
        symbols::SPINNER[0],
        colors::RESET
    );
    io::stdout().flush().ok();
}

/// Print thinking indicator with provider name
fn print_thinking_with_provider(provider: &str) {
    print!(
        "\r{}  {} {} is thinking {}{}",
        colors::AI_ACCENT,
        symbols::AI_ICON,
        provider,
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
fn print_response(response: &str) {
    println!();
    println!(
        "{}{}  {} Nexus AI {}",
        colors::AI_ACCENT, colors::BOLD, symbols::AI_ICON, colors::RESET
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

/// Print warning message
fn print_warning(message: &str) {
    println!(
        "\n{}  {} {}{}",
        colors::AI_ACCENT, symbols::ERROR, message, colors::RESET
    );
}
