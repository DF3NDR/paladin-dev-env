# Contributing to Paladin

> **Archived — historical document.** This page is an earlier draft of the contributing guide and
> is not maintained. For the current, maintained contributing guide, see
> [Development Setup](../contributing/development-setup.md). This disposition is recorded in
> ADR-0047 (`.planning/decisions/0047-architecture-appendix-disposition.md`).

Thank you for your interest in contributing to Paladin! This guide will help you get started with contributing code, documentation, or other improvements.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Architecture Guidelines](#architecture-guidelines)
- [Testing Requirements](#testing-requirements)
- [Documentation Standards](#documentation-standards)
- [Pull Request Process](#pull-request-process)
- [Community](#community)

## Code of Conduct

We follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Please be respectful, inclusive, and professional in all interactions.

## Getting Started

### Prerequisites

```bash
# Install Rust 1.88+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools
cargo install cargo-watch cargo-audit cargo-llvm-cov

# Clone repository
git clone https://github.com/DF3NDR/paladin-dev-env.git
cd paladin

# Start development services
make dev
```

### Project Structure

```
src/
├── core/                    # Domain layer (pure business logic)
├── application/             # Use cases and port definitions
└── infrastructure/          # Adapters for external systems

crates/                      # Cargo workspace member crates (paladin-core, paladin-ports, …)
docs/                        # Documentation
tests/                       # Integration and functional tests
examples/                    # Example code
```

See [docs/architecture/overview.md](../architecture/overview.md) for detailed architecture.

## Development Workflow

### 1. Create a Feature Branch

```bash
git checkout -b feature/your-feature-name
```

### 2. Make Changes Following TDD

```bash
# 1. Write failing test
cargo test test_new_feature  # Should fail

# 2. Implement feature
# Edit src/...

# 3. Make test pass
cargo test test_new_feature  # Should pass

# 4. Refactor
cargo fmt
cargo clippy
```

### 3. Ensure Quality

```bash
# Run all checks
make clean-code

# This runs:
# - cargo fmt --check
# - cargo clippy --all-targets --all-features -- -D warnings
# - cargo test --all-features
# - cargo audit
```

### 4. Commit with Conventional Commits

```bash
git add .
git commit -m "feat: add new Battalion pattern

- Implement Skirmish pattern for ad-hoc agent coordination
- Add configuration builder
- Include integration tests

Closes #123"
```

**Commit Types:**
- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `refactor:` Code refactoring
- `test:` Test additions/changes
- `chore:` Build/tooling changes

### 5. Push and Create PR

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub.

## Architecture Guidelines

### Hexagonal Architecture Rules

1. **Core Layer** (`src/core/`)
   - ✅ Pure business logic
   - ✅ Domain entities and value objects
   - ❌ No external dependencies
   - ❌ No I/O operations

2. **Application Layer** (`src/application/`)
   - ✅ Use case implementations
   - ✅ Port trait definitions
   - ✅ Can import `core`
   - ❌ Cannot import `infrastructure`

3. **Infrastructure Layer** (`src/infrastructure/`)
   - ✅ Adapter implementations
   - ✅ External integrations
   - ✅ Can import `core` and `application`

### Naming Conventions

Follow the **Medieval Military** theme:

| Concept | Term | Example |
|---------|------|---------|
| AI Agent | Paladin | `struct Paladin` |
| Memory | Garrison | `trait GarrisonPort` |
| Tool | Arsenal/Armament | `struct Arsenal` |
| Multi-Agent | Battalion | `enum BattalionPattern` |
| State Persistence | Citadel | `trait CitadelPort` |

See [docs/architecture/domain-model.md](../architecture/domain-model.md) for complete vocabulary.

### Design Patterns

Use established patterns consistently:

- **Builder Pattern**: Complex object construction
- **Port/Adapter Pattern**: External dependencies
- **Repository Pattern**: Data persistence
- **Strategy Pattern**: Algorithm variation

See [docs/architecture/design-patterns.md](../architecture/design-patterns.md) for details.

## Testing Requirements

### Coverage Requirements

- **Unit Tests**: ≥ 80% coverage
- **Integration Tests**: ≥ 70% coverage
- **Doc Tests**: All public APIs

### Test Organization

```
tests/
├── unit/              # Unit tests (fast, no I/O)
├── integration/       # Integration tests (Docker services)
└── functional/        # End-to-end functional tests
```

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paladin_builder() {
        let paladin = PaladinBuilder::new(mock_llm_port())
            .name("Test")
            .system_prompt("You are a tester")
            .build()
            .unwrap();

        assert_eq!(paladin.data.name, "Test");
    }

    #[tokio::test]
    async fn test_paladin_execution() {
        let paladin = create_test_paladin();
        let result = paladin.execute("test input").await.unwrap();
        assert!(!result.content.is_empty());
    }
}
```

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test --features integration-tests

# Specific test
cargo test test_paladin_builder

# With coverage
cargo llvm-cov --html
```

See [docs/contributing/testing-guide.md](../contributing/testing-guide.md) for complete testing guide.

## Documentation Standards

### Rustdoc Comments

All public items must have documentation:

```rust
/// Represents an autonomous AI agent.
///
/// A Paladin executes tasks using an LLM backend, maintains conversation
/// history via a Garrison, and can invoke external tools through an Arsenal.
///
/// # Examples
///
/// ```
/// use paladin::PaladinBuilder;
///
/// let paladin = PaladinBuilder::new(llm_port)
///     .name("Assistant")
///     .system_prompt("You are helpful")
///     .build()?;
/// ```
pub struct Paladin {
    // ...
}
```

### Module Documentation

```rust
//! Paladin agent execution system.
//!
//! This module provides the core Paladin agent implementation with support
//! for memory (Garrison), tools (Arsenal), and multi-agent coordination (Battalion).

mod paladin;
mod garrison;
```

### Markdown Documentation

- Use clear section hierarchy (H1 → H2 → H3)
- Include code examples
- Add diagrams (ASCII art)
- Provide troubleshooting sections
- Cross-reference related docs

## Pull Request Process

### PR Checklist

Before submitting, ensure:

- [ ] Code follows hexagonal architecture
- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated (rustdoc + markdown)
- [ ] Examples added/updated if applicable
- [ ] CHANGELOG.md updated
- [ ] Commit messages follow conventional format

### PR Template

```markdown
## Description

Brief description of the changes.

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing

Describe testing performed:
- Unit tests added/updated
- Integration tests added/updated
- Manual testing steps

## Checklist

- [ ] Tests pass
- [ ] Code formatted
- [ ] Documentation updated
- [ ] CHANGELOG updated
```

### Review Process

1. **Automated Checks**: CI must pass
2. **Code Review**: At least one approval required
3. **Documentation Review**: Check docs are clear
4. **Testing Review**: Verify adequate test coverage
5. **Merge**: Squash and merge to main

## Community

### Getting Help

- **Documentation**: See [docs/](../introduction.md)
- **Issues**: GitHub Issues for bugs/features
- **Discussions**: GitHub Discussions for questions
- **Discord**: Join our Discord server (link TBD)

### Reporting Bugs

Use this template for bug reports:

```markdown
**Description**
Clear description of the bug.

**To Reproduce**
Steps to reproduce:
1. Run command...
2. See error...

**Expected Behavior**
What should happen.

**Environment**
- Paladin version:
- Rust version:
- OS:

**Additional Context**
Logs, screenshots, etc.
```

### Suggesting Features

Use this template for feature requests:

```markdown
**Problem Statement**
What problem does this solve?

**Proposed Solution**
Describe your solution.

**Alternatives Considered**
Other approaches you've thought about.

**Additional Context**
Examples, mockups, etc.
```

## Specialized Contribution Guides

- **[Adapter Development](../contributing/contributing-providers.md)** - Creating new adapters
- **[Testing Guide](../contributing/testing-guide.md)** - Comprehensive testing guide
- **[Provider Integration](../contributing/contributing-providers.md)** - Adding LLM providers

## Recognition

Contributors are recognized in:
- `CONTRIBUTORS.md` file
- Release notes
- Project documentation

Thank you for contributing to Paladin! 🛡️
