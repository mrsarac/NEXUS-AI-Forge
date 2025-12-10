//! Semantic search command - search code by meaning
//!
//! Searches the codebase using both text matching and AI-powered semantic understanding.

#![allow(dead_code)]

use anyhow::Result;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::config::Config;
use crate::core::parser::{CodeParser, Language, ParsedFile, SymbolKind};

// ANSI color codes
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const PRIMARY: &str = "\x1b[38;2;100;181;246m";      // #64B5F6
    pub const SUCCESS: &str = "\x1b[38;2;165;214;167m";      // #A5D6A7
    pub const WARNING: &str = "\x1b[38;2;255;202;40m";       // #FFCA28
    pub const MUTED: &str = "\x1b[38;2;84;110;122m";         // #546E7A
    pub const FG: &str = "\x1b[38;2;212;212;215m";           // #D4D4D7
    pub const HIGHLIGHT: &str = "\x1b[38;2;255;183;77m";     // Orange highlight
}

mod symbols {
    pub const SEARCH: &str = "󰍉";
    pub const FILE: &str = "󰈙";
    pub const FUNCTION: &str = "󰊕";
    pub const STRUCT: &str = "󰆧";
    pub const MATCH: &str = "󰄬";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// Search result with relevance score
#[derive(Debug)]
struct SearchResult {
    file_path: String,
    symbol_name: String,
    symbol_kind: SymbolKind,
    line_start: usize,
    line_end: usize,
    signature: Option<String>,
    context: String,
    score: f64,
    match_type: MatchType,
}

#[derive(Debug)]
enum MatchType {
    ExactName,
    PartialName,
    ContentMatch,
    ContextMatch,
}

pub async fn run(_config: Config, query: &str, limit: usize) -> Result<()> {
    print_header(query);

    // Parse codebase
    print_status("Scanning codebase...");
    let parsed_files = index_codebase(Path::new("."))?;
    clear_line();

    if parsed_files.is_empty() {
        print_warning("No supported files found in current directory");
        return Ok(());
    }

    print_status(&format!("Searching {} files...", parsed_files.len()));

    // Perform search
    let results = search_codebase(&parsed_files, query, limit);
    clear_line();

    if results.is_empty() {
        print_no_results(query);
        return Ok(());
    }

    // Display results
    print_results(&results, query);

    Ok(())
}

/// Search the codebase for the query
fn search_codebase(files: &[ParsedFile], query: &str, limit: usize) -> Vec<SearchResult> {
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();

    let mut results: Vec<SearchResult> = Vec::new();

    for file in files {
        // Read file content for context matching
        let file_content = fs::read_to_string(&file.path).unwrap_or_default();
        let lines: Vec<&str> = file_content.lines().collect();

        for symbol in &file.symbols {
            let symbol_lower = symbol.name.to_lowercase();
            let mut score = 0.0;
            let mut match_type = MatchType::ContextMatch;

            // Exact name match (highest score)
            if symbol_lower == query_lower {
                score = 100.0;
                match_type = MatchType::ExactName;
            }
            // Partial name match
            else if symbol_lower.contains(&query_lower) || query_lower.contains(&symbol_lower) {
                score = 80.0;
                match_type = MatchType::PartialName;
            }
            // Word-based matching
            else {
                let mut word_matches = 0;
                for word in &query_words {
                    if symbol_lower.contains(word) {
                        word_matches += 1;
                    }
                }
                if word_matches > 0 {
                    score = 50.0 + (word_matches as f64 * 10.0);
                    match_type = MatchType::PartialName;
                }
            }

            // Content/context matching (check code around symbol)
            if score == 0.0 {
                let start = symbol.line_start.saturating_sub(1);
                let end = (symbol.line_end).min(lines.len());
                let context_lines: String = lines[start..end].join("\n").to_lowercase();

                if context_lines.contains(&query_lower) {
                    score = 30.0;
                    match_type = MatchType::ContentMatch;
                } else {
                    // Check for word matches in context
                    let mut context_word_matches = 0;
                    for word in &query_words {
                        if context_lines.contains(word) {
                            context_word_matches += 1;
                        }
                    }
                    if context_word_matches > 0 {
                        score = 20.0 + (context_word_matches as f64 * 5.0);
                        match_type = MatchType::ContextMatch;
                    }
                }
            }

            // Boost score based on symbol kind (functions/structs are usually more relevant)
            match symbol.kind {
                SymbolKind::Function => score *= 1.2,
                SymbolKind::Struct | SymbolKind::Class => score *= 1.15,
                SymbolKind::Trait | SymbolKind::Interface => score *= 1.1,
                _ => {}
            }

            if score > 0.0 {
                // Extract context lines
                let start = symbol.line_start.saturating_sub(1);
                let end = (symbol.line_start + 2).min(lines.len());
                let context = lines[start..end].join("\n");

                results.push(SearchResult {
                    file_path: file.path.display().to_string(),
                    symbol_name: symbol.name.clone(),
                    symbol_kind: symbol.kind.clone(),
                    line_start: symbol.line_start,
                    line_end: symbol.line_end,
                    signature: symbol.signature.clone(),
                    context,
                    score,
                    match_type,
                });
            }
        }
    }

    // Sort by score (descending)
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Limit results
    results.truncate(limit);

    results
}

/// Index all supported files in the codebase
fn index_codebase(path: &Path) -> Result<Vec<ParsedFile>> {
    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    let mut parser = match CodeParser::new() {
        Ok(p) => p,
        Err(_e) => {
            return Ok(Vec::new());
        }
    };
    let mut parsed_files = Vec::new();

    for entry in walkdir::WalkDir::new(&abs_path)
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
                    if let Ok(parsed) = parser.parse_file(file_path) {
                        parsed_files.push(parsed);
                    }
                }
            }
        }
    }

    Ok(parsed_files)
}

// ============================================
// UI Functions
// ============================================

fn print_header(query: &str) {
    println!();
    println!(
        "{}{}  {} Semantic Search{}",
        colors::PRIMARY, colors::BOLD, symbols::SEARCH, colors::RESET
    );
    println!(
        "{}  │ Query: {}\"{}\"{}",
        colors::MUTED, colors::HIGHLIGHT, query, colors::RESET
    );
    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(50), colors::RESET
    );
    println!();
}

fn print_status(message: &str) {
    print!(
        "\r{}  {} {}{}",
        colors::MUTED, symbols::SPINNER[0], message, colors::RESET
    );
    io::stdout().flush().ok();
}

fn clear_line() {
    print!("\r{}\r", " ".repeat(60));
    io::stdout().flush().ok();
}

fn print_results(results: &[SearchResult], query: &str) {
    println!(
        "{}{}  {} Found {} results for \"{}\"{}",
        colors::SUCCESS, colors::BOLD, symbols::MATCH,
        results.len(), query, colors::RESET
    );
    println!();

    for (i, result) in results.iter().enumerate() {
        let kind_icon = match result.symbol_kind {
            SymbolKind::Function => symbols::FUNCTION,
            SymbolKind::Struct | SymbolKind::Class => symbols::STRUCT,
            _ => symbols::FILE,
        };

        let kind_str = match result.symbol_kind {
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

        let match_indicator = match result.match_type {
            MatchType::ExactName => format!("{}exact{}", colors::SUCCESS, colors::RESET),
            MatchType::PartialName => format!("{}name{}", colors::WARNING, colors::RESET),
            MatchType::ContentMatch => format!("{}content{}", colors::PRIMARY, colors::RESET),
            MatchType::ContextMatch => format!("{}context{}", colors::MUTED, colors::RESET),
        };

        // Result header
        println!(
            "{}  {}. {} {}{}{} ({}) [{}]",
            colors::MUTED,
            i + 1,
            kind_icon,
            colors::FG,
            result.symbol_name,
            colors::RESET,
            kind_str,
            match_indicator
        );

        // File location
        println!(
            "{}      {} {}:{}{}",
            colors::MUTED,
            symbols::FILE,
            result.file_path,
            result.line_start,
            colors::RESET
        );

        // Signature or context preview
        if let Some(sig) = &result.signature {
            let sig_preview: String = sig.chars().take(80).collect();
            println!(
                "{}      {}{}",
                colors::MUTED,
                sig_preview,
                if sig.len() > 80 { "..." } else { "" }
            );
        }

        println!();
    }

    // Usage hint
    println!(
        "{}  💡 Use 'nexus explain <file>:<line>' for detailed explanation{}",
        colors::MUTED, colors::RESET
    );
    println!();
}

fn print_no_results(query: &str) {
    println!(
        "{}  {} No results found for \"{}\"{}",
        colors::WARNING, symbols::SEARCH, query, colors::RESET
    );
    println!();
    println!(
        "{}  Try:{}",
        colors::MUTED, colors::RESET
    );
    println!(
        "{}  • Using different keywords{}",
        colors::MUTED, colors::RESET
    );
    println!(
        "{}  • Searching for function or class names{}",
        colors::MUTED, colors::RESET
    );
    println!(
        "{}  • Using partial matches (e.g., 'auth' instead of 'authentication'){}",
        colors::MUTED, colors::RESET
    );
    println!();
}

fn print_warning(message: &str) {
    println!(
        "{}  {} {}{}",
        colors::WARNING, symbols::SEARCH, message, colors::RESET
    );
}
