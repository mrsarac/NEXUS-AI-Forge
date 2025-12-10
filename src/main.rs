//! NEXUS AI Forge - The Ultimate AI-Augmented Developer Tool
//!
//! A blazing-fast, privacy-first, multi-model AI coding assistant
//! built in Rust for maximum performance.

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod ai;
mod cli;
mod config;
mod core;
mod index;
mod ui;

/// NEXUS AI Forge - Your AI Development Partner
#[derive(Parser)]
#[command(name = "nexus")]
#[command(author = "Mustafa Saraç <mustafa@mustafasarac.com>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "The ultimate AI-augmented developer tool", long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an interactive session
    Chat {
        /// Initial prompt
        prompt: Option<String>,
    },

    /// Ask a question about your codebase
    Ask {
        /// The question to ask
        question: String,
    },

    /// Fix bugs with AI assistance
    Fix {
        /// File containing the buggy code
        file: String,

        /// Error message to help diagnose the bug
        #[arg(short, long)]
        error: Option<String>,
    },

    /// Generate unit tests for code
    Test {
        /// File to generate tests for
        file: String,

        /// Output file for generated tests
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Generate AI-powered commit messages
    Commit {
        /// Execute the commit after generating message
        #[arg(short, long)]
        execute: bool,
    },

    /// Generate documentation for code
    Doc {
        /// File to document
        file: String,

        /// Output file for documentation
        #[arg(short, long)]
        output: Option<String>,

        /// Generate inline doc comments instead of separate docs
        #[arg(long)]
        inline: bool,
    },

    /// Refactor code with AI assistance
    Refactor {
        /// Files or directories to refactor
        #[arg(required = true)]
        paths: Vec<String>,

        /// Description of the refactoring
        #[arg(short, long)]
        description: String,
    },

    /// Search your codebase semantically
    Search {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Index your codebase for faster operations
    Index {
        /// Path to index (defaults to current directory)
        path: Option<String>,

        /// Force re-index
        #[arg(short, long)]
        force: bool,
    },

    /// Generate code from natural language
    Generate {
        /// Description of what to generate
        description: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,

        /// Language to generate
        #[arg(short, long)]
        language: Option<String>,
    },

    /// Review code for issues and improvements
    Review {
        /// Files to review
        #[arg(required = true)]
        paths: Vec<String>,

        /// Focus areas (e.g., security, performance)
        #[arg(short, long)]
        focus: Option<Vec<String>>,
    },

    /// Explain code
    Explain {
        /// File or code snippet to explain
        target: String,

        /// Explanation depth (brief, detailed, expert)
        #[arg(short, long, default_value = "detailed")]
        depth: String,
    },

    /// Show configuration
    Config {
        /// Show current configuration
        #[arg(long)]
        show: bool,

        /// Initialize configuration file
        #[arg(long)]
        init: bool,
    },

    /// Show version and system info
    Info,

    /// Interactive setup wizard
    Init,

    /// Update NEXUS to the latest version
    Update {
        /// Only check for updates, don't install
        #[arg(long)]
        check: bool,

        /// Force update even if already on latest version
        #[arg(long)]
        force: bool,
    },

    /// AI-powered git diff analysis
    Diff {
        /// Analyze staged changes only
        #[arg(short, long)]
        staged: bool,

        /// Specific file to analyze
        file: Option<String>,
    },

    /// Convert code between programming languages
    Convert {
        /// Source file to convert
        file: String,

        /// Target language (e.g., python, rust, typescript)
        #[arg(short, long)]
        to: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Analyze code for performance optimizations
    Optimize {
        /// File to analyze
        file: String,

        /// Focus area (time, memory, io, all)
        #[arg(short, long)]
        focus: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let level = if cli.verbose { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load configuration
    let config = config::load_config(cli.config.as_deref())?;

    info!("NEXUS AI Forge v{}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Some(Commands::Chat { prompt }) => {
            cli::chat::run(config, prompt).await?;
        }
        Some(Commands::Ask { question }) => {
            cli::ask::run(config, &question).await?;
        }
        Some(Commands::Fix { file, error }) => {
            cli::fix::run(config, &file, error.as_deref()).await?;
        }
        Some(Commands::Test { file, output }) => {
            cli::test::run(config, &file, output.as_deref()).await?;
        }
        Some(Commands::Commit { execute }) => {
            cli::commit::run(config, execute).await?;
        }
        Some(Commands::Doc { file, output, inline }) => {
            cli::doc::run(config, &file, output.as_deref(), inline).await?;
        }
        Some(Commands::Refactor { paths, description }) => {
            cli::refactor::run(config, &paths, &description).await?;
        }
        Some(Commands::Search { query, limit }) => {
            cli::search::run(config, &query, limit).await?;
        }
        Some(Commands::Index { path, force }) => {
            cli::index::run(config, path.as_deref(), force).await?;
        }
        Some(Commands::Generate { description, output, language }) => {
            cli::generate::run(config, &description, output.as_deref(), language.as_deref()).await?;
        }
        Some(Commands::Review { paths, focus }) => {
            cli::review::run(config, &paths, focus.as_deref()).await?;
        }
        Some(Commands::Explain { target, depth }) => {
            cli::explain::run(config, &target, &depth).await?;
        }
        Some(Commands::Config { show, init }) => {
            if init {
                config::init_config()?;
            } else if show {
                config::show_config(&config)?;
            }
        }
        Some(Commands::Info) => {
            cli::info::run()?;
        }
        Some(Commands::Init) => {
            cli::init::run(config).await?;
        }
        Some(Commands::Update { check, force }) => {
            if check {
                cli::update::check().await?;
            } else {
                cli::update::run(false, force).await?;
            }
        }
        Some(Commands::Diff { staged, file }) => {
            cli::diff::run(config, staged, file.as_deref()).await?;
        }
        Some(Commands::Convert { file, to, output }) => {
            cli::convert::run(config, &file, &to, output.as_deref()).await?;
        }
        Some(Commands::Optimize { file, focus }) => {
            cli::optimize::run(config, &file, focus.as_deref()).await?;
        }
        None => {
            // Default: Start interactive chat
            cli::chat::run(config, None).await?;
        }
    }

    Ok(())
}
