//! Generate command - create code from natural language descriptions
//!
//! Uses AI to generate code based on plain English descriptions.
//! Supports automatic language detection and file output.

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::path::Path;
use std::fs;
use std::io::{self, Write};

use crate::ai::{ClaudeClient, Conversation, ProxyClient};
use crate::config::Config;
use crate::ui::{FormOption, NexusForm, FormResult};

/// AI Provider mode
#[derive(Debug, Clone, Copy, PartialEq)]
enum AiMode {
    /// Use local Claude API key (power users)
    Claude,
    /// Use NEXUS proxy (free tier, no API key needed)
    Proxy,
}

// ANSI color codes from design system
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
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
    pub const CODE: &str = "";
    pub const ERROR: &str = "󰅚";
    pub const SUCCESS: &str = "󰄂";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// Supported programming languages
#[derive(Debug, Clone, Copy)]
enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    CSharp,
    Ruby,
    Swift,
    Kotlin,
    Unknown,
}

impl Language {
    fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Language::Rust,
            "py" => Language::Python,
            "js" | "jsx" => Language::JavaScript,
            "ts" | "tsx" => Language::TypeScript,
            "go" => Language::Go,
            "java" => Language::Java,
            "cs" => Language::CSharp,
            "rb" => Language::Ruby,
            "swift" => Language::Swift,
            "kt" | "kts" => Language::Kotlin,
            _ => Language::Unknown,
        }
    }

    fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "rust" | "rs" => Language::Rust,
            "python" | "py" => Language::Python,
            "javascript" | "js" => Language::JavaScript,
            "typescript" | "ts" => Language::TypeScript,
            "go" | "golang" => Language::Go,
            "java" => Language::Java,
            "csharp" | "c#" | "cs" => Language::CSharp,
            "ruby" | "rb" => Language::Ruby,
            "swift" => Language::Swift,
            "kotlin" | "kt" => Language::Kotlin,
            _ => Language::Unknown,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Go => "Go",
            Language::Java => "Java",
            Language::CSharp => "C#",
            Language::Ruby => "Ruby",
            Language::Swift => "Swift",
            Language::Kotlin => "Kotlin",
            Language::Unknown => "Unknown",
        }
    }

    fn extension(&self) -> &'static str {
        match self {
            Language::Rust => "rs",
            Language::Python => "py",
            Language::JavaScript => "js",
            Language::TypeScript => "ts",
            Language::Go => "go",
            Language::Java => "java",
            Language::CSharp => "cs",
            Language::Ruby => "rb",
            Language::Swift => "swift",
            Language::Kotlin => "kt",
            Language::Unknown => "txt",
        }
    }

    fn code_fence(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
            Language::Go => "go",
            Language::Java => "java",
            Language::CSharp => "csharp",
            Language::Ruby => "ruby",
            Language::Swift => "swift",
            Language::Kotlin => "kotlin",
            Language::Unknown => "",
        }
    }
}

/// Get system prompt for code generation
fn get_system_prompt(language: Language) -> String {
    format!(r#"You are NEXUS AI, an expert code generator.

Your task is to generate clean, idiomatic, production-ready {} code based on the user's description.

Guidelines:
- Write complete, working code (not pseudocode)
- Follow {} best practices and conventions
- Include necessary imports/dependencies
- Add brief, helpful comments where appropriate
- Handle errors appropriately
- Make the code modular and testable
- Use descriptive variable and function names

Output Format:
- Return ONLY the code, no explanations before or after
- Do not wrap the code in markdown code blocks
- Start directly with the code (imports, etc.)
- End with the last line of code

The user will save this directly to a file, so it must be valid, compilable/runnable code."#,
        language.name(), language.name()
    )
}

pub async fn run(
    _config: Config,
    description: &str,
    output: Option<&str>,
    language: Option<&str>,
) -> Result<()> {
    // Determine language
    let lang = determine_language(output, language)?;

    // Print header
    print_header(description, lang, output);

    // Determine AI mode: Claude if API key exists, otherwise use free proxy
    let ai_mode = determine_ai_mode();

    match ai_mode {
        AiMode::Claude => {
            run_with_claude(description, lang, output).await
        }
        AiMode::Proxy => {
            run_with_proxy(description, lang, output).await
        }
    }
}

/// Determine which AI mode to use
fn determine_ai_mode() -> AiMode {
    // Check for Claude API key
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        return AiMode::Claude;
    }

    // Default to free proxy (Gemini-powered)
    AiMode::Proxy
}

/// Run code generation with Claude (requires API key)
async fn run_with_claude(description: &str, lang: Language, output: Option<&str>) -> Result<()> {
    let client = ClaudeClient::from_env()?;

    let prompt = format!(
        "Generate {} code for the following:\n\n{}",
        lang.name(), description
    );

    print_thinking_with_provider(lang, "Claude");

    let mut conversation = Conversation::new(client)
        .with_system(&get_system_prompt(lang));

    match conversation.send(&prompt).await {
        Ok(response) => {
            clear_line();
            let code = clean_code_response(&response);
            handle_output(output, &code, lang, description);
        }
        Err(e) => {
            clear_line();
            print_error(&format!("Claude error: {}", e));
        }
    }

    Ok(())
}

/// Run code generation with NEXUS Proxy (free tier, Gemini-powered)
async fn run_with_proxy(description: &str, lang: Language, output: Option<&str>) -> Result<()> {
    let proxy = ProxyClient::from_env();

    print_thinking_with_provider(lang, "NEXUS AI (Free)");

    match proxy.generate(description, lang.code_fence()).await {
        Ok(code) => {
            clear_line();
            let code = clean_code_response(&code);
            handle_output(output, &code, lang, description);
        }
        Err(e) => {
            clear_line();
            print_error(&format!("Generation failed: {}", e));
            print_proxy_help();
        }
    }

    Ok(())
}

/// Handle the generated code output
fn handle_output(output: Option<&str>, code: &str, lang: Language, description: &str) {
    if let Some(output_path) = output {
        if let Err(e) = write_to_file(output_path, code) {
            print_error(&format!("Failed to write file: {}", e));
            return;
        }
        print_file_created(output_path, code);
    } else {
        let suggested_name = suggest_filename(description, lang);
        print_code_preview(code, lang);
        print_save_suggestion(&suggested_name);
    }
}

/// Print help for proxy connection issues
fn print_proxy_help() {
    println!();
    println!(
        "{}  Troubleshooting:{}",
        colors::MUTED, colors::RESET
    );
    println!(
        "{}  • Check your internet connection{}",
        colors::FG, colors::RESET
    );
    println!(
        "{}  • NEXUS proxy may be temporarily unavailable{}",
        colors::FG, colors::RESET
    );
    println!(
        "{}  • For unlimited access, set ANTHROPIC_API_KEY{}",
        colors::FG, colors::RESET
    );
    println!();
}

/// Determine the target language
fn determine_language(output: Option<&str>, language: Option<&str>) -> Result<Language> {
    // Priority: explicit language flag > file extension > ask user
    if let Some(lang_name) = language {
        let lang = Language::from_name(lang_name);
        if matches!(lang, Language::Unknown) {
            anyhow::bail!("Unknown language: {}. Supported: rust, python, javascript, typescript, go, java, csharp, ruby, swift, kotlin", lang_name);
        }
        return Ok(lang);
    }

    if let Some(output_path) = output {
        if let Some(ext) = Path::new(output_path).extension() {
            let lang = Language::from_extension(&ext.to_string_lossy());
            if !matches!(lang, Language::Unknown) {
                return Ok(lang);
            }
        }
    }

    // Ask user interactively
    let options = vec![
        FormOption::new("Rust", "Systems programming, CLI tools, performance").recommended(),
        FormOption::new("Python", "Scripting, data science, web backends"),
        FormOption::new("TypeScript", "Type-safe JavaScript, web development"),
        FormOption::new("JavaScript", "Web development, Node.js"),
        FormOption::new("Go", "Cloud services, DevOps tools"),
    ];

    let form = NexusForm::new();
    match form.select("Which programming language?", &options)? {
        FormResult::Single(idx) => {
            Ok(match idx {
                0 => Language::Rust,
                1 => Language::Python,
                2 => Language::TypeScript,
                3 => Language::JavaScript,
                4 => Language::Go,
                _ => Language::Rust,
            })
        }
        _ => Ok(Language::Rust), // Default
    }
}

/// Clean up AI response (remove markdown code blocks if present)
fn clean_code_response(response: &str) -> String {
    let trimmed = response.trim();

    // Check if wrapped in code blocks
    if trimmed.starts_with("```") {
        // Find the end of the first line (language specifier)
        if let Some(first_newline) = trimmed.find('\n') {
            let rest = &trimmed[first_newline + 1..];
            // Find closing ```
            if let Some(end_pos) = rest.rfind("```") {
                return rest[..end_pos].trim().to_string();
            }
        }
    }

    trimmed.to_string()
}

/// Suggest a filename based on description
fn suggest_filename(description: &str, lang: Language) -> String {
    // Extract a simple name from description
    let words: Vec<&str> = description
        .split_whitespace()
        .take(3)
        .collect();

    let base_name = if words.is_empty() {
        "generated".to_string()
    } else {
        words.join("_").to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    };

    format!("{}.{}", base_name, lang.extension())
}

/// Write code to file
fn write_to_file(path: &str, code: &str) -> Result<()> {
    fs::write(path, code)
        .with_context(|| format!("Failed to write to {}", path))
}

/// Print the header
fn print_header(description: &str, lang: Language, output: Option<&str>) {
    println!();
    println!(
        "{}{}  {} Code Generator{}",
        colors::PRIMARY, colors::BOLD, symbols::CODE, colors::RESET
    );
    println!(
        "{}  │ Language: {}{}{}",
        colors::MUTED, colors::FG, lang.name(), colors::RESET
    );
    if let Some(out) = output {
        println!(
            "{}  │ Output: {}{}{}",
            colors::MUTED, colors::FG, out, colors::RESET
        );
    }
    println!(
        "{}  ╰ {}{}{}",
        colors::MUTED, colors::DIM, description, colors::RESET
    );
    println!();
}

/// Print thinking indicator
fn print_thinking(lang: Language) {
    print_thinking_with_provider(lang, "AI");
}

/// Print thinking indicator with provider name
fn print_thinking_with_provider(lang: Language, provider: &str) {
    print!(
        "\r{}  {} Generating {} code via {} {}{}",
        colors::AI_ACCENT,
        symbols::AI_ICON,
        lang.name(),
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

/// Print code preview (when no output file)
fn print_code_preview(code: &str, _lang: Language) {
    println!();
    println!(
        "{}{}  {} Generated Code {}",
        colors::AI_ACCENT, colors::BOLD, symbols::AI_ICON, colors::RESET
    );
    println!(
        "{}  ╭{}─{}",
        colors::MUTED, "─".repeat(60), colors::RESET
    );

    let lines: Vec<&str> = code.lines().collect();
    let max_lines = 50; // Limit preview

    for (i, line) in lines.iter().take(max_lines).enumerate() {
        println!(
            "{}  │ {}{:>4}{} {}{}",
            colors::MUTED,
            colors::DIM,
            i + 1,
            colors::RESET,
            colors::FG,
            line
        );
    }

    if lines.len() > max_lines {
        println!(
            "{}  │ {}... ({} more lines){}",
            colors::MUTED, colors::DIM, lines.len() - max_lines, colors::RESET
        );
    }

    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(60), colors::RESET
    );
    println!();
}

/// Print file created message
fn print_file_created(path: &str, code: &str) {
    let lines = code.lines().count();
    let bytes = code.len();

    println!();
    println!(
        "{}{}  {} File Created {}",
        colors::SUCCESS, colors::BOLD, symbols::SUCCESS, colors::RESET
    );
    println!(
        "{}  ╭{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );
    println!(
        "{}  │ {} Path: {}{}{}",
        colors::MUTED, symbols::FILE, colors::FG, path, colors::RESET
    );
    println!(
        "{}  │   Lines: {}{}{}",
        colors::MUTED, colors::FG, lines, colors::RESET
    );
    println!(
        "{}  │   Size: {}{} bytes{}",
        colors::MUTED, colors::FG, bytes, colors::RESET
    );
    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );
    println!();
}

/// Print save suggestion
fn print_save_suggestion(filename: &str) {
    println!(
        "{}  💡 To save: {}nexus generate \"...\" -o {}{}",
        colors::MUTED, colors::FG, filename, colors::RESET
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
