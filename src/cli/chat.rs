//! Interactive chat command for NEXUS AI Forge
//!
//! Provides a beautiful CLI chat interface with AI assistance.

#![allow(dead_code)]

use anyhow::Result;
use std::io::{self, Write};

use crate::ai::{ClaudeClient, Conversation, ProxyClient};
use crate::ai::claude::prompts;
use crate::config::Config;

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
    pub const DIM: &str = "\x1b[2m";

    // Design system colors
    pub const PRIMARY: &str = "\x1b[38;2;100;181;246m";      // #64B5F6
    pub const SUCCESS: &str = "\x1b[38;2;165;214;167m";      // #A5D6A7
    pub const ERROR: &str = "\x1b[38;2;239;154;154m";        // #EF9A9A
    pub const AI_ACCENT: &str = "\x1b[38;2;255;202;40m";     // #FFCA28
    pub const MUTED: &str = "\x1b[38;2;84;110;122m";         // #546E7A
    pub const FG: &str = "\x1b[38;2;212;212;215m";           // #D4D4D7
}

// Unicode symbols
mod symbols {
    pub const AI_ICON: &str = "󰌤";
    pub const USER_ICON: &str = ">";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    pub const SUCCESS: &str = "󰄂";
    pub const ERROR: &str = "󰅚";
    pub const DIVIDER: &str = "─";
}

/// Print a horizontal divider
fn print_divider() {
    println!(
        "{}{}{}",
        colors::MUTED,
        symbols::DIVIDER.repeat(55),
        colors::RESET
    );
}

/// Print user message bubble
fn print_user_message(content: &str) {
    println!();
    println!(
        "{}{}  You {}{}",
        colors::PRIMARY, colors::BOLD, colors::RESET, colors::MUTED
    );
    for line in content.lines() {
        println!("{}  │ {}{}", colors::MUTED, colors::FG, line);
    }
    println!("{}  ╰{}─{}", colors::MUTED, symbols::DIVIDER.repeat(50), colors::RESET);
}

/// Print AI response bubble
fn print_ai_message(content: &str) {
    println!();
    println!(
        "{}{}  {} Nexus AI {}{}",
        colors::AI_ACCENT, colors::BOLD, symbols::AI_ICON, colors::RESET, colors::MUTED
    );
    for line in content.lines() {
        println!("{}  │ {}{}", colors::MUTED, colors::FG, line);
    }
    println!("{}  ╰{}─{}", colors::MUTED, symbols::DIVIDER.repeat(50), colors::RESET);
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

/// Clear thinking indicator
fn clear_thinking() {
    print!("\r{}\r", " ".repeat(50));
    io::stdout().flush().ok();
}

/// Print error message
fn print_error(message: &str) {
    println!(
        "\n{}  {} Error: {}{}",
        colors::ERROR, symbols::ERROR, message, colors::RESET
    );
}

/// Print success message
fn print_success(message: &str) {
    println!(
        "\n{}  {} {}{}",
        colors::SUCCESS, symbols::SUCCESS, message, colors::RESET
    );
}

/// Print help information
fn print_help() {
    println!();
    println!(
        "{}{}  Available Commands:{}",
        colors::PRIMARY, colors::BOLD, colors::RESET
    );
    println!(
        "{}  /help{}    - Show this help message",
        colors::FG, colors::MUTED
    );
    println!(
        "{}  /clear{}   - Clear conversation history",
        colors::FG, colors::MUTED
    );
    println!(
        "{}  /exit{}    - Exit the chat",
        colors::FG, colors::MUTED
    );
    println!(
        "{}  /model{}   - Show current AI model",
        colors::FG, colors::MUTED
    );
    println!();
    println!(
        "{}  Tips:{}",
        colors::PRIMARY, colors::RESET
    );
    println!(
        "{}  • Type your message and press Enter twice to send",
        colors::MUTED
    );
    println!(
        "{}  • Use ``` for code blocks",
        colors::MUTED
    );
    println!(
        "{}  • Paste code directly - I'll understand it",
        colors::MUTED
    );
    println!();
}

/// Read multi-line input from user
fn read_input() -> Option<String> {
    print!(
        "\n{}  {} {}",
        colors::PRIMARY, symbols::USER_ICON, colors::RESET
    );
    io::stdout().flush().ok();

    let mut lines = Vec::new();
    let mut empty_count = 0;

    loop {
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => return None, // EOF
            Ok(_) => {
                let trimmed = line.trim_end();

                if trimmed.is_empty() {
                    empty_count += 1;
                    if empty_count >= 1 && !lines.is_empty() {
                        // Double enter = send
                        break;
                    }
                } else {
                    empty_count = 0;
                    lines.push(trimmed.to_string());
                    // Continue prompt
                    print!(
                        "{}  {} {}",
                        colors::MUTED, ".", colors::RESET
                    );
                    io::stdout().flush().ok();
                }
            }
            Err(_) => return None,
        }
    }

    let input = lines.join("\n").trim().to_string();
    if input.is_empty() {
        None
    } else {
        Some(input)
    }
}

/// Determine which AI mode to use
fn determine_ai_mode() -> AiMode {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        AiMode::Claude
    } else {
        AiMode::Proxy
    }
}

/// Main chat loop
pub async fn run(_config: Config, initial_prompt: Option<String>) -> Result<()> {
    let ai_mode = determine_ai_mode();

    match ai_mode {
        AiMode::Claude => run_with_claude(initial_prompt).await,
        AiMode::Proxy => run_with_proxy(initial_prompt).await,
    }
}

/// Run chat with Claude (requires API key)
async fn run_with_claude(initial_prompt: Option<String>) -> Result<()> {
    let client = ClaudeClient::from_env()?;
    let mut conversation = Conversation::new(client)
        .with_system(prompts::CODING_ASSISTANT);

    print_banner_with_provider("Claude");

    // Handle initial prompt
    if let Some(prompt) = initial_prompt {
        print_user_message(&prompt);
        print_thinking();

        match conversation.send(&prompt).await {
            Ok(response) => {
                clear_thinking();
                print_ai_message(&response);
            }
            Err(e) => {
                clear_thinking();
                print_error(&format!("AI error: {}", e));
            }
        }
    }

    // Main chat loop
    loop {
        let input = match read_input() {
            Some(i) => i,
            None => {
                println!();
                break;
            }
        };

        // Handle commands
        if let Some(should_break) = handle_command(&input, Some(&mut conversation), AiMode::Claude) {
            if should_break {
                break;
            }
            continue;
        }

        // Send message to AI
        print_user_message(&input);
        print_thinking();

        match conversation.send(&input).await {
            Ok(response) => {
                clear_thinking();
                print_ai_message(&response);
            }
            Err(e) => {
                clear_thinking();
                print_error(&format!("AI error: {}", e));
            }
        }
    }

    println!();
    Ok(())
}

/// Run chat with NEXUS Proxy (free tier, Gemini-powered)
async fn run_with_proxy(initial_prompt: Option<String>) -> Result<()> {
    let proxy = ProxyClient::from_env();
    let mut history: Vec<String> = Vec::new();

    print_banner_with_provider("NEXUS AI (Free)");

    // Handle initial prompt
    if let Some(prompt) = initial_prompt {
        print_user_message(&prompt);
        print_thinking();

        let context = if history.is_empty() {
            None
        } else {
            Some(history.join("\n\n"))
        };

        match proxy.chat(&prompt, context.as_deref()).await {
            Ok(response) => {
                clear_thinking();
                print_ai_message(&response);
                history.push(format!("User: {}", prompt));
                history.push(format!("Assistant: {}", response));
            }
            Err(e) => {
                clear_thinking();
                print_error(&format!("AI error: {}", e));
            }
        }
    }

    // Main chat loop
    loop {
        let input = match read_input() {
            Some(i) => i,
            None => {
                println!();
                break;
            }
        };

        // Handle commands
        if let Some(should_break) = handle_command_proxy(&input, &mut history) {
            if should_break {
                break;
            }
            continue;
        }

        // Send message to AI
        print_user_message(&input);
        print_thinking();

        let context = if history.is_empty() {
            None
        } else {
            Some(history.join("\n\n"))
        };

        match proxy.chat(&input, context.as_deref()).await {
            Ok(response) => {
                clear_thinking();
                print_ai_message(&response);
                history.push(format!("User: {}", input));
                history.push(format!("Assistant: {}", response));
            }
            Err(e) => {
                clear_thinking();
                print_error(&format!("AI error: {}", e));
            }
        }
    }

    println!();
    Ok(())
}

/// Handle slash commands for Claude mode
fn handle_command(input: &str, conversation: Option<&mut Conversation>, mode: AiMode) -> Option<bool> {
    if !input.starts_with('/') {
        return None;
    }

    match input.to_lowercase().as_str() {
        "/exit" | "/quit" | "/q" => {
            print_success("Goodbye! Happy coding!");
            Some(true)
        }
        "/help" | "/h" | "/?" => {
            print_help();
            Some(false)
        }
        "/clear" | "/c" => {
            if let Some(conv) = conversation {
                conv.clear();
            }
            print_success("Conversation cleared");
            Some(false)
        }
        "/model" | "/m" => {
            let model_name = match mode {
                AiMode::Claude => "Claude (claude-sonnet-4-20250514)",
                AiMode::Proxy => "NEXUS AI Free (Gemini 2.0 Flash)",
            };
            println!(
                "\n{}  Current model: {}{}",
                colors::MUTED, model_name, colors::RESET
            );
            Some(false)
        }
        _ => {
            print_error(&format!("Unknown command: {}", input));
            println!("{}  Type /help for available commands{}", colors::MUTED, colors::RESET);
            Some(false)
        }
    }
}

/// Handle slash commands for Proxy mode
fn handle_command_proxy(input: &str, history: &mut Vec<String>) -> Option<bool> {
    if !input.starts_with('/') {
        return None;
    }

    match input.to_lowercase().as_str() {
        "/exit" | "/quit" | "/q" => {
            print_success("Goodbye! Happy coding!");
            Some(true)
        }
        "/help" | "/h" | "/?" => {
            print_help();
            Some(false)
        }
        "/clear" | "/c" => {
            history.clear();
            print_success("Conversation cleared");
            Some(false)
        }
        "/model" | "/m" => {
            println!(
                "\n{}  Current model: NEXUS AI Free (Gemini 2.0 Flash){}",
                colors::MUTED, colors::RESET
            );
            Some(false)
        }
        _ => {
            print_error(&format!("Unknown command: {}", input));
            println!("{}  Type /help for available commands{}", colors::MUTED, colors::RESET);
            Some(false)
        }
    }
}

/// Print banner with provider info
fn print_banner_with_provider(provider: &str) {
    println!();
    println!(
        "{}{}╭─────────────────────────────────────────────────────╮{}",
        colors::PRIMARY, colors::BOLD, colors::RESET
    );
    println!(
        "{}│{}  ███╗   ██╗███████╗██╗  ██╗██╗   ██╗███████╗     {}│{}",
        colors::PRIMARY, colors::RESET, colors::PRIMARY, colors::RESET
    );
    println!(
        "{}│{}  ████╗  ██║██╔════╝╚██╗██╔╝██║   ██║██╔════╝     {}│{}",
        colors::PRIMARY, colors::RESET, colors::PRIMARY, colors::RESET
    );
    println!(
        "{}│{}  ██╔██╗ ██║█████╗   ╚███╔╝ ██║   ██║███████╗     {}│{}",
        colors::PRIMARY, colors::RESET, colors::PRIMARY, colors::RESET
    );
    println!(
        "{}│{}  ██║╚██╗██║██╔══╝   ██╔██╗ ██║   ██║╚════██║     {}│{}",
        colors::PRIMARY, colors::RESET, colors::PRIMARY, colors::RESET
    );
    println!(
        "{}│{}  ██║ ╚████║███████╗██╔╝ ██╗╚██████╔╝███████║     {}│{}",
        colors::PRIMARY, colors::RESET, colors::PRIMARY, colors::RESET
    );
    println!(
        "{}│{}  ╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝     {}│{}",
        colors::PRIMARY, colors::RESET, colors::PRIMARY, colors::RESET
    );
    println!(
        "{}╰─────────────────────────────────────────────────────╯{}",
        colors::PRIMARY, colors::RESET
    );
    println!(
        "{}  {} AI Forge v{} - {}{}",
        colors::AI_ACCENT,
        symbols::AI_ICON,
        env!("CARGO_PKG_VERSION"),
        provider,
        colors::RESET
    );
    println!();
    println!(
        "{}  Commands: /help, /clear, /exit{}",
        colors::MUTED, colors::RESET
    );
    println!(
        "{}  Press Enter twice to send your message{}",
        colors::MUTED, colors::RESET
    );
    print_divider();
}
