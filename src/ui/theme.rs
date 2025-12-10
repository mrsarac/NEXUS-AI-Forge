//! NEXUS theme for interactive UI components
//!
//! Defines colors and styling consistent with the design system.

#![allow(dead_code)]

use console::Style;
use dialoguer::theme::Theme;
use std::fmt;

/// NEXUS design system colors
pub mod colors {
    pub const PRIMARY: (u8, u8, u8) = (100, 181, 246);      // #64B5F6
    pub const SUCCESS: (u8, u8, u8) = (165, 214, 167);      // #A5D6A7
    pub const WARNING: (u8, u8, u8) = (255, 202, 40);       // #FFCA28
    pub const ERROR: (u8, u8, u8) = (239, 154, 154);        // #EF9A9A
    pub const AI_ACCENT: (u8, u8, u8) = (255, 202, 40);     // #FFCA28
    pub const MUTED: (u8, u8, u8) = (84, 110, 122);         // #546E7A
    pub const FG: (u8, u8, u8) = (212, 212, 215);           // #D4D4D7
    pub const BG_HIGHLIGHT: (u8, u8, u8) = (38, 50, 56);    // #263238
}

/// NEXUS branded theme for dialoguer
pub struct NexusTheme {
    /// Style for prompts/questions
    pub prompt_style: Style,
    /// Style for active/selected items
    pub active_style: Style,
    /// Style for inactive items
    pub inactive_style: Style,
    /// Style for descriptions
    pub description_style: Style,
    /// Style for hints
    pub hint_style: Style,
    /// Style for success messages
    pub success_style: Style,
    /// Style for error messages
    pub error_style: Style,
    /// Prefix for active items
    pub active_prefix: String,
    /// Prefix for inactive items
    pub inactive_prefix: String,
    /// Prefix for prompts
    pub prompt_prefix: String,
    /// Success prefix
    pub success_prefix: String,
}

impl Default for NexusTheme {
    fn default() -> Self {
        Self::new()
    }
}

impl NexusTheme {
    pub fn new() -> Self {
        // Use Color256 codes that approximate our design system colors
        // PRIMARY (#64B5F6) ≈ 117 (light blue)
        // SUCCESS (#A5D6A7) ≈ 114 (light green)
        // WARNING/AI_ACCENT (#FFCA28) ≈ 220 (gold)
        // ERROR (#EF9A9A) ≈ 210 (light red)
        // MUTED (#546E7A) ≈ 242 (gray)
        // FG (#D4D4D7) ≈ 252 (light gray)

        Self {
            prompt_style: Style::new().fg(console::Color::Color256(117)).bold(), // Bright blue
            active_style: Style::new().fg(console::Color::Color256(220)).bold(), // Gold (AI accent)
            inactive_style: Style::new().fg(console::Color::Color256(252)),      // Light gray
            description_style: Style::new().fg(console::Color::Color256(242)),   // Gray (muted)
            hint_style: Style::new().fg(console::Color::Color256(242)),          // Gray (muted)
            success_style: Style::new().fg(console::Color::Color256(114)),       // Light green
            error_style: Style::new().fg(console::Color::Color256(210)),         // Light red
            active_prefix: "❯ ".to_string(),
            inactive_prefix: "  ".to_string(),
            prompt_prefix: "󰌤 ".to_string(),  // AI icon
            success_prefix: "✓ ".to_string(),
        }
    }
}

impl Theme for NexusTheme {
    fn format_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        write!(
            f,
            "{}{} {}",
            self.prompt_style.apply_to(&self.prompt_prefix),
            self.prompt_style.apply_to(prompt),
            self.hint_style.apply_to("(↑↓ navigate, enter select)")
        )
    }

    fn format_error(&self, f: &mut dyn fmt::Write, err: &str) -> fmt::Result {
        write!(f, "{}", self.error_style.apply_to(err))
    }

    fn format_confirm_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<bool>,
    ) -> fmt::Result {
        write!(f, "{}{}", self.prompt_prefix, self.prompt_style.apply_to(prompt))?;
        match default {
            Some(true) => write!(f, " {}", self.hint_style.apply_to("[Y/n]")),
            Some(false) => write!(f, " {}", self.hint_style.apply_to("[y/N]")),
            None => write!(f, " {}", self.hint_style.apply_to("[y/n]")),
        }
    }

    fn format_confirm_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selection: Option<bool>,
    ) -> fmt::Result {
        write!(f, "{}{}", self.prompt_prefix, self.prompt_style.apply_to(prompt))?;
        match selection {
            Some(true) => write!(f, " {}", self.success_style.apply_to("Yes")),
            Some(false) => write!(f, " {}", self.error_style.apply_to("No")),
            None => Ok(()),
        }
    }

    fn format_input_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<&str>,
    ) -> fmt::Result {
        write!(f, "{}{}", self.prompt_prefix, self.prompt_style.apply_to(prompt))?;
        if let Some(default) = default {
            write!(f, " {}", self.hint_style.apply_to(format!("[{}]", default)))?;
        }
        write!(f, ": ")
    }

    fn format_input_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> fmt::Result {
        write!(
            f,
            "{}{}: {}",
            self.success_prefix,
            self.prompt_style.apply_to(prompt),
            self.success_style.apply_to(sel)
        )
    }

    fn format_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.format_prompt(f, prompt)
    }

    fn format_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> fmt::Result {
        write!(
            f,
            "{}{}: {}",
            self.success_style.apply_to(&self.success_prefix),
            self.prompt_style.apply_to(prompt),
            self.success_style.apply_to(sel)
        )
    }

    fn format_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        active: bool,
    ) -> fmt::Result {
        if active {
            write!(
                f,
                "{}{}",
                self.active_style.apply_to(&self.active_prefix),
                self.active_style.apply_to(text)
            )
        } else {
            write!(
                f,
                "{}{}",
                self.inactive_prefix,
                self.inactive_style.apply_to(text)
            )
        }
    }

    fn format_multi_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        write!(
            f,
            "{}{} {}",
            self.prompt_style.apply_to(&self.prompt_prefix),
            self.prompt_style.apply_to(prompt),
            self.hint_style.apply_to("(↑↓ navigate, space select, enter confirm)")
        )
    }

    fn format_multi_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        write!(
            f,
            "{}{}: {}",
            self.success_style.apply_to(&self.success_prefix),
            self.prompt_style.apply_to(prompt),
            self.success_style.apply_to(selections.join(", "))
        )
    }

    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        let checkbox = if checked { "◉" } else { "○" };

        if active {
            write!(
                f,
                "{}{} {}",
                self.active_style.apply_to(&self.active_prefix),
                self.active_style.apply_to(checkbox),
                self.active_style.apply_to(text)
            )
        } else {
            write!(
                f,
                "{}{} {}",
                self.inactive_prefix,
                if checked {
                    self.success_style.apply_to(checkbox).to_string()
                } else {
                    self.inactive_style.apply_to(checkbox).to_string()
                },
                self.inactive_style.apply_to(text)
            )
        }
    }

    fn format_sort_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.format_prompt(f, prompt)
    }

    fn format_sort_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        self.format_multi_select_prompt_selection(f, prompt, selections)
    }

    fn format_sort_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        picked: bool,
        active: bool,
    ) -> fmt::Result {
        self.format_multi_select_prompt_item(f, text, picked, active)
    }
}
