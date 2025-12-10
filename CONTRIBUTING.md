# Contributing to NEXUS AI Forge

Thank you for your interest in contributing to NEXUS AI Forge!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/NEXUS-AI-Forge.git`
3. Create a branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Run tests: `cargo test`
6. Build: `cargo build --release`
7. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Git

### Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with verbose output
cargo run -- --verbose generate "hello world"
```

## Code Style

- Follow Rust conventions (rustfmt)
- Use meaningful variable names
- Add documentation for public APIs
- Keep functions small and focused

## Pull Request Guidelines

1. One feature per PR
2. Include tests for new features
3. Update documentation if needed
4. Ensure CI passes

## Reporting Issues

- Use GitHub Issues
- Include reproduction steps
- Specify your OS and Rust version

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
