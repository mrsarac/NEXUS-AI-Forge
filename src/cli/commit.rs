//! Commit command - AI-powered commit message generation
//!
//! Analyzes staged changes and generates semantic commit messages.

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::io::{self, Write};
use std::process::Command;

use crate::ai::ProxyClient;
use crate::config::Config;

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
    pub const COMMIT: &str = "󰜘";
    pub const AI_ICON: &str = "󰌤";
    pub const SUCCESS: &str = "󰄂";
    pub const ERROR: &str = "󰅚";
    pub const GIT: &str = "󰊢";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// System prompt for commit messages
const COMMIT_PROMPT: &str = r#"You are NEXUS AI, an expert at writing git commit messages.

Based on the git diff provided, generate a semantic commit message following these rules:

## Format
```
<type>(<scope>): <subject>

<body>
```

## Types
- feat: New feature
- fix: Bug fix
- docs: Documentation changes
- style: Code style changes (formatting, semicolons, etc.)
- refactor: Code refactoring without functionality change
- perf: Performance improvements
- test: Adding or updating tests
- chore: Maintenance tasks, dependencies, configs
- ci: CI/CD changes

## Rules
1. Subject line: max 50 characters, imperative mood ("add" not "added")
2. Body: wrap at 72 characters, explain what and why (not how)
3. Scope is optional but helpful for larger projects
4. Keep it concise but informative

## Output
Provide ONLY the commit message, no explanations or markdown formatting."#;

pub async fn run(_config: Config, execute: bool) -> Result<()> {
    print_header();

    // Check if we're in a git repository
    if !is_git_repo() {
        print_error("Not a git repository");
        return Ok(());
    }

    // Get staged changes
    let diff = get_staged_diff()?;

    if diff.is_empty() {
        print_error("No staged changes. Use 'git add' first.");
        return Ok(());
    }

    // Get changed files summary
    let files = get_staged_files()?;
    print_changes_summary(&files, &diff);

    // Generate commit message
    print_thinking();

    let proxy = ProxyClient::from_env();
    let prompt = format!(
        "{}\n\n## Git Diff\n\n```diff\n{}\n```\n\n## Changed Files\n{}\n\nGenerate a commit message:",
        COMMIT_PROMPT,
        truncate_diff(&diff, 4000),
        files.join("\n")
    );

    let response = proxy.chat(&prompt, None).await?;
    clear_line();

    let commit_msg = response.trim();
    print_commit_message(commit_msg);

    if execute {
        // Execute the commit
        print_committing();
        execute_commit(commit_msg)?;
        print_success();
    } else {
        // Show copy hint
        print_copy_hint(commit_msg);
    }

    Ok(())
}

/// Check if current directory is a git repository
fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get staged diff
fn get_staged_diff() -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--no-color"])
        .output()
        .context("Failed to run git diff")?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Get list of staged files
fn get_staged_files() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-status"])
        .output()
        .context("Failed to get staged files")?;

    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let status = match parts[0] {
                    "A" => "added",
                    "M" => "modified",
                    "D" => "deleted",
                    "R" => "renamed",
                    _ => parts[0],
                };
                format!("  {} {}", status, parts[1])
            } else {
                line.to_string()
            }
        })
        .collect();

    Ok(files)
}

/// Truncate diff to fit in context window
fn truncate_diff(diff: &str, max_len: usize) -> String {
    if diff.len() <= max_len {
        diff.to_string()
    } else {
        format!("{}...\n[diff truncated]", &diff[..max_len])
    }
}

/// Execute git commit with the message
fn execute_commit(message: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["commit", "-m", message])
        .status()
        .context("Failed to execute git commit")?;

    if !status.success() {
        return Err(anyhow::anyhow!("Git commit failed"));
    }

    Ok(())
}

// ============================================
// UI Functions
// ============================================

fn print_header() {
    println!();
    println!(
        "{}{}  {} AI Commit Message{}",
        colors::PRIMARY, colors::BOLD, symbols::COMMIT, colors::RESET
    );
    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(40), colors::RESET
    );
    println!();
}

fn print_changes_summary(files: &[String], diff: &str) {
    let additions = diff.lines().filter(|l| l.starts_with('+')).count();
    let deletions = diff.lines().filter(|l| l.starts_with('-')).count();

    println!(
        "{}  {} Changes: {} files, {}+{} {}-{}{}",
        colors::MUTED,
        symbols::GIT,
        files.len(),
        colors::SUCCESS, additions,
        colors::ERROR, deletions,
        colors::RESET
    );
    println!();

    for file in files.iter().take(10) {
        println!("{}  {}{}", colors::MUTED, file, colors::RESET);
    }

    if files.len() > 10 {
        println!(
            "{}  ... and {} more files{}",
            colors::MUTED, files.len() - 10, colors::RESET
        );
    }

    println!();
}

fn print_thinking() {
    print!(
        "\r{}  {} Generating commit message {}{}",
        colors::WARNING,
        symbols::AI_ICON,
        symbols::SPINNER[0],
        colors::RESET
    );
    io::stdout().flush().ok();
}

fn clear_line() {
    print!("\r{}\r", " ".repeat(60));
    io::stdout().flush().ok();
}

fn print_commit_message(message: &str) {
    println!(
        "{}{}  {} Suggested Commit Message{}",
        colors::SUCCESS, colors::BOLD, symbols::COMMIT, colors::RESET
    );
    println!(
        "{}  ╭{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );

    for line in message.lines() {
        println!("{}  │ {}{}{}", colors::MUTED, colors::FG, line, colors::RESET);
    }

    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );
    println!();
}

fn print_committing() {
    print!(
        "\r{}  {} Committing {}{}",
        colors::PRIMARY,
        symbols::GIT,
        symbols::SPINNER[0],
        colors::RESET
    );
    io::stdout().flush().ok();
}

fn print_success() {
    println!(
        "\r{}  {} Commit successful!{}",
        colors::SUCCESS, symbols::SUCCESS, colors::RESET
    );
    println!();
}

fn print_copy_hint(message: &str) {
    println!(
        "{}  💡 Use 'nexus commit --execute' to commit automatically{}",
        colors::MUTED, colors::RESET
    );
    println!();
    println!(
        "{}  Or copy the message:{}",
        colors::MUTED, colors::RESET
    );
    println!(
        "{}  git commit -m \"{}\"{}",
        colors::FG,
        message.lines().next().unwrap_or(""),
        colors::RESET
    );
    println!();
}

fn print_error(message: &str) {
    println!(
        "\n{}  {} Error: {}{}",
        colors::ERROR, symbols::ERROR, message, colors::RESET
    );
}
