//! Init command - Interactive setup wizard
//!
//! Claude Code style interactive setup for NEXUS AI Forge.

use anyhow::Result;

use crate::config::Config;
use crate::ui::{FormOption, NexusForm};

// ANSI color codes from design system
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const PRIMARY: &str = "\x1b[38;2;100;181;246m";
    pub const SUCCESS: &str = "\x1b[38;2;165;214;167m";
    pub const AI_ACCENT: &str = "\x1b[38;2;255;202;40m";
    pub const MUTED: &str = "\x1b[38;2;84;110;122m";
    pub const FG: &str = "\x1b[38;2;212;212;215m";
}

pub async fn run(_config: Config) -> Result<()> {
    print_banner();

    // Step 1: AI Provider Selection
    let provider_options = vec![
        FormOption::new("Claude (Anthropic)", "Best for complex reasoning and code review").recommended(),
        FormOption::new("GPT-4 (OpenAI)", "Fast completions, good for simple tasks"),
        FormOption::new("Gemini (Google)", "Long context, great for documentation"),
        FormOption::new("Local (Ollama)", "Privacy-first, offline capable"),
    ];

    let form = NexusForm::new();
    let provider_result = form.select("Which AI provider do you want to use?", &provider_options)?;

    let provider = match provider_result {
        crate::ui::FormResult::Single(idx) => idx,
        _ => 0,
    };

    // Step 2: Use case selection
    let usecase_options = vec![
        FormOption::new("Code Review & Analysis", "Security, performance, best practices checks").recommended(),
        FormOption::new("Code Generation", "Generate code from natural language"),
        FormOption::new("Learning & Exploration", "Understand and explain codebases"),
        FormOption::new("All of the above", "Full feature set"),
    ];

    let _usecase_result = form.select("What will you primarily use NEXUS for?", &usecase_options)?;

    // Step 3: Project type
    let project_options = vec![
        FormOption::new("Web Development", "React, Vue, Node.js, APIs"),
        FormOption::new("Systems Programming", "Rust, C/C++, low-level").recommended(),
        FormOption::new("Data Science / ML", "Python, Jupyter, TensorFlow"),
        FormOption::new("Mobile Development", "iOS, Android, React Native"),
    ];

    let _project_result = form.select("What type of projects do you work on?", &project_options)?;

    // Step 4: API Key check
    let has_api_key = std::env::var("ANTHROPIC_API_KEY").is_ok();

    if !has_api_key && provider == 0 {
        println!();
        println!(
            "{}{}  ⚠ API Key Required{}",
            colors::AI_ACCENT, colors::BOLD, colors::RESET
        );
        println!(
            "{}  To use Claude, set your API key:{}",
            colors::MUTED, colors::RESET
        );
        println!();
        println!(
            "{}  export ANTHROPIC_API_KEY=\"sk-ant-xxxxx\"{}",
            colors::FG, colors::RESET
        );
        println!();

        let setup_now = NexusForm::ask_confirm("Would you like to enter your API key now?", true)?;

        if setup_now {
            let api_key = NexusForm::ask_input("Enter your Anthropic API key:", None)?;
            println!();
            println!(
                "{}  Add this to your shell profile (~/.zshrc or ~/.bashrc):{}",
                colors::MUTED, colors::RESET
            );
            println!();
            println!(
                "{}  export ANTHROPIC_API_KEY=\"{}\"{}",
                colors::FG, api_key, colors::RESET
            );
        }
    }

    // Final summary
    print_setup_complete();

    Ok(())
}

fn print_banner() {
    println!();
    println!(
        "{}{}╭─────────────────────────────────────────────────────╮{}",
        colors::PRIMARY, colors::BOLD, colors::RESET
    );
    println!(
        "{}{}│  ███╗   ██╗███████╗██╗  ██╗██╗   ██╗███████╗      │{}",
        colors::PRIMARY, colors::BOLD, colors::RESET
    );
    println!(
        "{}{}│  ████╗  ██║██╔════╝╚██╗██╔╝██║   ██║██╔════╝      │{}",
        colors::PRIMARY, colors::BOLD, colors::RESET
    );
    println!(
        "{}{}│  ██╔██╗ ██║█████╗   ╚███╔╝ ██║   ██║███████╗      │{}",
        colors::PRIMARY, colors::BOLD, colors::RESET
    );
    println!(
        "{}{}│  ██║╚██╗██║██╔══╝   ██╔██╗ ██║   ██║╚════██║      │{}",
        colors::PRIMARY, colors::BOLD, colors::RESET
    );
    println!(
        "{}{}│  ██║ ╚████║███████╗██╔╝ ██╗╚██████╔╝███████║      │{}",
        colors::PRIMARY, colors::BOLD, colors::RESET
    );
    println!(
        "{}{}│  ╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝      │{}",
        colors::PRIMARY, colors::BOLD, colors::RESET
    );
    println!(
        "{}{}╰─────────────────────────────────────────────────────╯{}",
        colors::PRIMARY, colors::BOLD, colors::RESET
    );
    println!(
        "{}  󰌤 AI Forge Setup Wizard{}",
        colors::AI_ACCENT, colors::RESET
    );
    println!();
}

fn print_setup_complete() {
    println!();
    println!(
        "{}{}╭─────────────────────────────────────────────────────╮{}",
        colors::SUCCESS, colors::BOLD, colors::RESET
    );
    println!(
        "{}{}│  ✓ Setup Complete!                                  │{}",
        colors::SUCCESS, colors::BOLD, colors::RESET
    );
    println!(
        "{}{}╰─────────────────────────────────────────────────────╯{}",
        colors::SUCCESS, colors::BOLD, colors::RESET
    );
    println!();
    println!(
        "{}  Quick Start:{}",
        colors::MUTED, colors::RESET
    );
    println!();
    println!(
        "{}  {}nexus index .{}          Index your codebase{}",
        colors::MUTED, colors::FG, colors::MUTED, colors::RESET
    );
    println!(
        "{}  {}nexus ask \"question\"{}   Ask about your code{}",
        colors::MUTED, colors::FG, colors::MUTED, colors::RESET
    );
    println!(
        "{}  {}nexus review src/{}      Review for issues{}",
        colors::MUTED, colors::FG, colors::MUTED, colors::RESET
    );
    println!(
        "{}  {}nexus chat{}              Start AI conversation{}",
        colors::MUTED, colors::FG, colors::MUTED, colors::RESET
    );
    println!();
    println!(
        "{}  Full documentation: {}nexus --help{}",
        colors::MUTED, colors::FG, colors::RESET
    );
    println!();
}
