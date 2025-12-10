//! Review command - AI-powered code review
//!
//! Analyzes code for security vulnerabilities, performance issues,
//! and best practices violations.

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::path::Path;
use std::fs;
use std::io::{self, Write};

use crate::ai::{ClaudeClient, Conversation};
use crate::config::Config;
use crate::core::parser::{CodeParser, Language};

// ANSI color codes from design system
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const PRIMARY: &str = "\x1b[38;2;100;181;246m";      // #64B5F6
    pub const SUCCESS: &str = "\x1b[38;2;165;214;167m";      // #A5D6A7
    pub const WARNING: &str = "\x1b[38;2;255;202;40m";       // #FFCA28
    pub const ERROR: &str = "\x1b[38;2;239;154;154m";        // #EF9A9A
    pub const AI_ACCENT: &str = "\x1b[38;2;255;202;40m";     // #FFCA28
    pub const MUTED: &str = "\x1b[38;2;84;110;122m";         // #546E7A
    pub const FG: &str = "\x1b[38;2;212;212;215m";           // #D4D4D7
}

mod symbols {
    pub const AI_ICON: &str = "󰌤";
    pub const FILE: &str = "󰈙";
    pub const REVIEW: &str = "󰈈";
    pub const SECURITY: &str = "󰒃";
    pub const PERFORMANCE: &str = "󰓅";
    pub const BEST_PRACTICE: &str = "󰄭";
    pub const ERROR: &str = "󰅚";
    pub const WARNING: &str = "󰀦";
    pub const SUCCESS: &str = "󰄂";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// Focus areas for code review
#[derive(Debug, Clone, Copy)]
enum ReviewFocus {
    Security,
    Performance,
    BestPractices,
    All,
}

impl ReviewFocus {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "security" | "sec" => ReviewFocus::Security,
            "performance" | "perf" => ReviewFocus::Performance,
            "best-practices" | "bp" | "practices" => ReviewFocus::BestPractices,
            _ => ReviewFocus::All,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            ReviewFocus::Security => "Security",
            ReviewFocus::Performance => "Performance",
            ReviewFocus::BestPractices => "Best Practices",
            ReviewFocus::All => "Comprehensive",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            ReviewFocus::Security => symbols::SECURITY,
            ReviewFocus::Performance => symbols::PERFORMANCE,
            ReviewFocus::BestPractices => symbols::BEST_PRACTICE,
            ReviewFocus::All => symbols::REVIEW,
        }
    }
}

/// Get system prompt based on focus area
fn get_system_prompt(focus: ReviewFocus) -> &'static str {
    match focus {
        ReviewFocus::Security => r#"You are NEXUS AI, a security-focused code reviewer.

Your job is to identify security vulnerabilities and risks in code.

Focus Areas:
- Injection vulnerabilities (SQL, Command, XSS, etc.)
- Authentication/Authorization issues
- Sensitive data exposure
- Insecure cryptographic practices
- Input validation gaps
- Error handling that leaks information
- Hardcoded secrets or credentials
- Unsafe deserialization
- Path traversal vulnerabilities
- Race conditions

Output Format:
## Security Review

### Critical Issues 🔴
[List critical security issues with line numbers]

### High Risk 🟠
[List high-risk issues]

### Medium Risk 🟡
[List medium-risk issues]

### Recommendations
[Specific fixes with code examples]

Be specific, reference line numbers, and provide fix examples."#,

        ReviewFocus::Performance => r#"You are NEXUS AI, a performance-focused code reviewer.

Your job is to identify performance issues and optimization opportunities.

Focus Areas:
- Algorithm complexity (O(n²) loops, etc.)
- Memory leaks and unnecessary allocations
- N+1 query patterns in database code
- Blocking operations in async code
- Unnecessary cloning or copying
- Missing caching opportunities
- Inefficient data structures
- Hot path optimizations
- Resource cleanup issues
- Concurrency bottlenecks

Output Format:
## Performance Review

### Critical Performance Issues 🔴
[Major performance problems]

### Optimization Opportunities 🟡
[Places to improve]

### Memory Concerns 💾
[Memory-related issues]

### Recommendations
[Specific optimizations with benchmarks if applicable]

Be specific with complexity analysis and provide optimized alternatives."#,

        ReviewFocus::BestPractices => r#"You are NEXUS AI, a code quality reviewer.

Your job is to ensure code follows best practices and idioms.

Focus Areas:
- Code readability and clarity
- Naming conventions
- Function/method length and complexity
- DRY principle violations
- SOLID principles
- Error handling patterns
- Documentation gaps
- Test coverage suggestions
- Code organization
- Language-specific idioms

Output Format:
## Best Practices Review

### Code Smells 👃
[Anti-patterns and code smells]

### Readability Issues 📖
[Hard to understand code]

### Maintainability Concerns 🔧
[Future maintenance problems]

### Suggestions
[Specific improvements with examples]

Focus on making code more maintainable and idiomatic."#,

        ReviewFocus::All => r#"You are NEXUS AI, a comprehensive code reviewer.

You review code for security, performance, and best practices.

## Security Checklist
- Injection vulnerabilities
- Auth issues
- Data exposure
- Input validation

## Performance Checklist
- Algorithm complexity
- Memory efficiency
- Async patterns
- Caching opportunities

## Quality Checklist
- Code clarity
- Error handling
- Documentation
- Idioms

Output Format:
## Comprehensive Code Review

### 🔒 Security
[Security findings]

### ⚡ Performance
[Performance findings]

### ✨ Quality
[Best practices findings]

### Summary
- Critical issues: X
- Warnings: Y
- Suggestions: Z

### Top 3 Priorities
1. [Most important fix]
2. [Second priority]
3. [Third priority]

Be thorough but prioritized. Focus on actionable feedback."#,
    }
}

pub async fn run(_config: Config, paths: &[String], focus: Option<&[String]>) -> Result<()> {
    // Determine focus areas
    let focus_areas: Vec<ReviewFocus> = if let Some(areas) = focus {
        areas.iter().map(|s| ReviewFocus::from_str(s)).collect()
    } else {
        vec![ReviewFocus::All]
    };

    let primary_focus = focus_areas.first().copied().unwrap_or(ReviewFocus::All);

    // Print header
    print_header(paths, primary_focus);

    // Try to create Claude client
    let client = match ClaudeClient::from_env() {
        Ok(c) => c,
        Err(e) => {
            print_error(&format!("Could not initialize AI: {}", e));
            println!(
                "\n{}  To use review, set your Anthropic API key:{}",
                colors::MUTED, colors::RESET
            );
            println!(
                "{}  export ANTHROPIC_API_KEY=\"your-api-key\"{}",
                colors::FG, colors::RESET
            );
            return Ok(());
        }
    };

    // Collect all file contents
    let mut all_content = String::new();
    let mut file_count = 0;
    let mut total_lines = 0;
    let mut parser = CodeParser::new().context("Failed to initialize parser")?;

    for path_str in paths {
        let path = Path::new(path_str);

        if path.is_file() {
            if let Ok(content) = fs::read_to_string(path) {
                let line_count = content.lines().count();
                total_lines += line_count;
                file_count += 1;

                // Get language and parse for structure
                let language = Language::from_path(path);
                let structure_info = if language != Language::Unknown {
                    if let Ok(parsed) = parser.parse_file(path) {
                        let counts = parsed.symbol_counts();
                        format!(
                            "({}: {} functions, {} types)",
                            language.name(), counts.functions, counts.types
                        )
                    } else {
                        format!("({})", language.name())
                    }
                } else {
                    String::new()
                };

                all_content.push_str(&format!(
                    "\n## File: {} {}\n```{}\n{}\n```\n",
                    path_str,
                    structure_info,
                    language.name().to_lowercase(),
                    content
                ));
            }
        } else if path.is_dir() {
            // Walk directory for supported files
            for entry in walkdir::WalkDir::new(path)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| {
                    let name = e.file_name().to_string_lossy();
                    !name.starts_with('.') &&
                    name != "node_modules" &&
                    name != "target" &&
                    name != "build" &&
                    name != "dist" &&
                    name != "__pycache__" &&
                    name != "vendor"
                })
            {
                if let Ok(entry) = entry {
                    let file_path = entry.path();
                    if file_path.is_file() {
                        let language = Language::from_path(file_path);
                        if language != Language::Unknown {
                            if let Ok(content) = fs::read_to_string(file_path) {
                                let line_count = content.lines().count();
                                total_lines += line_count;
                                file_count += 1;

                                // Limit to reasonable size
                                if total_lines > 2000 {
                                    print_warning(&format!(
                                        "Limiting review to {} files ({} lines) for best results",
                                        file_count, total_lines
                                    ));
                                    break;
                                }

                                all_content.push_str(&format!(
                                    "\n## File: {}\n```{}\n{}\n```\n",
                                    file_path.display(),
                                    language.name().to_lowercase(),
                                    content
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    if file_count == 0 {
        print_error("No supported files found to review");
        return Ok(());
    }

    print_stats(file_count, total_lines);

    // Build prompt
    let prompt = format!(
        "Please review the following code:\n{}\n\nProvide a thorough {} review.",
        all_content, primary_focus.name().to_lowercase()
    );

    // Send to Claude
    print_thinking(primary_focus);

    let mut conversation = Conversation::new(client)
        .with_system(get_system_prompt(primary_focus));

    match conversation.send(&prompt).await {
        Ok(response) => {
            clear_line();
            print_response(&response, primary_focus);
        }
        Err(e) => {
            clear_line();
            print_error(&format!("AI error: {}", e));
        }
    }

    Ok(())
}

/// Print the header
fn print_header(paths: &[String], focus: ReviewFocus) {
    println!();
    println!(
        "{}{}  {} Code Review{}",
        colors::PRIMARY, colors::BOLD, symbols::REVIEW, colors::RESET
    );
    println!(
        "{}  │ Focus: {} {}{}",
        colors::MUTED, focus.icon(), focus.name(), colors::RESET
    );

    // Show files being reviewed
    for (i, path) in paths.iter().take(3).enumerate() {
        let prefix = if i == paths.len().min(3) - 1 { "╰" } else { "├" };
        println!(
            "{}  {} {} {}{}",
            colors::MUTED, prefix, symbols::FILE, path, colors::RESET
        );
    }
    if paths.len() > 3 {
        println!(
            "{}  ╰ ... and {} more{}",
            colors::MUTED, paths.len() - 3, colors::RESET
        );
    }

    println!(
        "{}  {}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );
    println!();
}

/// Print file stats
fn print_stats(file_count: usize, total_lines: usize) {
    println!(
        "{}  {} Analyzing {} files ({} lines)...{}",
        colors::MUTED, symbols::SUCCESS, file_count, total_lines, colors::RESET
    );
}

/// Print thinking indicator
fn print_thinking(focus: ReviewFocus) {
    print!(
        "\r{}  {} Reviewing for {} issues {}{}",
        colors::AI_ACCENT,
        symbols::AI_ICON,
        focus.name().to_lowercase(),
        symbols::SPINNER[0],
        colors::RESET
    );
    io::stdout().flush().ok();
}

/// Clear the current line
fn clear_line() {
    print!("\r{}\r", " ".repeat(70));
    io::stdout().flush().ok();
}

/// Print the AI response
fn print_response(response: &str, focus: ReviewFocus) {
    println!();
    println!(
        "{}{}  {} {} Review Complete {}",
        colors::AI_ACCENT, colors::BOLD, focus.icon(), focus.name(), colors::RESET
    );
    println!(
        "{}  ╭{}─{}",
        colors::MUTED, "─".repeat(60), colors::RESET
    );

    for line in response.lines() {
        // Color code different severity levels
        let colored_line = if line.contains("Critical") || line.contains("🔴") {
            format!("{}  │ {}{}{}", colors::MUTED, colors::ERROR, line, colors::RESET)
        } else if line.contains("High Risk") || line.contains("🟠") {
            format!("{}  │ {}{}{}", colors::MUTED, colors::WARNING, line, colors::RESET)
        } else if line.contains("Medium") || line.contains("🟡") {
            format!("{}  │ {}{}{}", colors::MUTED, colors::AI_ACCENT, line, colors::RESET)
        } else if line.starts_with("##") {
            format!("{}  │ {}{}{}{}", colors::MUTED, colors::PRIMARY, colors::BOLD, line, colors::RESET)
        } else if line.starts_with("###") {
            format!("{}  │ {}{}{}", colors::MUTED, colors::PRIMARY, line, colors::RESET)
        } else {
            format!("{}  │ {}{}", colors::MUTED, colors::FG, line)
        };
        println!("{}", colored_line);
    }

    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(60), colors::RESET
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
        "{}  {} {}{}",
        colors::WARNING, symbols::WARNING, message, colors::RESET
    );
}
