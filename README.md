# NEXUS AI Forge

> **The ultimate AI-augmented developer tool.**
> Rust-powered. Free tier included. Multi-model support.

```
███╗   ██╗███████╗██╗  ██╗██╗   ██╗███████╗
████╗  ██║██╔════╝╚██╗██╔╝██║   ██║██╔════╝
██╔██╗ ██║█████╗   ╚███╔╝ ██║   ██║███████╗
██║╚██╗██║██╔══╝   ██╔██╗ ██║   ██║╚════██║
██║ ╚████║███████╗██╔╝ ██╗╚██████╔╝███████║
╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝
         AI FORGE - Developer's Companion
```

## Quick Start

### Installation (macOS ARM64 - M1/M2/M3)

```bash
# Download and install (30 seconds)
curl -L https://github.com/mrsarac/NEXUS-AI-Forge/releases/latest/download/nexus-darwin-arm64 -o nexus
chmod +x nexus
sudo mv nexus /usr/local/bin/
nexus --version
```

### Installation (Other Platforms - Build from Source)

```bash
# Requires Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone https://github.com/mrsarac/NEXUS-AI-Forge.git
cd NEXUS-AI-Forge
cargo build --release
sudo cp target/release/nexus /usr/local/bin/
```

### First Run (No API Key Needed!)

```bash
# Generate code using free tier (Gemini-powered)
nexus generate "A function that checks if a number is prime" -o prime.py

# Interactive setup wizard
nexus init
```

## Features

### Working Now

| Command | Description | Example |
|---------|-------------|---------|
| `generate` | AI code generation | `nexus generate "REST API client" -o client.rs` |
| `chat` | Interactive AI conversation | `nexus chat` |
| `ask` | Quick questions about code | `nexus ask "What does this function do?"` |
| `explain` | Code explanation | `nexus explain src/main.rs` |
| `review` | Security & quality review | `nexus review src/` |
| `fix` | AI-powered bug fixing | `nexus fix src/buggy.rs -e "error message"` |
| `test` | Generate unit tests | `nexus test src/lib.rs -o tests/lib_test.rs` |
| `commit` | Smart commit messages | `nexus commit --execute` |
| `doc` | Generate documentation | `nexus doc src/main.rs -o docs/API.md` |
| `refactor` | Refactor code | `nexus refactor src/ -d "improve naming"` |
| `search` | Semantic code search | `nexus search "error handling"` |
| `index` | Index codebase for search | `nexus index .` |
| `diff` | AI-powered git diff analysis | `nexus diff --staged` |
| `convert` | Convert code between languages | `nexus convert main.py --to rust` |
| `optimize` | Performance optimization tips | `nexus optimize src/lib.rs --focus time` |
| `init` | Interactive setup wizard | `nexus init` |
| `update` | Self-update to latest version | `nexus update` |

### AI Providers

| Provider | API Key Required | Cost | Best For |
|----------|------------------|------|----------|
| **NEXUS Free** | No | Free | Quick tasks, trying out |
| **Claude** | Yes | Pay-per-use | Complex reasoning, reviews |
| **Ollama** | No | Free | Local/offline, privacy-focused |

```bash
# Free tier (default, no setup needed)
nexus generate "fibonacci function" -l rust

# Power user mode (set your own API key)
export ANTHROPIC_API_KEY="sk-ant-xxx"
nexus generate "fibonacci function" -l rust
```

## Commands

### `nexus generate` - AI Code Generation

Generate production-ready code from natural language descriptions.

```bash
# Basic usage
nexus generate "A function that sorts a list using quicksort"

# Specify output file (auto-detects language from extension)
nexus generate "HTTP server with /health endpoint" -o server.go

# Specify language explicitly
nexus generate "Fibonacci sequence generator" -l python

# Combine both
nexus generate "Binary search tree implementation" -l rust -o bst.rs
```

**Supported Languages:**
- Rust, Python, JavaScript, TypeScript, Go
- Java, C#, Ruby, Swift, Kotlin

### `nexus chat` - Interactive AI Session

Start a conversation with AI about your code.

```bash
nexus chat
# or with initial prompt
nexus chat "Help me optimize this algorithm"
```

### `nexus ask` - Quick Questions

Get quick answers about your codebase.

```bash
nexus ask "What design patterns are used in this project?"
nexus ask "Where is user authentication handled?"
```

### `nexus explain` - Code Explanation

Get detailed explanations of code.

```bash
nexus explain src/main.rs
nexus explain src/utils.py --depth expert
```

Depth options: `brief`, `detailed`, `expert`

### `nexus review` - Code Review

AI-powered security and quality review.

```bash
nexus review src/
nexus review src/auth.rs --focus security,performance
```

Focus areas: `security`, `performance`, `style`, `bugs`

### `nexus index` - Codebase Indexing

Index your codebase for faster searches.

```bash
nexus index .
nexus index ./src --force  # Force re-index
```

### `nexus search` - Semantic Search

Search code by meaning, not just text.

```bash
nexus search "error handling"
nexus search "database connection" --limit 20
```

### `nexus init` - Setup Wizard

Interactive setup for first-time users.

```bash
nexus init
```

### `nexus update` - Self-Update

Update NEXUS to the latest version automatically.

```bash
# Check for updates
nexus update --check

# Update to latest version
nexus update

# Force reinstall (useful for corrupted installs)
nexus update --force
```

## Configuration

### Environment Variables

```bash
# Optional: Use your own Claude API key for unlimited access
export ANTHROPIC_API_KEY="sk-ant-api03-xxx"

# Optional: Custom proxy URL (advanced)
export NEXUS_PROXY_URL="https://your-proxy.example.com"
```

### Config File

```toml
# ~/.config/nexus/config.toml

[general]
theme = "dark"

[ai]
default_provider = "claude"  # or "proxy" for free tier

[ai.providers.claude]
api_key_env = "ANTHROPIC_API_KEY"
model = "claude-sonnet-4-20250514"
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      NEXUS AI Forge                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   User Commands                                             │
│   ├── generate    → AI code generation                      │
│   ├── chat        → Interactive conversation                │
│   ├── ask         → Quick Q&A                               │
│   ├── explain     → Code explanation                        │
│   ├── review      → Security & quality review               │
│   ├── index       → Codebase indexing                       │
│   └── search      → Semantic search                         │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │              AI Router (Smart Dispatch)              │   │
│   │                                                      │   │
│   │   API Key Set?                                       │   │
│   │   ├── Yes → Claude API (unlimited, paid)            │   │
│   │   └── No  → NEXUS Proxy (free tier, Gemini)         │   │
│   │                                                      │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│   Core Engine (Rust)                                        │
│   ├── tree-sitter  → Fast AST parsing                       │
│   ├── tokio        → Async runtime                          │
│   └── reqwest      → HTTP client                            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Security

NEXUS takes security seriously:

- **Free tier API keys** are stored server-side only - never exposed to clients
- **Your code** is processed and immediately discarded - no storage
- **HTTPS only** for all API communications
- **No telemetry** unless explicitly enabled

## Tech Stack

| Component | Technology |
|-----------|------------|
| Core | Rust |
| Async | Tokio |
| CLI | Clap |
| HTTP | Reqwest |
| Parser | Tree-sitter |
| UI | Dialoguer + Custom theme |
| Free AI | Gemini 2.0 Flash |
| Premium AI | Claude (Anthropic) |

## Building from Source

### Prerequisites

- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Git

### Build

```bash
git clone https://github.com/mrsarac/NEXUS-AI-Forge.git
cd NEXUS-AI-Forge

# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run directly
cargo run -- generate "hello world function" -l python

# Install globally
cargo install --path .
```

### Binary Size

Release build: ~9 MB (ARM64)

## Examples

### Generate a REST API

```bash
nexus generate "Express.js REST API with CRUD endpoints for users" -o api.js
```

### Review Security

```bash
nexus review src/auth/ --focus security
```

### Explain Complex Code

```bash
nexus explain src/core/engine.rs --depth expert
```

### Interactive Coding Session

```bash
nexus chat "I'm building a CLI tool in Rust. Help me design the argument parser."
```

## Roadmap

- [x] Core CLI framework
- [x] AI code generation
- [x] Free tier (Gemini)
- [x] Claude integration
- [x] Interactive forms (Claude Code style)
- [x] Code review
- [x] Code explanation
- [x] Self-update command
- [x] Bug fixing (fix command)
- [x] Test generation (test command)
- [x] Smart commits (commit command)
- [x] Documentation generation (doc command)
- [x] Git diff analysis (diff command)
- [x] Code conversion (convert command)
- [x] Performance optimization (optimize command)
- [x] Local AI support (Ollama)
- [ ] VS Code extension
- [ ] Plugin system (WASM)
- [ ] Team features

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
  <strong>Built with Rust. Powered by AI. Free to start.</strong>
  <br>
  <em>By <a href="https://github.com/mrsarac">Mustafa Saraç</a></em>
</p>
