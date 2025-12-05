# Contributing to P2P SuperApp

Thank you for your interest in contributing to P2P SuperApp! This document outlines the guidelines and process for contributing.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Important Legal Notice](#-important-legal-notice)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)
- [Testing](#testing)
- [Architecture Guide](#architecture-guide)

---

## Code of Conduct

- Be respectful and inclusive in all interactions
- Provide constructive feedback
- Focus on the code, not the person
- Help maintain a welcoming community

---

## ⚠️ Important Legal Notice

Before contributing, please understand the following:

1. **Ownership Transfer**: By submitting a contribution (pull request, code, documentation, etc.), you **irrevocably transfer all rights, title, and interest** in your contribution to the project owner.

2. **No Forking**: You may NOT fork this repository to create your own project.

3. **No Code Reuse**: You may NOT copy, extract, or reuse any code snippets from this project in other projects.

4. **Contribution License**: Your contributions will be incorporated into the project under the project's proprietary license.

5. **No Compensation**: Contributions are voluntary and no compensation is provided.

By submitting a pull request, you acknowledge and agree to these terms.

---

## How to Contribute

### Reporting Bugs

1. **Search existing issues** to ensure the bug hasn't been reported
2. Create a new issue with:
   - Clear, descriptive title
   - Steps to reproduce
   - Expected vs actual behavior
   - Environment details (OS, Rust version, etc.)
   - Relevant logs or screenshots

### Suggesting Features

1. Open an issue with the `enhancement` label
2. Describe the feature and its use case
3. Explain why it would benefit the project
4. Be prepared for discussion

### Submitting Code

1. Open an issue first to discuss your proposed changes
2. Wait for approval before starting significant work
3. Follow the [Pull Request Process](#pull-request-process)

---

## Development Setup

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Dioxus CLI
cargo install dioxus-cli

# Clone the repository (view-only, do not fork)
git clone <repo-url>
cd superApp
```

### Running the Application

```bash
# Desktop development (with hot-reload)
dx serve --desktop

# Build for production
dx build --release --desktop

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Project Structure

```
superApp/
├── src/
│   ├── main.rs           # Entry point, routing, state management
│   ├── backend/
│   │   ├── mod.rs        # Backend event loop, command handlers
│   │   ├── dag.rs        # DAG node structures
│   │   ├── store.rs      # Storage layer (SQLite/In-memory)
│   │   ├── network.rs    # libp2p network behaviors
│   │   ├── identity.rs   # Keypair management
│   │   ├── vm.rs         # Smart contract VM
│   │   └── wasm.rs       # WASM runtime
│   └── components/       # Dioxus UI components
├── assets/               # Static assets (CSS, images)
├── Cargo.toml           # Dependencies
└── Dioxus.toml          # Dioxus configuration
```

---

## Coding Standards

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix all warnings
- Use meaningful variable and function names
- Document public APIs with doc comments

### Code Organization

```rust
// Good: Clear, documented function
/// Calculates the token balance for a given public key.
/// 
/// Returns the total balance including pending transfers.
pub fn get_balance(&self, pubkey: &str) -> Result<i64, Box<dyn std::error::Error>> {
    // Implementation
}

// Bad: Unclear, undocumented
pub fn gb(p: &str) -> Result<i64, Box<dyn std::error::Error>> {
    // Implementation
}
```

### Error Handling

- Use `Result` for fallible operations
- Provide meaningful error messages
- Avoid `unwrap()` in production code; use `expect()` with context or proper error handling

### Dioxus Components

- Use the `#[component]` macro for all components
- Keep components focused and reusable
- Use `use_signal` for local state
- Use `use_context` for shared state
- Follow the existing component patterns

```rust
#[component]
pub fn MyComponent(#[props] some_prop: String) -> Element {
    let mut local_state = use_signal(|| String::new());
    let app_state = use_context::<AppState>();
    
    rsx! {
        div { class: "my-component",
            // Component content
        }
    }
}
```

---

## Commit Guidelines

### Commit Message Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation changes |
| `style` | Formatting, no code change |
| `refactor` | Code restructuring |
| `test` | Adding/updating tests |
| `chore` | Maintenance tasks |

### Examples

```
feat(browser): add search functionality for SuperWeb pages

Implement full-text search across title, description, and tags.
Add SearchWebPages command and SearchResultsFetched event.

Closes #123
```

```
fix(messaging): resolve decryption failure for some messages

The nonce was being decoded incorrectly when containing special characters.
Added proper hex decoding with error handling.
```

---

## Pull Request Process

### Before Creating a PR

1. ✅ Ensure your code compiles: `cargo build`
2. ✅ All tests pass: `cargo test`
3. ✅ Code is formatted: `cargo fmt`
4. ✅ No clippy warnings: `cargo clippy`
5. ✅ You've read and agreed to the [Legal Notice](#-important-legal-notice)

### Creating the PR

1. Create a branch from `main`: `git checkout -b feature/my-feature`
2. Make your changes with clear, atomic commits
3. Push to the repository
4. Open a Pull Request with:
   - Clear title describing the change
   - Description of what and why
   - Link to related issue(s)
   - Screenshots for UI changes

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation update
- [ ] Refactoring
- [ ] Other (describe)

## Related Issue
Closes #<issue number>

## Testing
Describe how you tested the changes

## Screenshots
(if applicable)

## Legal Acknowledgment
- [ ] I understand and agree that by submitting this PR, I transfer all rights to my contribution to the project owner
```

### Review Process

1. Maintainer reviews the code
2. Address any requested changes
3. Once approved, maintainer will merge

---

## Testing

### Writing Tests

- Add tests for new functionality
- Update tests when modifying existing code
- Place unit tests in the same file with `#[cfg(test)]` module
- Use descriptive test names

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_node_creation_and_verification() {
        let keypair = Keypair::generate_ed25519();
        let node = DagNode::new(
            "test:v1".to_string(),
            DagPayload::Post(PostPayload { ... }),
            vec![],
            &keypair,
            1,
        ).expect("Failed to create node");
        
        assert!(node.verify().expect("Verification failed"));
    }
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_dag_node

# Run tests with output
cargo test -- --nocapture
```

---

## Architecture Guide

### Adding a New Feature

1. **Define Data Structures** (`dag.rs`)
   - Create new payload types if needed
   - Ensure they implement `Serialize`, `Deserialize`, `Clone`, `PartialEq`

2. **Add Storage Methods** (`store.rs`)
   - Implement read/write methods for the new data type

3. **Add Commands/Events** (`mod.rs`)
   - Define new `AppCmd` variants for user actions
   - Define new `AppEvent` variants for updates
   - Implement handlers in `handle_command`

4. **Create UI Components** (`components/`)
   - Build Dioxus component following existing patterns
   - Connect to backend via commands and events
   - Add to routing if it's a new page

5. **Update State** (`components/mod.rs`)
   - Add new fields to `AppState` if needed
   - Handle new events in `main.rs`

### Network Protocol Changes

When modifying network behavior:

1. Update `BlockRequest` or `BlockResponse` in `network.rs`
2. Handle new message types in `handle_swarm_event`
3. Consider backward compatibility
4. Update replication logic if needed

---

## Questions?

If you have questions about contributing, please open an issue with the `question` label.

---

**Thank you for contributing to P2P SuperApp!** 🎉
