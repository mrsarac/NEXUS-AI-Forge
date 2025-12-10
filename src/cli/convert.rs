//! Convert command - Convert code between programming languages
//!
//! Uses AI to intelligently convert code from one language to another.

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
    pub const CONVERT: &str = "󰁕";
    pub const AI_ICON: &str = "󰌤";
    pub const FILE: &str = "󰈙";
    pub const SUCCESS: &str = "󰄂";
    pub const ERROR: &str = "󰅚";
    pub const ARROW: &str = "→";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// System prompt for code conversion
const CONVERT_PROMPT: &str = r#"You are NEXUS AI, an expert polyglot programmer specializing in code translation.

## Your Task
Convert code from one programming language to another while preserving:
- Functionality and logic
- Code style and best practices of the target language
- Comments and documentation (translated appropriately)
- Error handling patterns

## Guidelines
1. Use idiomatic patterns of the target language
2. Adapt data structures to native equivalents
3. Handle language-specific features gracefully
4. Preserve the original code's intent
5. Add comments for non-obvious translations

## Output Format
Return ONLY the converted code in a code block.
Do not include explanations unless there are important caveats.

If something cannot be directly translated, add a TODO comment explaining the limitation."#;

/// Determine which AI mode to use
fn determine_ai_mode() -> AiMode {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        AiMode::Claude
    } else {
        AiMode::Proxy
    }
}

/// Detect language from file extension or explicit parameter
fn detect_language(file: &str, explicit: Option<&str>) -> String {
    if let Some(lang) = explicit {
        return lang.to_lowercase();
    }

    let path = Path::new(file);
    Language::from_path(path).to_string().to_lowercase()
}

/// Get file extension for language
fn get_extension_for_language(lang: &str) -> &str {
    match lang.to_lowercase().as_str() {
        "rust" => "rs",
        "python" => "py",
        "javascript" => "js",
        "typescript" => "ts",
        "go" | "golang" => "go",
        "java" => "java",
        "csharp" | "c#" => "cs",
        "ruby" => "rb",
        "swift" => "swift",
        "kotlin" => "kt",
        "cpp" | "c++" => "cpp",
        "c" => "c",
        "php" => "php",
        "scala" => "scala",
        _ => "txt"
    }
}

/// Extract code from markdown code blocks
fn extract_code_from_response(response: &str) -> String {
    let lines: Vec<&str> = response.lines().collect();
    let mut in_code_block = false;
    let mut code_lines = Vec::new();

    for line in lines {
        if line.starts_with("```") {
            if in_code_block {
                break; // End of first code block
            } else {
                in_code_block = true;
                continue;
            }
        }

        if in_code_block {
            code_lines.push(line);
        }
    }

    if code_lines.is_empty() {
        // No code block found, return the whole response
        response.to_string()
    } else {
        code_lines.join("\n")
    }
}

pub async fn run(
    _config: Config,
    file: &str,
    target_lang: &str,
    output: Option<&str>,
) -> Result<()> {
    let path = Path::new(file);

    // Verify file exists
    if !path.exists() {
        print_error(&format!("File not found: {}", file));
        return Ok(());
    }

    // Read source file
    let source_code = fs::read_to_string(path)?;
    let source_lang = detect_language(file, None);
    let target = target_lang.to_lowercase();

    print_header(file, &source_lang, &target);
    print_file_info(file, source_code.lines().count());

    let ai_mode = determine_ai_mode();
    let provider_name = match ai_mode {
        AiMode::Claude => "Claude",
        AiMode::Proxy => "NEXUS AI (Free)",
    };

    // Prepare prompt
    let prompt = format!(
        "## Source Code ({source_lang})\n\n```{source_lang}\n{source_code}\n```\n\n## Target Language\nConvert this code to {target}.\n\nFollow {target} best practices and idioms.",
        source_lang = source_lang,
        source_code = source_code,
        target = target
    );

    // Send to AI
    print_thinking(provider_name, &source_lang, &target);

    let response = match ai_mode {
        AiMode::Claude => {
            let client = ClaudeClient::from_env()?;
            let mut conversation = Conversation::new(client)
                .with_system(CONVERT_PROMPT);

            conversation.send(&prompt).await?
        }
        AiMode::Proxy => {
            let proxy = ProxyClient::from_env();
            let prompt_with_system = format!("{}\n\n{}", CONVERT_PROMPT, prompt);
            proxy.chat(&prompt_with_system, None).await?
        }
    };

    clear_line();

    // Extract code from response
    let converted_code = extract_code_from_response(&response);

    // Save or print
    if let Some(out_path) = output {
        fs::write(out_path, &converted_code)?;
        print_saved(out_path);
    } else {
        // Generate default output filename
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = get_extension_for_language(&target);
        let default_output = format!("{}_converted.{}", stem, ext);
        fs::write(&default_output, &converted_code)?;
        print_saved(&default_output);
    }

    Ok(())
}

// ============================================
// UI Functions
// ============================================

fn print_header(file: &str, source: &str, target: &str) {
    println!();
    println!(
        "{}{}  {} Code Converter{}",
        colors::PRIMARY, colors::BOLD, symbols::CONVERT, colors::RESET
    );
    println!(
        "{}  │ {} {} {} {}{}",
        colors::MUTED, source, symbols::ARROW, target, colors::RESET, ""
    );
    println!(
        "{}  │ Source: {}{}",
        colors::MUTED, file, colors::RESET
    );
    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );
    println!();
}

fn print_file_info(file: &str, lines: usize) {
    println!(
        "{}  {} {} ({} lines){}",
        colors::MUTED, symbols::FILE, file, lines, colors::RESET
    );
    println!();
}

fn print_thinking(provider: &str, source: &str, target: &str) {
    print!(
        "\r{}  {} {} is converting {} {} {} {}{}",
        colors::WARNING,
        symbols::AI_ICON,
        provider,
        source,
        symbols::ARROW,
        target,
        symbols::SPINNER[0],
        colors::RESET
    );
    io::stdout().flush().ok();
}

fn clear_line() {
    print!("\r{}\r", " ".repeat(80));
    io::stdout().flush().ok();
}

fn print_saved(path: &str) {
    println!();
    println!(
        "{}{}  {} Converted code saved to {}{}",
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
