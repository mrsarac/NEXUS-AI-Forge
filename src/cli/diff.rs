//! Diff command - AI-powered git diff analysis
//!
//! Analyzes git diffs and provides insights about changes.

#![allow(dead_code)]

use anyhow::Result;
use std::io::{self, Write};
use std::process::Command;

use crate::ai::{ClaudeClient, Conversation, ProxyClient};
use crate::config::Config;

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
    pub const ADDED: &str = "\x1b[38;2;129;199;132m";        // Green
    pub const REMOVED: &str = "\x1b[38;2;229;115;115m";      // Red
}

mod symbols {
    pub const DIFF: &str = "󰦓";
    pub const AI_ICON: &str = "󰌤";
    pub const FILE: &str = "󰈙";
    pub const SUCCESS: &str = "󰄂";
    pub const ERROR: &str = "󰅚";
    pub const ADDED: &str = "+";
    pub const REMOVED: &str = "-";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// System prompt for diff analysis
const DIFF_PROMPT: &str = r#"You are NEXUS AI, an expert code reviewer analyzing git diffs.

## Your Task
Analyze the provided git diff and provide insights about the changes.

## Analysis Format
Provide your analysis in this structure:

### Summary
A brief overview of what changed (2-3 sentences)

### Changes Breakdown
- List each significant change
- Group related changes together
- Note any patterns

### Risk Assessment
- **High Risk**: Breaking changes, security implications
- **Medium Risk**: Logic changes, new dependencies
- **Low Risk**: Formatting, comments, minor tweaks

### Suggestions
- Any improvements or concerns
- Potential bugs introduced
- Best practices recommendations

Keep the analysis concise but thorough."#;

/// Determine which AI mode to use
fn determine_ai_mode() -> AiMode {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        AiMode::Claude
    } else {
        AiMode::Proxy
    }
}

/// Check if we're in a git repository
fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get git diff output
fn get_diff(staged: bool, file: Option<&str>) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.arg("diff");

    if staged {
        cmd.arg("--cached");
    }

    if let Some(f) = file {
        cmd.arg(f);
    }

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git diff failed: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Get diff stats
fn get_diff_stats(staged: bool) -> Result<(usize, usize, usize)> {
    let mut cmd = Command::new("git");
    cmd.args(["diff", "--stat"]);

    if staged {
        cmd.arg("--cached");
    }

    let output = cmd.output()?;
    let stat_output = String::from_utf8_lossy(&output.stdout);

    // Count files, additions, deletions from stat output
    let mut files = 0;
    let mut additions = 0;
    let mut deletions = 0;

    for line in stat_output.lines() {
        if line.contains("|") {
            files += 1;
            // Count + and - in the line
            additions += line.matches('+').count();
            deletions += line.matches('-').count();
        }
    }

    Ok((files, additions, deletions))
}

pub async fn run(_config: Config, staged: bool, file: Option<&str>) -> Result<()> {
    print_header(staged, file);

    // Check if in git repo
    if !is_git_repo() {
        print_error("Not a git repository");
        return Ok(());
    }

    // Get the diff
    let diff = get_diff(staged, file)?;

    if diff.trim().is_empty() {
        print_no_changes(staged);
        return Ok(());
    }

    // Get stats
    let (files, additions, deletions) = get_diff_stats(staged)?;
    print_diff_stats(files, additions, deletions);

    let ai_mode = determine_ai_mode();
    let provider_name = match ai_mode {
        AiMode::Claude => "Claude",
        AiMode::Proxy => "NEXUS AI (Free)",
    };

    // Prepare prompt
    let prompt = format!(
        "## Git Diff to Analyze\n\n```diff\n{}\n```\n\n## Statistics\n- Files changed: {}\n- Additions: {}\n- Deletions: {}\n\nPlease analyze this diff.",
        diff, files, additions, deletions
    );

    // Send to AI
    print_thinking(provider_name);

    let response = match ai_mode {
        AiMode::Claude => {
            let client = ClaudeClient::from_env()?;
            let mut conversation = Conversation::new(client)
                .with_system(DIFF_PROMPT);

            conversation.send(&prompt).await?
        }
        AiMode::Proxy => {
            let proxy = ProxyClient::from_env();
            let prompt_with_system = format!("{}\n\n{}", DIFF_PROMPT, prompt);
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

fn print_header(staged: bool, file: Option<&str>) {
    println!();
    println!(
        "{}{}  {} AI Diff Analysis{}",
        colors::PRIMARY, colors::BOLD, symbols::DIFF, colors::RESET
    );

    let scope = if let Some(f) = file {
        format!("File: {}", f)
    } else if staged {
        "Staged changes".to_string()
    } else {
        "Working directory".to_string()
    };

    println!(
        "{}  │ Scope: {}{}",
        colors::MUTED, scope, colors::RESET
    );
    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );
    println!();
}

fn print_diff_stats(files: usize, additions: usize, deletions: usize) {
    println!(
        "{}  {} {} file(s) changed",
        colors::MUTED, symbols::FILE, files
    );
    println!(
        "{}  {} {} insertion(s)  {}{}  {} {} deletion(s){}",
        colors::ADDED, symbols::ADDED, additions,
        colors::REMOVED, symbols::REMOVED, deletions,
        colors::RESET, ""
    );
    println!();
}

fn print_no_changes(staged: bool) {
    let scope = if staged { "staged" } else { "unstaged" };
    println!(
        "{}  {} No {} changes to analyze{}",
        colors::WARNING, symbols::SUCCESS, scope, colors::RESET
    );
    println!();
}

fn print_thinking(provider: &str) {
    print!(
        "\r{}  {} {} is analyzing diff {}{}",
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
        "{}{}  {} Analysis Results{}",
        colors::SUCCESS, colors::BOLD, symbols::DIFF, colors::RESET
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
