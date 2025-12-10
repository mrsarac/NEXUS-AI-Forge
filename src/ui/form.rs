//! Claude Code style interactive forms for NEXUS
//!
//! Provides a simple API for creating interactive selection forms
//! with descriptions, similar to Claude Code's question format.

#![allow(dead_code)]

use anyhow::Result;
use dialoguer::{Select, MultiSelect, Confirm, Input};
use console::Term;

use super::theme::NexusTheme;

// ANSI codes for custom formatting
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const ITALIC: &str = "\x1b[3m";
    pub const PRIMARY: &str = "\x1b[38;2;100;181;246m";
    pub const AI_ACCENT: &str = "\x1b[38;2;255;202;40m";
    pub const MUTED: &str = "\x1b[38;2;84;110;122m";
    pub const FG: &str = "\x1b[38;2;212;212;215m";
    pub const SUCCESS: &str = "\x1b[38;2;165;214;167m";
}

/// A single option in a form selection
#[derive(Debug, Clone)]
pub struct FormOption {
    /// Short label shown in the selection list
    pub label: String,
    /// Longer description shown below the label
    pub description: String,
    /// Whether this is the recommended option
    pub recommended: bool,
}

impl FormOption {
    /// Create a new form option
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: description.into(),
            recommended: false,
        }
    }

    /// Mark this option as recommended
    pub fn recommended(mut self) -> Self {
        self.recommended = true;
        self
    }

    /// Format for display in selection list
    fn display(&self) -> String {
        if self.recommended {
            format!("{} (Recommended)", self.label)
        } else {
            self.label.clone()
        }
    }
}

/// Result from a form interaction
#[derive(Debug)]
pub enum FormResult {
    /// Single selection result
    Single(usize),
    /// Multiple selection result
    Multiple(Vec<usize>),
    /// Confirmation result
    Confirmed(bool),
    /// Text input result
    Text(String),
    /// User cancelled
    Cancelled,
}

/// Claude Code style interactive form builder
pub struct NexusForm {
    theme: NexusTheme,
}

impl Default for NexusForm {
    fn default() -> Self {
        Self::new()
    }
}

impl NexusForm {
    /// Create a new form with NEXUS theme
    pub fn new() -> Self {
        Self {
            theme: NexusTheme::new(),
        }
    }

    /// Display a single-select question with options and descriptions
    ///
    /// # Example
    /// ```
    /// let options = vec![
    ///     FormOption::new("Option A", "Description for A").recommended(),
    ///     FormOption::new("Option B", "Description for B"),
    /// ];
    /// let result = NexusForm::new().select("Choose an option:", &options)?;
    /// ```
    pub fn select(&self, question: &str, options: &[FormOption]) -> Result<FormResult> {
        // Print header
        self.print_question_header(question);

        // Print options with descriptions
        self.print_options_preview(options);

        // Create selection items
        let items: Vec<String> = options.iter().map(|o| o.display()).collect();

        // Find default (recommended) option
        let default = options.iter().position(|o| o.recommended).unwrap_or(0);

        let selection = Select::with_theme(&self.theme)
            .items(&items)
            .default(default)
            .interact_on_opt(&Term::stderr())?;

        match selection {
            Some(idx) => {
                self.print_selection_result(&options[idx].label);
                Ok(FormResult::Single(idx))
            }
            None => Ok(FormResult::Cancelled),
        }
    }

    /// Display a multi-select question
    pub fn multi_select(&self, question: &str, options: &[FormOption]) -> Result<FormResult> {
        self.print_question_header(question);
        self.print_options_preview(options);

        let items: Vec<String> = options.iter().map(|o| o.display()).collect();

        let selections = MultiSelect::with_theme(&self.theme)
            .items(&items)
            .interact_on_opt(&Term::stderr())?;

        match selections {
            Some(idxs) => {
                let labels: Vec<&str> = idxs.iter().map(|&i| options[i].label.as_str()).collect();
                self.print_multi_selection_result(&labels);
                Ok(FormResult::Multiple(idxs))
            }
            None => Ok(FormResult::Cancelled),
        }
    }

    /// Display a yes/no confirmation
    pub fn confirm(&self, question: &str, default: bool) -> Result<FormResult> {
        println!();

        let result = Confirm::with_theme(&self.theme)
            .with_prompt(question)
            .default(default)
            .interact_on_opt(&Term::stderr())?;

        match result {
            Some(confirmed) => Ok(FormResult::Confirmed(confirmed)),
            None => Ok(FormResult::Cancelled),
        }
    }

    /// Display a text input prompt
    pub fn input(&self, question: &str, default: Option<&str>) -> Result<FormResult> {
        println!();

        let mut input = Input::with_theme(&self.theme)
            .with_prompt(question);

        if let Some(def) = default {
            input = input.default(def.to_string());
        }

        let result: Result<String, _> = input.interact_text();

        match result {
            Ok(text) => Ok(FormResult::Text(text)),
            Err(_) => Ok(FormResult::Cancelled),
        }
    }

    /// Print question header
    fn print_question_header(&self, question: &str) {
        println!();
        println!(
            "{}{}󰌤 {}{}",
            colors::PRIMARY, colors::BOLD, question, colors::RESET
        );
        println!(
            "{}  ╭{}─{}",
            colors::MUTED, "─".repeat(50), colors::RESET
        );
    }

    /// Print options with their descriptions
    fn print_options_preview(&self, options: &[FormOption]) {
        for (i, opt) in options.iter().enumerate() {
            let prefix = if i == options.len() - 1 { "╰" } else { "├" };
            let rec_badge = if opt.recommended {
                format!(" {}★ Recommended{}", colors::AI_ACCENT, colors::RESET)
            } else {
                String::new()
            };

            println!(
                "{}  {} {}{}{}{}{}",
                colors::MUTED,
                prefix,
                colors::FG,
                colors::BOLD,
                opt.label,
                rec_badge,
                colors::RESET
            );
            println!(
                "{}  {}   {}{}{}",
                colors::MUTED,
                if i == options.len() - 1 { " " } else { "│" },
                colors::DIM,
                opt.description,
                colors::RESET
            );
        }
        println!();
    }

    /// Print result after selection
    fn print_selection_result(&self, label: &str) {
        println!(
            "\n{}  ✓ Selected: {}{}{}",
            colors::SUCCESS, colors::BOLD, label, colors::RESET
        );
    }

    /// Print result after multi-selection
    fn print_multi_selection_result(&self, labels: &[&str]) {
        println!(
            "\n{}  ✓ Selected: {}{}{}",
            colors::SUCCESS,
            colors::BOLD,
            labels.join(", "),
            colors::RESET
        );
    }
}

/// Quick helper functions for common form patterns
impl NexusForm {
    /// Ask a simple A/B/C question
    pub fn ask_choice(
        question: &str,
        choices: &[(&str, &str)],  // (label, description)
        recommended: Option<usize>,
    ) -> Result<usize> {
        let options: Vec<FormOption> = choices
            .iter()
            .enumerate()
            .map(|(i, (label, desc))| {
                let opt = FormOption::new(*label, *desc);
                if Some(i) == recommended {
                    opt.recommended()
                } else {
                    opt
                }
            })
            .collect();

        let form = NexusForm::new();
        match form.select(question, &options)? {
            FormResult::Single(idx) => Ok(idx),
            _ => anyhow::bail!("Selection cancelled"),
        }
    }

    /// Ask for yes/no confirmation
    pub fn ask_confirm(question: &str, default: bool) -> Result<bool> {
        let form = NexusForm::new();
        match form.confirm(question, default)? {
            FormResult::Confirmed(yes) => Ok(yes),
            _ => anyhow::bail!("Confirmation cancelled"),
        }
    }

    /// Ask for text input
    pub fn ask_input(question: &str, default: Option<&str>) -> Result<String> {
        let form = NexusForm::new();
        match form.input(question, default)? {
            FormResult::Text(text) => Ok(text),
            _ => anyhow::bail!("Input cancelled"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_option_creation() {
        let opt = FormOption::new("Test", "Description");
        assert_eq!(opt.label, "Test");
        assert_eq!(opt.description, "Description");
        assert!(!opt.recommended);
    }

    #[test]
    fn test_form_option_recommended() {
        let opt = FormOption::new("Test", "Description").recommended();
        assert!(opt.recommended);
        assert_eq!(opt.display(), "Test (Recommended)");
    }
}
