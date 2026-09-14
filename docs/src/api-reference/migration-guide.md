# Migration Guide

This guide covers all breaking changes since v0.1.0 up to the current **v0.5.0** release.

## Upgrading to v0.10.0 (from v0.9.x)

This historical guide stops at v0.5.0. The v0.10.0 upgrade record lives on the
[Upgrading](upgrading.md) page and in the root
[`MIGRATION.md`](https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md) file, which
together cover every behavioral change, Rust API change, schema migration, configuration
change and the operator upgrade checklist for v0.9.x → v0.10.0. The intervening 0.6 through 0.9
changes are recorded in
[`CHANGELOG.md`](https://github.com/DF3NDR/paladin-dev-env/blob/main/CHANGELOG.md), not in this
guide.

## Table of Contents

- [Upgrading to v0.10.0 (from v0.9.x)](#upgrading-to-v0100-from-v09x)
- [Migrating to v0.5.0 (from v0.4.x)](#migrating-to-v050-from-v04x)
- [Migrating to v0.4.x (from v0.3.x)](#migrating-to-v04x-from-v03x)
- [Migrating to v0.2.0 (from v0.1.x)](#migrating-to-v020-from-v01x)
- [Migrating to v0.1.0 (Feature Flag Reorganization)](#migrating-to-v010-feature-flag-reorganization)
- [Migration Scenarios](#migration-scenarios)
- [Testing Your Migration](#testing-your-migration)

---

## Migrating to v0.5.0 (from v0.4.x)

**No user-facing breaking changes.** v0.5.0 is the documentation-overhaul release (Milestone 11):
the full MDBook was published to GitHub Pages, new orchestration / content-processing / bridge
guides and a crate-map reference were added, and all documentation examples are now compile-verified
against the workspace. No public API changed — bump your dependency from `0.4` to `0.5` and rebuild.

The historical `version = "0.4"` snippets below remain valid for the 0.3 → 0.4 migration they
document; for v0.5.0 simply substitute `"0.5"`.

---

## Migrating to v0.4.x (from v0.3.x)

No user-facing breaking changes in v0.4.0–v0.4.3. Internal module renames only:

- **`paladin-content` module rename** (v0.4.0): `crates/paladin-content/src/use_cases/` was renamed to `crates/paladin-content/src/services/`. If you import `paladin_content::use_cases::*` directly, update to `paladin_content::services::*`.

---

## Migrating to v0.2.0 (from v0.1.x)

v0.2.0 contains two categories of breaking changes:

### 1. Module Path Rename: `use_cases` → `services`

`src/application/use_cases/` was renamed to `src/application/services/`. All import paths changed:

| Old path | New path |
|----------|----------|
| `paladin::application::use_cases::paladin::*` | `paladin::application::services::paladin::*` |
| `paladin::application::use_cases::battalion::*` | `paladin::application::services::battalion::*` |
| `paladin::application::use_cases::arsenal::*` | `paladin::application::services::arsenal::*` |
| `paladin::application::use_cases::content::*` | `paladin::application::services::content::*` |
| `paladin::application::use_cases::herald::*` | `paladin::application::services::herald::*` |
| `paladin::application::use_cases::orchestration::*` | `paladin::application::services::orchestration::*` |
| `paladin::application::use_cases::sanctum::*` | `paladin::application::services::sanctum::*` |

**Fix**: Replace `::use_cases::` with `::services::` in all import paths.

```bash
# Find affected imports
grep -r "use_cases" src/
# Replace
find src/ -name "*.rs" -exec sed -i 's/use_cases/services/g' {} +
```

### 2. Removed Short-path Aliases

Zero-consumer `pub use` re-export aliases were removed from `src/lib.rs`. These had no workspace consumers; all underlying types are unchanged.

**Fix**: Replace `paladin::<Type>` short paths with crate-level import paths (`paladin_ports::`, `paladin_core::`, `paladin_battalion::`, etc.). See [STABLE_API.md](https://github.com/DF3NDR/paladin-dev-env/blob/main/STABLE_API.md) for the canonical import paths.

---

## Migrating to v0.1.0 (Feature Flag Reorganization)

This section covers the original feature-flag reorganization that happened at v0.1.0.

### The Change

**Old Default Features** (pre-v0.1.0):
```toml
default = ["redis-queue", "s3-storage", "openai-embeddings"]
```

**New Default Features** (v0.1.0+):
```toml
default = ["llm-openai"]
```

### Impact

If you were relying on **default features** to provide:
- ❌ Redis queue adapter (`redis-queue`)
- ❌ S3/MinIO storage adapter (`s3-storage`)
- ❌ OpenAI embeddings (`openai-embeddings`)

These are **no longer enabled by default** and must be explicitly added to your `Cargo.toml`.

### Who Is Affected?

You are affected if:

1. **You use Redis queues** in your code
2. **You use S3/MinIO file storage** in your code
3. **You use OpenAI embeddings** in your code
4. **Your `Cargo.toml` does NOT explicitly list features**, relying only on:
   ```toml
   [dependencies]
   paladin-ai = "0.4"  # No features = default features
   ```

You are **NOT** affected if:

- ✅ You already explicitly list all required features in `Cargo.toml`
- ✅ You only use core Paladin orchestration (agents, battalions)
- ✅ You use `features = ["full"]` for development

## Quick Fix

### Option 1: Restore Old Behavior (Recommended for Migration)

Add the old default features explicitly:

```toml
[dependencies]
paladin-ai = { version = "0.4", features = ["llm-openai", "redis-queue", "s3-storage", "openai-embeddings"] }
```

This maintains exact functionality while being explicit about requirements.

### Option 2: Use the `full` Feature (Development/Testing)

Enable all features:

```toml
[dependencies]
paladin-ai = { version = "0.4", features = ["full"] }
```

**Warning**: This includes ALL optional features. For production, explicitly list only what you need.

### Option 3: Minimal Migration (Production Recommended)

Add only the features you actually use:

```toml
[dependencies]
# Example: Only need Redis queue
paladin-ai = { version = "0.4", features = ["redis-queue"] }

# Example: Only need S3 storage
paladin-ai = { version = "0.4", features = ["s3-storage"] }

# Example: Need both
paladin-ai = { version = "0.4", features = ["redis-queue", "s3-storage"] }
```

## Migration Scenarios

### Scenario 1: Production API Server with Storage

**Before:**
```toml
[dependencies]
paladin-ai = "0.4"  # Implicitly got redis-queue, s3-storage, openai-embeddings
```

**After:**
```toml
[dependencies]
paladin-ai = { version = "0.4", features = ["llm-openai", "redis-queue", "s3-storage", "web-server"] }
```

**Why**: Explicitly declares infrastructure dependencies. Adds `web-server` if you use REST APIs.

### Scenario 2: Content Processing Pipeline

**Before:**
```toml
[dependencies]
paladin-ai = "0.4"
```

**Your code** uses:
- PDF extraction
- Web scraping
- S3 storage
- Redis queues

**After:**
```toml
[dependencies]
paladin-ai = { version = "0.4", features = [
    "llm-openai",           # Default LLM provider
    "content-processing",   # PDF, scraping, RSS, tokenization
    "redis-queue",          # Async job queue
    "s3-storage"            # File storage
] }
```

### Scenario 3: Multi-Provider Agent Orchestration

**Before:**
```toml
[dependencies]
paladin-ai = "0.4"
```

**Your code** uses:
- Multiple LLM providers (OpenAI, Anthropic, DeepSeek)
- No storage or queues

**After:**
```toml
[dependencies]
paladin-ai = { version = "0.4", default-features = false, features = ["llm-all"] }
```

**Why**: `default-features = false` removes the default `llm-openai`, then `llm-all` adds all providers.

### Scenario 4: Microservice with Notifications

**Before:**
```toml
[dependencies]
paladin-ai = "0.4"
```

**Your code** uses:
- Email notifications
- Web API
- S3 storage

**After:**
```toml
[dependencies]
paladin-ai = { version = "0.4", features = [
    "llm-openai",      # LLM provider
    "web-server",      # REST API
    "notifications",   # Email with templates
    "s3-storage"       # File storage
] }
```

### Scenario 5: Development Environment

**Before:**
```toml
[dependencies]
paladin-ai = "0.4"

[dev-dependencies]
# Additional test deps...
```

**After:**
```toml
[dependencies]
# Production - minimal features
paladin-ai = { version = "0.4", features = ["llm-openai", "redis-queue"] }

[dev-dependencies]
# Development - all features for testing
paladin-ai = { version = "0.4", features = ["full"] }
```

## What Changed

### Feature Flag Reorganization

| Category | Old Behavior | New Behavior |
|----------|-------------|--------------|
| **Default Features** | `redis-queue`, `s3-storage`, `openai-embeddings` | `llm-openai` only |
| **LLM Providers** | Implicit (always included) | Explicit flags: `llm-openai`, `llm-anthropic`, `llm-deepseek` |
| **Content Processing** | Always included | `content-processing` flag gates `pdf-extract`, `scraper`, etc. |
| **Web Server** | Always included | `web-server` flag gates `actix-web`, `axum` |
| **Notifications** | Always included | `notifications` flag gates `lettre`, `handlebars` |
| **Vision** | Implicit | `vision` flag for multimodal capabilities |

### New Convenience Flags

| Flag | Equivalent To | Purpose |
|------|---------------|---------|
| `llm-all` | `llm-openai` + `llm-anthropic` + `llm-deepseek` | All LLM providers |
| `full` | All optional features | Development/testing |

## Why This Change

### Benefits

1. **Smaller Binaries** - Default build is ~40% smaller (10-14 MB vs 25-35 MB)
2. **Faster Compile Times** - Default build compiles ~60% faster (40-60s vs 3-5 min)
3. **Clearer Dependencies** - Explicit about what your application actually uses
4. **Better Modularity** - Pick only the LLM providers you need
5. **Security** - Smaller attack surface by excluding unused dependencies

### Philosophy

**Old Approach**: "Include everything by default, users opt-out if needed"
- ❌ Slow compilation for simple use cases
- ❌ Large binaries even for minimal deployments
- ❌ Unclear what features are actually required

**New Approach**: "Start minimal, opt-in to what you need"
- ✅ Fast iteration for core orchestration development
- ✅ Explicit about infrastructure dependencies
- ✅ Production builds include only necessary code

## Testing Your Migration

### Step 1: Update Cargo.toml

Apply one of the migration scenarios above.

### Step 2: Verify Compilation

```bash
# Clean build to ensure no cached artifacts
cargo clean

# Build with your new features
cargo build

# Check for missing features (look for errors like):
# error[E0433]: failed to resolve: use of undeclared crate or module `redis`
```

### Step 3: Run Tests

```bash
# Run all tests with your feature set
cargo test

# If you have integration tests requiring services:
cargo test --features integration-tests
```

### Step 4: Check for Warnings

```bash
# Ensure no clippy warnings about unused dependencies
cargo clippy --all-targets -- -D warnings
```

### Step 5: Verify Runtime Behavior

Test critical paths that use:
- Redis queues (if using `redis-queue`)
- S3 storage (if using `s3-storage`)
- Email notifications (if using `notifications`)
- Web APIs (if using `web-server`)

## Common Migration Errors

### Error 1: Unresolved Import

```
error[E0432]: unresolved import `paladin::infrastructure::adapters::queue::redis`
```

**Cause**: Missing `redis-queue` feature

**Fix**:
```toml
paladin-ai = { version = "0.4", features = ["redis-queue"] }
```

### Error 2: Missing Adapter Struct

```
error[E0433]: failed to resolve: use of undeclared type `MinioAdapter`
```

**Cause**: Missing `s3-storage` feature

**Fix**:
```toml
paladin-ai = { version = "0.4", features = ["s3-storage"] }
```

### Error 3: Content Type Detection Missing

```
error[E0425]: cannot find function `detect_content_type` in this scope
```

**Cause**: Missing `s3-storage` feature (function is feature-gated)

**Fix**:
```toml
paladin-ai = { version = "0.4", features = ["s3-storage"] }
```

### Error 4: PDF Extraction Failed

```
error[E0433]: failed to resolve: use of undeclared crate `pdf_extract`
```

**Cause**: Missing `content-processing` feature

**Fix**:
```toml
paladin-ai = { version = "0.4", features = ["content-processing"] }
```

## Rollback Plan

If you need to temporarily revert to old behavior while planning migration:

### Option 1: Pin to Old Version

```toml
[dependencies]
paladin = "0.0.x"  # Use specific pre-v0.1.0 version
```

Check available versions:
```bash
cargo search paladin
```

### Option 2: Use Full Features

```toml
[dependencies]
paladin-ai = { version = "0.4", features = ["full"] }
```

This includes everything and more, allowing time for proper migration planning.

## Getting Help

### Documentation

- **Feature Flags Reference**: [Feature Flags](feature-flags.md)
- **Configuration Guide**: [Configuration Guide](../getting-started/configuration.md)
- **Changelog**: [CHANGELOG](https://github.com/DF3NDR/paladin-dev-env/blob/main/CHANGELOG.md)

### Support Channels

- **GitHub Issues**: [Report migration problems](https://github.com/DF3NDR/paladin-dev-env/issues)
- **GitHub Discussions**: [Ask migration questions](https://github.com/DF3NDR/paladin-dev-env/discussions)
- **Examples**: Check [examples/](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples) for feature-annotated examples

## Checklist

Use this checklist to track your migration:

- [ ] Read this migration guide
- [ ] Identify which features your code uses
- [ ] Update `Cargo.toml` with explicit features
- [ ] Run `cargo clean && cargo build`
- [ ] Run `cargo test`
- [ ] Run `cargo clippy --all-targets -- -D warnings`
- [ ] Test critical runtime paths
- [ ] Update CI/CD workflows if needed
- [ ] Document feature requirements in your README
- [ ] Deploy to staging and verify
- [ ] Deploy to production

## Timeline

| Version | Status | Default Features |
|---------|--------|------------------|
| < 0.1.0 | Old | `redis-queue`, `s3-storage`, `openai-embeddings` |
| 0.1.0 | **Current** | `llm-openai` only |
| Future | Planned | May add more granular LLM provider features |

## Feedback

This migration guide is a living document. If you encounter migration scenarios not covered here, please:

1. Open a GitHub issue describing your use case
2. Submit a PR to add your scenario to this guide
3. Share your experience in GitHub Discussions

Your feedback helps improve Paladin for everyone! 🛡️

---

## CLI Feature Isolation (Milestone 4 — Epic 3)

### What Changed

The `application::cli` module and the `paladin-cli` binary are now gated behind the `cli` feature flag. The following dependencies are now **optional** and only compiled when `cli` is enabled:

- `clap` (CLI argument parsing)
- `dialoguer` (interactive prompts)
- `indicatif` (progress bars)
- `console` (terminal styling)
- `serde_yaml` (YAML config parsing)

### Who Is Affected?

**Library consumers**: No impact. The `cli` feature was never part of the `default` feature set. Library builds are unaffected.

**`paladin-cli` binary users**: The binary now requires `--features cli` to compile:

```bash
# Before (always compiled):
cargo build --bin paladin-cli

# After (requires cli feature):
cargo build --bin paladin-cli --features cli
```

**`full` feature users**: No change — `full` already includes `cli`.

### Migration

If you directly import from `paladin::application::cli` (uncommon — internal use only):

```toml
# Cargo.toml — add the cli feature
[dependencies]
paladin-ai = { version = "0.4", features = ["cli"] }
```

Or add `cli` to your own feature re-export:

```toml
[features]
my-cli = ["paladin/cli"]
```
