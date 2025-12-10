//! Codebase indexing with beautiful CLI UI
//!
//! Indexes source files using tree-sitter for context-aware AI assistance.

#![allow(dead_code)]

pub mod semantic;

use std::path::{Path, PathBuf};
use std::time::Instant;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use walkdir::WalkDir;
use ignore::gitignore::Gitignore;

use crate::core::parser::{CodeParser, Language, ParsedFile, SymbolCounts};

// ANSI color codes from design system
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const PRIMARY: &str = "\x1b[38;2;100;181;246m";      // #64B5F6
    pub const SUCCESS: &str = "\x1b[38;2;165;214;167m";      // #A5D6A7
    pub const WARNING: &str = "\x1b[38;2;255;245;157m";      // #FFF59D
    pub const ERROR: &str = "\x1b[38;2;239;154;154m";        // #EF9A9A
    pub const AI_ACCENT: &str = "\x1b[38;2;255;202;40m";     // #FFCA28
    pub const MUTED: &str = "\x1b[38;2;84;110;122m";         // #546E7A
    pub const FG: &str = "\x1b[38;2;212;212;215m";           // #D4D4D7
}

mod symbols {
    pub const LOADING: &str = "󰊍";
    pub const SUCCESS: &str = "󰄂";
    pub const WARNING: &str = "⚠";
    pub const ERROR: &str = "󰅚";
    pub const DIVIDER: &str = "─";
}

/// Index a directory and return statistics
pub async fn index_directory(path: &Path, _force: bool, verbose: bool) -> Result<IndexResult> {
    let start_time = Instant::now();
    let abs_path = path.canonicalize()
        .with_context(|| format!("Invalid path: {}", path.display()))?;

    // Print header
    print_header(&abs_path);

    // Collect files to index
    let files = collect_files(&abs_path, verbose)?;

    if files.is_empty() {
        print_warning("No supported files found in directory");
        return Ok(IndexResult::empty());
    }

    // Create parser
    let mut parser = CodeParser::new()
        .context("Failed to initialize code parser")?;

    // Create progress bar
    let pb = create_progress_bar(files.len() as u64);

    // Parse all files
    let mut parsed_files: Vec<ParsedFile> = Vec::new();
    let mut errors: Vec<(PathBuf, String)> = Vec::new();
    let mut total_symbols = SymbolCounts::default();

    for file_path in &files {
        let relative_path = file_path.strip_prefix(&abs_path).unwrap_or(file_path);
        pb.set_message(format!("{}", relative_path.display()));

        match parser.parse_file(file_path) {
            Ok(parsed) => {
                let counts = parsed.symbol_counts();
                total_symbols.functions += counts.functions;
                total_symbols.types += counts.types;
                total_symbols.enums += counts.enums;
                total_symbols.traits += counts.traits;
                total_symbols.modules += counts.modules;
                total_symbols.constants += counts.constants;
                total_symbols.impls += counts.impls;
                parsed_files.push(parsed);
            }
            Err(e) => {
                if verbose {
                    errors.push((file_path.clone(), e.to_string()));
                }
            }
        }

        pb.inc(1);
    }

    pb.finish_and_clear();

    let duration = start_time.elapsed();

    // Build result
    let result = IndexResult {
        files_indexed: parsed_files.len(),
        files_skipped: errors.len(),
        total_lines: parsed_files.iter().map(|f| f.line_count).sum(),
        symbols: total_symbols,
        time_taken_ms: duration.as_millis() as u64,
        errors,
    };

    // Print summary
    print_summary(&result, &abs_path);

    Ok(result)
}

/// Collect all supported source files in directory
fn collect_files(path: &Path, _verbose: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    // Try to load .gitignore
    let gitignore_path = path.join(".gitignore");
    let gitignore = if gitignore_path.exists() {
        Gitignore::new(&gitignore_path).0
    } else {
        Gitignore::empty()
    };

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            let name = e.file_name().to_string_lossy();

            // Skip hidden directories and common non-source dirs
            if name.starts_with('.') { return false; }
            if name == "node_modules" { return false; }
            if name == "target" { return false; }
            if name == "build" { return false; }
            if name == "dist" { return false; }
            if name == "__pycache__" { return false; }
            if name == ".git" { return false; }
            if name == "vendor" { return false; }

            // Check gitignore
            if gitignore.matched(path, path.is_dir()).is_ignore() {
                return false;
            }

            true
        })
    {
        let entry = entry?;
        let file_path = entry.path();

        if file_path.is_file() {
            let language = Language::from_path(file_path);
            if language != Language::Unknown {
                files.push(file_path.to_path_buf());
            }
        }
    }

    Ok(files)
}

/// Create a styled progress bar
fn create_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);

    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.cyan} {prefix:.bold} [{bar:40.cyan/dim}] {pos}/{len} {msg:.dim}")
        .unwrap()
        .progress_chars("█▓░")
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]));

    pb.set_prefix("Parsing");
    pb.enable_steady_tick(std::time::Duration::from_millis(80));

    pb
}

/// Print the indexing header
fn print_header(path: &Path) {
    println!();
    println!(
        "{}{}╭─ {} NEXUS AI Forge ──────────────────────────────────────────╮{}",
        colors::PRIMARY, colors::BOLD, symbols::LOADING, colors::RESET
    );
    println!(
        "{}│{}                                                              {}│{}",
        colors::PRIMARY, colors::RESET, colors::PRIMARY, colors::RESET
    );
    println!(
        "{}│{}  Target: {}{}{}{}",
        colors::PRIMARY, colors::RESET, colors::FG,
        truncate_path(path, 50), colors::PRIMARY, colors::RESET
    );
    println!(
        "{}│{}  Mode:   {}Deep AST Analysis (Tree-sitter){}                   {}│{}",
        colors::PRIMARY, colors::RESET, colors::MUTED, colors::RESET, colors::PRIMARY, colors::RESET
    );
    println!(
        "{}│{}                                                              {}│{}",
        colors::PRIMARY, colors::RESET, colors::PRIMARY, colors::RESET
    );
    println!(
        "{}╰──────────────────────────────────────────────────────────────╯{}",
        colors::PRIMARY, colors::RESET
    );
    println!();
}

/// Print the indexing summary
fn print_summary(result: &IndexResult, _path: &Path) {
    println!();

    let (icon, color, title) = if result.files_skipped > 0 {
        (symbols::WARNING, colors::WARNING, "Indexing Completed with Warnings")
    } else {
        (symbols::SUCCESS, colors::SUCCESS, "Indexing Successful")
    };

    println!(
        "{}{}╭─ {} {} ─────────────────────────────────────────╮{}",
        color, colors::BOLD, icon, title, colors::RESET
    );
    println!(
        "{}│{}                                                              {}│{}",
        color, colors::RESET, color, colors::RESET
    );

    // Stats
    println!(
        "{}│{}  {}Files Indexed:{}     {}{:>6}{}                                  {}│{}",
        color, colors::RESET, colors::MUTED, colors::RESET,
        colors::FG, result.files_indexed, colors::RESET, color, colors::RESET
    );
    println!(
        "{}│{}  {}Total Lines:{}       {}{:>6}{}                                  {}│{}",
        color, colors::RESET, colors::MUTED, colors::RESET,
        colors::FG, result.total_lines, colors::RESET, color, colors::RESET
    );
    println!(
        "{}│{}  {}Symbols Found:{}     {}{:>6}{}                                  {}│{}",
        color, colors::RESET, colors::MUTED, colors::RESET,
        colors::AI_ACCENT, result.symbols.total(), colors::RESET, color, colors::RESET
    );

    // Symbol breakdown
    if result.symbols.total() > 0 {
        println!(
            "{}│{}    {}󰊕 Functions: {} │ 󰆧 Types: {} │ 󰕘 Enums: {}{}        {}│{}",
            color, colors::RESET, colors::MUTED,
            result.symbols.functions, result.symbols.types, result.symbols.enums,
            colors::RESET, color, colors::RESET
        );
    }

    println!(
        "{}│{}  {}Time Elapsed:{}      {}{:.2}s{}                                  {}│{}",
        color, colors::RESET, colors::MUTED, colors::RESET,
        colors::FG, result.time_taken_ms as f64 / 1000.0, colors::RESET, color, colors::RESET
    );

    // Errors if any
    if result.files_skipped > 0 {
        println!(
            "{}│{}                                                              {}│{}",
            color, colors::RESET, color, colors::RESET
        );
        println!(
            "{}│{}  {}Skipped Files (Parse Error): {}{}                          {}│{}",
            color, colors::RESET, colors::ERROR, result.files_skipped, colors::RESET, color, colors::RESET
        );
    }

    println!(
        "{}│{}                                                              {}│{}",
        color, colors::RESET, color, colors::RESET
    );
    println!(
        "{}│{}  {}Ready for queries. Try: `nexus ask \"How does auth work?\"`{}  {}│{}",
        color, colors::RESET, colors::MUTED, colors::RESET, color, colors::RESET
    );
    println!(
        "{}╰──────────────────────────────────────────────────────────────╯{}",
        color, colors::RESET
    );
    println!();
}

/// Print a warning message
fn print_warning(message: &str) {
    println!(
        "\n{}  {} {}{}",
        colors::WARNING, symbols::WARNING, message, colors::RESET
    );
}

/// Truncate a path for display
fn truncate_path(path: &Path, max_len: usize) -> String {
    let s = path.display().to_string();
    if s.len() <= max_len {
        format!("{:<width$}", s, width = max_len)
    } else {
        format!("...{}", &s[s.len() - max_len + 3..])
    }
}

/// Result of indexing operation
#[derive(Debug)]
pub struct IndexResult {
    pub files_indexed: usize,
    pub files_skipped: usize,
    pub total_lines: usize,
    pub symbols: SymbolCounts,
    pub time_taken_ms: u64,
    pub errors: Vec<(PathBuf, String)>,
}

impl IndexResult {
    pub fn empty() -> Self {
        Self {
            files_indexed: 0,
            files_skipped: 0,
            total_lines: 0,
            symbols: SymbolCounts::default(),
            time_taken_ms: 0,
            errors: Vec::new(),
        }
    }
}

/// Legacy IndexStats for backward compatibility
#[derive(Debug)]
pub struct IndexStats {
    pub files_indexed: usize,
    pub tokens_processed: usize,
    pub time_taken_ms: u64,
}
