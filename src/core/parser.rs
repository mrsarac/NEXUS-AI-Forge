//! Code parsing using tree-sitter
//!
//! Extracts AST structure from source files for context-aware AI assistance.

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::path::Path;
use std::fs;
use tree_sitter::{Parser, Tree, Node};

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Language::Rust,
            "py" | "pyw" => Language::Python,
            "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "ts" | "tsx" | "mts" | "cts" => Language::TypeScript,
            _ => Language::Unknown,
        }
    }

    pub fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|e| e.to_str())
            .map(Self::from_extension)
            .unwrap_or(Language::Unknown)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Code parser using tree-sitter
pub struct CodeParser {
    rust_parser: Parser,
    python_parser: Parser,
    javascript_parser: Parser,
    typescript_parser: Parser,
}

impl CodeParser {
    /// Create a new code parser with all supported languages
    pub fn new() -> Result<Self> {
        let mut rust_parser = Parser::new();
        rust_parser.set_language(tree_sitter_rust::language())
            .context("Failed to set Rust language")?;

        let mut python_parser = Parser::new();
        python_parser.set_language(tree_sitter_python::language())
            .context("Failed to set Python language")?;

        let mut javascript_parser = Parser::new();
        javascript_parser.set_language(tree_sitter_javascript::language())
            .context("Failed to set JavaScript language")?;

        let mut typescript_parser = Parser::new();
        typescript_parser.set_language(tree_sitter_typescript::language_typescript())
            .context("Failed to set TypeScript language")?;

        Ok(Self {
            rust_parser,
            python_parser,
            javascript_parser,
            typescript_parser,
        })
    }

    /// Parse a file and extract its structure
    pub fn parse_file(&mut self, path: &Path) -> Result<ParsedFile> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let language = Language::from_path(path);

        let tree = self.parse_content(&content, language)?;

        let symbols = self.extract_symbols(&tree, &content, language);

        Ok(ParsedFile {
            path: path.to_path_buf(),
            language,
            content,
            symbols,
            line_count: tree.root_node().end_position().row + 1,
        })
    }

    /// Parse content string with the appropriate language parser
    fn parse_content(&mut self, content: &str, language: Language) -> Result<Tree> {
        let parser = match language {
            Language::Rust => &mut self.rust_parser,
            Language::Python => &mut self.python_parser,
            Language::JavaScript => &mut self.javascript_parser,
            Language::TypeScript => &mut self.typescript_parser,
            Language::Unknown => {
                anyhow::bail!("Unsupported language");
            }
        };

        parser.parse(content, None)
            .context("Tree-sitter parsing failed")
    }

    /// Extract symbols (functions, structs, classes, etc.) from AST
    fn extract_symbols(&self, tree: &Tree, content: &str, language: Language) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let root = tree.root_node();

        self.walk_tree(root, content, language, &mut symbols, 0);

        symbols
    }

    /// Recursively walk the AST and extract symbols
    fn walk_tree(
        &self,
        node: Node,
        content: &str,
        language: Language,
        symbols: &mut Vec<Symbol>,
        depth: usize
    ) {
        let kind = node.kind();

        // Extract symbols based on language and node type
        match language {
            Language::Rust => self.extract_rust_symbol(node, content, kind, symbols, depth),
            Language::Python => self.extract_python_symbol(node, content, kind, symbols, depth),
            Language::JavaScript | Language::TypeScript => {
                self.extract_js_symbol(node, content, kind, symbols, depth)
            }
            Language::Unknown => {}
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_tree(child, content, language, symbols, depth + 1);
        }
    }

    /// Extract Rust-specific symbols
    fn extract_rust_symbol(
        &self,
        node: Node,
        content: &str,
        kind: &str,
        symbols: &mut Vec<Symbol>,
        _depth: usize
    ) {
        match kind {
            "function_item" | "function_signature_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, content);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Function,
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: Some(self.get_signature(node, content)),
                    });
                }
            }
            "struct_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, content);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Struct,
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: None,
                    });
                }
            }
            "enum_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, content);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Enum,
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: None,
                    });
                }
            }
            "impl_item" => {
                if let Some(type_node) = node.child_by_field_name("type") {
                    let name = self.node_text(type_node, content);
                    symbols.push(Symbol {
                        name: format!("impl {}", name),
                        kind: SymbolKind::Impl,
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: None,
                    });
                }
            }
            "trait_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, content);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Trait,
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: None,
                    });
                }
            }
            "mod_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, content);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Module,
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: None,
                    });
                }
            }
            "const_item" | "static_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, content);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Constant,
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: None,
                    });
                }
            }
            _ => {}
        }
    }

    /// Extract Python-specific symbols
    fn extract_python_symbol(
        &self,
        node: Node,
        content: &str,
        kind: &str,
        symbols: &mut Vec<Symbol>,
        _depth: usize
    ) {
        match kind {
            "function_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, content);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Function,
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: Some(self.get_signature(node, content)),
                    });
                }
            }
            "class_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, content);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Class,
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: None,
                    });
                }
            }
            _ => {}
        }
    }

    /// Extract JavaScript/TypeScript-specific symbols
    fn extract_js_symbol(
        &self,
        node: Node,
        content: &str,
        kind: &str,
        symbols: &mut Vec<Symbol>,
        _depth: usize
    ) {
        match kind {
            "function_declaration" | "method_definition" | "arrow_function" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, content);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Function,
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: Some(self.get_signature(node, content)),
                    });
                }
            }
            "class_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, content);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Class,
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: None,
                    });
                }
            }
            "interface_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, content);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Interface,
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: None,
                    });
                }
            }
            "type_alias_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, content);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::TypeAlias,
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: None,
                    });
                }
            }
            _ => {}
        }
    }

    /// Get text content of a node
    fn node_text(&self, node: Node, content: &str) -> String {
        content[node.byte_range()].to_string()
    }

    /// Get signature (first line) of a node
    fn get_signature(&self, node: Node, content: &str) -> String {
        let text = &content[node.byte_range()];
        text.lines().next().unwrap_or("").to_string()
    }
}

/// Parsed file with extracted symbols
#[derive(Debug)]
pub struct ParsedFile {
    pub path: std::path::PathBuf,
    pub language: Language,
    pub content: String,
    pub symbols: Vec<Symbol>,
    pub line_count: usize,
}

impl ParsedFile {
    /// Get count of each symbol type
    pub fn symbol_counts(&self) -> SymbolCounts {
        let mut counts = SymbolCounts::default();
        for symbol in &self.symbols {
            match symbol.kind {
                SymbolKind::Function => counts.functions += 1,
                SymbolKind::Struct | SymbolKind::Class => counts.types += 1,
                SymbolKind::Enum => counts.enums += 1,
                SymbolKind::Trait | SymbolKind::Interface => counts.traits += 1,
                SymbolKind::Module => counts.modules += 1,
                SymbolKind::Constant => counts.constants += 1,
                SymbolKind::Impl => counts.impls += 1,
                SymbolKind::TypeAlias => counts.type_aliases += 1,
            }
        }
        counts
    }
}

/// Symbol extracted from code
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line_start: usize,
    pub line_end: usize,
    pub signature: Option<String>,
}

/// Types of symbols
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Struct,
    Class,
    Enum,
    Trait,
    Interface,
    Module,
    Constant,
    Impl,
    TypeAlias,
}

impl SymbolKind {
    pub fn icon(&self) -> &'static str {
        match self {
            SymbolKind::Function => "󰊕",
            SymbolKind::Struct => "󰆧",
            SymbolKind::Class => "󰠱",
            SymbolKind::Enum => "󰕘",
            SymbolKind::Trait | SymbolKind::Interface => "󰜰",
            SymbolKind::Module => "󰏗",
            SymbolKind::Constant => "󰏿",
            SymbolKind::Impl => "󰡱",
            SymbolKind::TypeAlias => "󰊄",
        }
    }
}

/// Counts of different symbol types
#[derive(Debug, Default)]
pub struct SymbolCounts {
    pub functions: usize,
    pub types: usize,
    pub enums: usize,
    pub traits: usize,
    pub modules: usize,
    pub constants: usize,
    pub impls: usize,
    pub type_aliases: usize,
}

impl SymbolCounts {
    pub fn total(&self) -> usize {
        self.functions + self.types + self.enums + self.traits +
        self.modules + self.constants + self.impls + self.type_aliases
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("tsx"), Language::TypeScript);
        assert_eq!(Language::from_extension("unknown"), Language::Unknown);
    }

    #[test]
    fn test_parse_rust_code() {
        let mut parser = CodeParser::new().unwrap();
        let code = r#"
fn main() {
    println!("Hello");
}

struct User {
    name: String,
}

impl User {
    fn new(name: String) -> Self {
        Self { name }
    }
}
"#;
        // Write to temp file and parse
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        std::fs::write(&file_path, code).unwrap();

        let parsed = parser.parse_file(&file_path).unwrap();

        assert_eq!(parsed.language, Language::Rust);
        assert!(parsed.symbols.iter().any(|s| s.name == "main"));
        assert!(parsed.symbols.iter().any(|s| s.name == "User"));
    }
}
