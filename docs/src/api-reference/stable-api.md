# Stable Public API Contract

**Version:** 0.5.0
**Last Updated:** 2026-06-02
**Status:** Active

> **Breaking Changes in v0.2.0**: This release includes two categories of breaking changes:
>
> **v0.5.0 API Note**: The canonical import path for all port traits is `crates/paladin-ports/`. Short-path aliases (`paladin::<Type>`) have been removed from `src/lib.rs`. Use full crate-level import paths (e.g. `use paladin_ports::output::llm_port::LlmPort`). The `application::use_cases` module path was renamed to `application::services` in a prior release.
>
> See [CHANGELOG](https://github.com/DF3NDR/paladin-dev-env/blob/main/CHANGELOG.md) for the complete migration tables.

---

## Table of Contents

- [Introduction](#introduction)
- [API Stability Guarantee](#api-stability-guarantee)
- [Versioning Policy](#versioning-policy)
- [Stability Tiers](#stability-tiers)
- [Per-Crate API Surface and Stability](#per-crate-api-surface-and-stability)
- [Stable Public API Catalog](#stable-public-api-catalog)
  - [Port Traits (Output Ports)](#port-traits-output-ports)
  - [Port Traits (Input Ports)](#port-traits-input-ports)
  - [Domain Entities](#domain-entities)
  - [Builder Types](#builder-types)
  - [Configuration Types](#configuration-types)
  - [Error Types](#error-types)
  - [Base Types](#base-types)
- [Internal Implementation Details (Not Stable)](#internal-implementation-details-not-stable)
- [API Change Process](#api-change-process)
- [Migration Guide for Breaking Changes](#migration-guide-for-breaking-changes)
- [Tracking API Changes](#tracking-api-changes)
- [Frequently Asked Questions](#frequently-asked-questions)
- [Questions and Support](#questions-and-support)

---

## Introduction

This document defines the **stable public API contract** for the Paladin framework—a Rust-based enterprise multi-agent orchestration framework built with Hexagonal Architecture and Domain-Driven Design principles.

### Purpose

The stable API contract serves as:
- **Backwards Compatibility Promise**: Types listed here follow strict semantic versioning
- **Integration Guide**: Clear catalog of public types for framework users
- **Evolution Policy**: Transparent process for API changes and deprecations
- **Architectural Boundary**: Distinction between public API and internal implementation

### Scope

This contract covers:
- ✅ **Port Traits**: Primary extension points (LlmPort, GarrisonPort, etc.)
- ✅ **Domain Entities**: Core business types (Paladin, Battalion, etc.)
- ✅ **Builders**: Fluent construction patterns
- ✅ **Configuration**: Application settings types
- ✅ **Errors**: All public error enums
- ✅ **Base Types**: Generic framework primitives

This contract excludes:
- ❌ **Adapter Implementations**: Concrete LLM, storage, queue adapters (internal)
- ❌ **Repositories**: Database access implementations (internal)
- ❌ **CLI**: Command-line interface modules (binary-only)
- ❌ **Web Server**: HTTP server implementation (binary-only)
- ❌ **Managers**: Internal service coordinators (internal)

### Target Audience

- **Library Users**: Building applications with Paladin as a dependency
- **Adapter Developers**: Implementing custom port trait adapters
- **Maintainers**: Managing API evolution and compatibility

## API Stability Guarantee

The types and traits listed in this document follow these rules:

1. **Backwards Compatibility**: Breaking changes will only occur in major version bumps (0.x.0 → 1.0.0, 1.x.0 → 2.0.0)
2. **Deprecation Process**: Types/methods being removed will be deprecated for at least one minor version before removal
3. **Addition Safety**: New methods can be added to traits only if they have default implementations
4. **Documentation**: All public API items must have comprehensive rustdoc with examples
5. **Semver Compliance**: Version numbers follow [Semantic Versioning 2.0.0](https://semver.org/)
6. **MSRV Policy**: Minimum Supported Rust Version (MSRV) changes require minor version bump

---

## Versioning Policy

### Semantic Versioning Interpretation

Paladin follows [Semantic Versioning 2.0.0](https://semver.org/) with the following interpretation:

#### Major Version (X.0.0)

**Breaking changes** that require code changes in dependent crates:

- Removing public types, traits, or functions
- Removing trait methods (even with default implementations)
- Changing trait method signatures
- Changing public struct field types
- Changing error enum variants
- Renaming public items
- Changing function parameter types or return types
- Making previously public items private

#### Minor Version (0.X.0)

**Backwards-compatible additions**:

- Adding new public types, traits, or functions
- Adding new trait methods with default implementations
- Adding new struct fields (with defaults or using builder pattern)
- Adding new error enum variants (when using `#[non_exhaustive]`)
- Adding new modules
- Deprecating APIs (without removing)
- MSRV (Minimum Supported Rust Version) increases

#### Patch Version (0.0.X)

**Backwards-compatible bug fixes**:

- Bug fixes that don't change public API
- Documentation improvements
- Performance optimizations
- Internal refactoring
- Dependency updates (when not affecting public API)

### Pre-1.0 Versioning

During pre-1.0 development (0.x.y):
- **0.x.0** (minor bump): May include breaking changes
- **0.0.x** (patch bump): Backwards-compatible changes only
- Breaking changes will be clearly documented in CHANGELOG.md

### Minimum Supported Rust Version (MSRV)

- **Current MSRV**: Rust 1.88 (stable)
- **MSRV Policy**: Increasing MSRV requires a minor version bump
- **Support Window**: We support the latest stable Rust release and the previous 2 minor releases

---

## Stability Tiers

All public API items are classified into one of four stability tiers:

### 🟢 Stable

**Definition**: Production-ready API with strong backwards compatibility guarantees.

**Guarantees**:
- Will not be removed without deprecation period
- Breaking changes only in major versions
- Comprehensive documentation with examples
- Well-tested with >80% coverage

**Applies to**: All port traits, core domain entities, error types

### 🟡 Unstable

**Definition**: API under active development, subject to change.

**Warnings**:
- May have breaking changes in minor versions
- Documentation may be incomplete
- Not recommended for production use
- Will eventually move to Stable or be removed

**Marked with**: `#[doc(unstable)]` or documented as "Unstable" in rustdoc

### 🔵 Experimental

**Definition**: Early-stage API for testing new features.

**Warnings**:
- May be removed without deprecation
- API design may change significantly
- Requires explicit opt-in via feature flags
- Not suitable for production

**Marked with**: Feature-gated (e.g., `#[cfg(feature = "experimental")]`)

### 🔴 Deprecated

**Definition**: API scheduled for removal in a future version.

**Process**:
- Marked with `#[deprecated(since = "x.y.z", note = "use X instead")]`
- Will be removed in next major version
- Migration path documented in MIGRATION.md
- Alternative APIs provided

**Marked with**: `#[deprecated]` attribute with migration guidance

### Tier Progression

```
Experimental → Unstable → Stable → Deprecated → Removed
                   ↓          ↓
                Removed   (Maintained)
```

---

## Per-Crate API Surface and Stability

This section documents the public API contract per crate, aligned with the
workspace decomposition completed in Milestone 7.

### Stability Legend

- **Stable**: Backward-compatible under normal semver rules.
- **Unstable**: Public but expected to evolve; avoid strict coupling.
- **Experimental**: Feature-gated or early-stage APIs, not guaranteed stable.

### `paladin-core`

- **Stable**: Domain entities, value objects, and core container/base types.
- **Unstable**: None declared.
- **Experimental**: Feature-gated additions, if introduced later.

### `paladin-ports`

- **Stable**: Input and output port traits used as architectural contracts.
- **Unstable**: Traits explicitly documented as in-progress, if any.
- **Experimental**: Feature-gated ports only.

### `paladin-battalion`

- **Stable**: Battalion orchestration surface (Formation, Phalanx, Campaign,
   Chain of Command, Conclave, Council, Grove, Maneuver, Commander).
- **Unstable**: New orchestration APIs marked as in-progress.
- **Experimental**: Feature-gated orchestration behaviors.

### `paladin-llm`

- **Stable**: Provider-agnostic request/response contracts and adapter entrypoints.
- **Unstable**: Provider-specific extensions pending stabilization.
- **Experimental**: Feature-gated or preview provider capabilities.

### `paladin-memory`

- **Stable**: Garrison and Sanctum public service/adapter contracts.
- **Unstable**: New retrieval and extraction options under evaluation.
- **Experimental**: Feature-gated memory backends or indexing variants.

### `paladin-web`

- **Stable**: Public web adapter integration surface used by the facade/composition root.
- **Unstable**: Handler contracts in active iteration.
- **Experimental**: Feature-gated web extensions.

### `paladin-notifications`

- **Stable**: Notification adapter contracts and channel abstractions.
- **Unstable**: Provider-specific channel enhancements.
- **Experimental**: New feature-gated notification channels.

### `paladin-content`

- **Stable**: Content adapter and use-case service entrypoints.
- **Unstable**: Rapidly iterating analysis and ingestion specializations.
- **Experimental**: Feature-gated parsing and enrichment capabilities.

### `paladin-storage`

- **Stable**: Repository adapter contracts and storage entrypoints.
- **Unstable**: Backend-specific tuning hooks and migration internals.
- **Experimental**: Feature-gated storage backends.

### `paladin` (facade crate)

The facade crate is the **application assembly point and composition root**. It wires leaf
crates together into a runnable application via `ServiceRunner`. It does not contain business
logic, port trait definitions, or infrastructure adapter implementations — those live exclusively
in the leaf crates.

Module layout (post-Milestone 8):

- `application/services/` — Application coordination services (11 sub-modules)
- `application/cli/` — CLI command implementations (feature-gated: `cli`)
- `config/` — Multi-source configuration loading and settings types
- `infrastructure/` — Infrastructure adapter implementations not yet extracted to a leaf crate
- `core/` — Minimal re-export bridge to `paladin-core`
- `bin/paladin-cli.rs` — CLI binary entry point (feature-gated: `cli`)
- `main.rs` — Default binary entry point

Stability tiers:

- **Stable**: Curated top-level re-exports and extension points listed in this
   stable API document.
- **Unstable**: Convenience exports marked as transitional.
- **Experimental**: Feature-gated facade exports.

### Cross-Crate Dependency Contract

The public dependency chain is intentionally layered:

1. `paladin-core` (domain foundation)
2. `paladin-ports` (contracts on top of core)
3. leaf crates (`paladin-battalion`, `paladin-llm`, `paladin-memory`,
    `paladin-web`, `paladin-notifications`, `paladin-content`, `paladin-storage`)
4. `paladin` facade (curated re-exports)

Breaking changes to lower layers can cascade upward. Therefore, compatibility
reviews must start at `paladin-core` and `paladin-ports` before assessing leaf
crate or facade impacts.

---

## Stable Public API Catalog

## Tracking API Changes

### Automated Tracking with cargo-public-api

We use [`cargo-public-api`](https://github.com/Enselic/cargo-public-api) to track changes to the public API surface:

#### Generate Current API Surface

```bash
./scripts/extract-public-api.sh project/current-exports.txt
```

This creates a baseline snapshot of all public items (items as of v0.5.0).

#### Check for API Changes (CI)

```bash
./scripts/check-api-surface.sh project/current-exports.txt
```

Compares current API against baseline. Fails CI if changes detected without baseline update.

#### Check Deprecation Warnings

```bash
./scripts/check-deprecations.sh
```

Verifies that deprecated items compile with warnings.

### CI Integration

API surface changes are automatically detected in CI (`.github/workflows/ci.yml`):

```yaml
- name: Check API Surface
  run: ./scripts/check-api-surface.sh project/current-exports.txt
```

**If the API changes:**
1. CI build will fail with diff showing changes
2. Review changes carefully for breaking changes
3. Update `CHANGELOG.md` with details
4. Update baseline: `./scripts/extract-public-api.sh project/current-exports.txt`
5. Increment version per semver

### Manual API Verification

```bash
# View current public API
cargo public-api --simplified | less

# Compare against previous version
cargo public-api --diff-git-checkouts v0.3.0 v0.5.0

# Generate HTML diff
cargo public-api --diff-git-checkouts v0.3.0 v0.5.0 --output-format markdown
```

---

## Frequently Asked Questions

### General

**Q: What is considered a "breaking change"?**

A: Any change that would cause existing code to fail compilation or change behavior:
- Removing public types, traits, or functions
- Removing trait methods
- Changing method signatures (parameters, return types)
- Renaming public items
- Changing struct field types
- Making previously public items private
- Removing error enum variants (without `#[non_exhaustive]`)

See [Versioning Policy](#versioning-policy) for complete list.

**Q: Can I depend on adapter implementations (e.g., `OpenAIAdapter`)?**

A: **Not recommended for library code**. Adapters are internal implementation details that may change in minor versions. Use port traits (`LlmPort`, etc.) instead. Adapters are fine in application code and examples.

**Q: How long are deprecated APIs supported?**

A: Deprecated APIs remain functional for at least one minor version (e.g., deprecated in 0.2.0, removed in 0.3.0 or 1.0.0). We aim to provide at least 3 months of deprecation period for major APIs.

**Q: What's the timeline for 1.0.0?**

A: We'll release 1.0.0 when:
1. All major features are implemented and stable
2. API design has proven stable in production use
3. Documentation is comprehensive
4. At least 6 months of pre-1.0 usage in real projects

Expected: Q3-Q4 2026.

### Port Traits

**Q: Can I add methods to existing port traits?**

A: **Yes**, if the method has a default implementation. This is backwards-compatible. Methods without defaults are breaking changes.

**Q: Can I implement port traits for my own types?**

A: **Yes!** Port traits are designed for user implementation. Implement `LlmPort` for your custom LLM provider, `GarrisonPort` for your storage system, etc.

**Q: Do port traits require specific async runtimes?**

A: Port traits are runtime-agnostic. The default implementations use Tokio, but you can implement ports for any async runtime.

### Error Handling

**Q: Can I add new variants to error enums?**

A: **Yes**, all error enums are marked `#[non_exhaustive]`, allowing new variants in minor versions. Always use a wildcard match:

```rust,ignore
match error {
    PaladinError::ConfigurationError(_) => { /* ... */ },
    PaladinError::Timeout(_) => { /* ... */ },
    _ => { /* catch-all for future variants */ },
}
```

**Q: Are error messages part of the stable API?**

A: **No**. Error messages may change in any version. Don't parse error strings—use enum variants instead.

### Versioning

**Q: What does "0.x.0" mean before 1.0?**

A: During pre-1.0:
- **0.x.0** (minor bump): May include breaking changes
- **0.0.x** (patch bump): Backwards-compatible changes only

Breaking changes in 0.x versions will be clearly documented.

**Q: When will you increase MSRV (Minimum Supported Rust Version)?**

A: MSRV increases require a minor version bump. We target the latest stable Rust and the previous 2 minor releases. Current MSRV: **Rust 1.88**.

### Migration

**Q: Where do I find migration guides?**

A:
- **CHANGELOG.md**: List of all breaking changes by version
- **docs/MIGRATION.md**: Step-by-step upgrade guides
- **GitHub Releases**: Migration highlights in release notes
- **Rustdoc**: Deprecated item documentation includes alternatives

**Q: Can I use both old and new APIs during migration?**

A: **Yes**. During the deprecation period, both old and new APIs coexist. This allows gradual migration.

### Contributing

**Q: How do I propose an API change?**

A: See [API Change Process](#api-change-process) above. Start by opening a GitHub issue with the `api-change` label.

**Q: Can I contribute new port traits?**

A: **Yes!** Propose new ports via GitHub issue. New stable ports require:
- Clear use case and motivation
- Comprehensive rustdoc with examples
- At least one concrete implementation
- Tests and doc tests

---

## Stable Public API Surface

### Port Traits (Output Ports)

Port traits are the **primary stable API** and define extension points for integrating external systems. All output ports are located in `src/application/ports/output/`.

| Type | Fully Qualified Path | Tier | Description | Documentation |
|------|---------------------|------|-------------|---------------|
| `LlmPort` | `paladin_ports::output::llm_port::LlmPort` | 🟢 Stable | LLM provider abstraction (OpenAI, DeepSeek, Anthropic) | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/llm_port/trait.LlmPort.html) |
| `GarrisonPort` | `paladin_ports::output::garrison_port::GarrisonPort` | 🟢 Stable | Short-term conversation memory storage | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/garrison_port/trait.GarrisonPort.html) |
| `LongTermGarrisonPort` | `paladin_ports::output::garrison_port::LongTermGarrisonPort` | 🟢 Stable | Long-term memory with semantic search | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/garrison_port/trait.LongTermGarrisonPort.html) |
| `SanctumPort` | `paladin_ports::output::sanctum_port::SanctumPort` | 🟢 Stable | Vector storage and similarity search | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/sanctum_port/trait.SanctumPort.html) |
| `EmbeddingPort` | `paladin_ports::output::embedding_port::EmbeddingPort` | 🟢 Stable | Text-to-vector embedding generation | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/embedding_port/trait.EmbeddingPort.html) |
| `ArsenalPort` | `paladin_ports::output::arsenal_port::ArsenalPort` | 🟢 Stable | External tool execution via MCP | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/arsenal_port/trait.ArsenalPort.html) |
| `ArsenalRegistry` | `paladin_ports::output::arsenal_port::ArsenalRegistry` | 🟢 Stable | Tool discovery and registration | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/arsenal_port/trait.ArsenalRegistry.html) |
| `CitadelPort` | `paladin_ports::output::citadel_port::CitadelPort` | 🟢 Stable | State persistence and recovery | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/citadel_port/trait.CitadelPort.html) |
| `QueuePort` | `paladin_ports::output::queue_port::QueuePort` | 🟢 Stable | Async task queue and job processing | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/queue_port/trait.QueuePort.html) |
| `NotificationDeliveryPort` | `paladin_ports::output::notification_port::NotificationDeliveryPort` | 🟢 Stable | Multi-channel notification delivery | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/notification_port/trait.NotificationDeliveryPort.html) |
| `NotificationTemplatePort` | `paladin_ports::output::notification_port::NotificationTemplatePort` | 🟢 Stable | Notification template management | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/notification_port/trait.NotificationTemplatePort.html) |
| `FileStoragePort` | `paladin_ports::output::file_storage_port::FileStoragePort` | 🟢 Stable | Cloud and local file storage | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/file_storage_port/trait.FileStoragePort.html) |
| `PaladinPort` | `paladin_ports::output::paladin_port::PaladinPort` | 🟢 Stable | AI agent execution abstraction | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/paladin_port/trait.PaladinPort.html) |
| `BattalionPort` | `paladin_ports::output::battalion_port::BattalionPort` | 🟢 Stable | Multi-agent orchestration | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/battalion_port/trait.BattalionPort.html) |

### Port Traits (Input Ports)

Input ports define use case interfaces for application entry points. Located in `src/application/ports/input/`.

| Type | Fully Qualified Path | Tier | Description | Documentation |
|------|---------------------|------|-------------|---------------|
| `ContentIngestionPort` | `paladin_ports::input::content_input_port::ContentIngestionPort` | 🟡 Unstable | Content ingestion use cases | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/input/content_input_port/trait.ContentIngestionPort.html) |
| `DocumentPort` | `paladin_ports::input::document_port::DocumentPort` | 🟢 Stable | Document processing use cases | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/input/document_port/trait.DocumentPort.html) |
| `MlPort` | `paladin_ports::input::ml_port::MlPort` | 🟡 Unstable | Machine learning use cases | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/input/ml_port/trait.MlPort.html) |

### Domain Entities

Core business domain types that represent the framework's entities. Located in `src/core/platform/container/`.

#### Paladin (Agent) Types

| Type | Fully Qualified Path | Tier | Description | Documentation |
|------|---------------------|------|-------------|---------------|
| `Paladin` | `paladin::core::platform::container::paladin::Paladin` | 🟢 Stable | Autonomous AI agent entity (Node<PaladinData>) | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/paladin/type.Paladin.html) |
| `PaladinData` | `paladin::core::platform::container::paladin::PaladinData` | 🟢 Stable | Paladin configuration and state data | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/paladin/struct.PaladinData.html) |
| `PaladinConfig` | `paladin::core::platform::container::paladin::PaladinConfig` | 🟢 Stable | Runtime execution configuration | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/paladin/struct.PaladinConfig.html) |
| `PaladinStatus` | `paladin::core::platform::container::paladin::PaladinStatus` | 🟢 Stable | Agent execution status enum | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/paladin/enum.PaladinStatus.html) |
| `PaladinResult` | `paladin_ports::output::paladin_port::PaladinResult` | 🟢 Stable | Agent execution result with metadata | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/paladin_port/struct.PaladinResult.html) |
| `StopReason` | `paladin_ports::output::paladin_port::StopReason` | 🟢 Stable | Why agent execution terminated | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/paladin_port/enum.StopReason.html) |

#### Battalion (Multi-Agent) Types

| Type | Fully Qualified Path | Tier | Description | Documentation |
|------|---------------------|------|-------------|---------------|
| `Battalion` | `paladin::core::platform::container::battalion::Battalion` | 🟢 Stable | Multi-agent coordination entity | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/battalion/type.Battalion.html) |
| `BattalionData` | `paladin::core::platform::container::battalion::BattalionData` | 🟢 Stable | Battalion configuration and state | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/battalion/struct.BattalionData.html) |
| `BattalionResult` | `paladin::core::platform::container::battalion::BattalionResult` | 🟢 Stable | Orchestration execution result | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/battalion/struct.BattalionResult.html) |
| `BattalionStatus` | `paladin::core::platform::container::battalion::BattalionStatus` | 🟢 Stable | Orchestration status enum | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/battalion/enum.BattalionStatus.html) |
| `Formation` | `paladin::core::platform::container::battalion::formation::Formation` | 🟢 Stable | Sequential execution pattern | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/battalion/formation/struct.Formation.html) |
| `Phalanx` | `paladin::core::platform::container::battalion::phalanx::Phalanx` | 🟢 Stable | Parallel execution pattern | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/battalion/phalanx/struct.Phalanx.html) |
| `Campaign` | `paladin::core::platform::container::battalion::campaign::Campaign` | 🟢 Stable | Graph/DAG execution pattern | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/battalion/campaign/struct.Campaign.html) |
| `ChainOfCommand` | `paladin::core::platform::container::battalion::chain_of_command::ChainOfCommand` | 🟢 Stable | Hierarchical delegation pattern | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/battalion/chain_of_command/struct.ChainOfCommand.html) |

#### Memory (Garrison) Types

| Type | Fully Qualified Path | Tier | Description | Documentation |
|------|---------------------|------|-------------|---------------|
| `Garrison` | `paladin::core::platform::container::garrison::Garrison` | 🟢 Stable | Memory storage entity | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/garrison/type.Garrison.html) |
| `Memory` | `paladin::core::platform::container::garrison::Memory` | 🟢 Stable | Individual memory record | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/garrison/struct.Memory.html) |
| `GarrisonStats` | `paladin_ports::output::garrison_port::GarrisonStats` | 🟢 Stable | Memory storage statistics | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/garrison_port/struct.GarrisonStats.html) |

#### Tool (Arsenal) Types

| Type | Fully Qualified Path | Tier | Description | Documentation |
|------|---------------------|------|-------------|---------------|
| `Arsenal` | `paladin::core::platform::container::arsenal::Arsenal` | 🟢 Stable | Tool registry entity | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/arsenal/type.Arsenal.html) |
| `Armament` | `paladin::core::platform::container::arsenal::Armament` | 🟢 Stable | Individual tool/capability metadata | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/arsenal/struct.Armament.html) |
| `ArmamentCall` | `paladin::core::platform::container::arsenal::ArmamentCall` | 🟢 Stable | Tool invocation request | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/arsenal/struct.ArmamentCall.html) |
| `ArmamentResult` | `paladin::core::platform::container::arsenal::ArmamentResult` | 🟢 Stable | Tool execution result | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/arsenal/struct.ArmamentResult.html) |

### Builder Types

Fluent builder patterns for complex object construction. Located in `src/application/services/`.

| Type | Fully Qualified Path | Tier | Description | Documentation |
|------|---------------------|------|-------------|---------------|
| `PaladinBuilder` | `paladin::application::services::paladin::PaladinBuilder` | 🟢 Stable | Fluent builder for Paladin agents | [Docs](https://docs.rs/paladin/latest/paladin/application/services/paladin/struct.PaladinBuilder.html) |
| `CommanderBuilder` | `paladin::application::services::commander::CommanderBuilder` | 🟢 Stable | Fluent builder for Commander routers | [Docs](https://docs.rs/paladin/latest/paladin/application/services/commander/struct.CommanderBuilder.html) |
| `CouncilBuilder` | `paladin::application::services::council::CouncilBuilder` | 🟢 Stable | Fluent builder for Council discussions | [Docs](https://docs.rs/paladin/latest/paladin/application/services/council/struct.CouncilBuilder.html) |
| `GroveBuilder` | `paladin::application::services::grove::GroveBuilder` | 🟢 Stable | Fluent builder for Grove routing | [Docs](https://docs.rs/paladin/latest/paladin/application/services/grove/struct.GroveBuilder.html) |

### Configuration Types

Application and service configuration types. Located in `src/config/`.

| Type | Fully Qualified Path | Tier | Description | Documentation |
|------|---------------------|------|-------------|---------------|
| `ApplicationSettings` | `paladin::config::application_settings::ApplicationSettings` | 🟢 Stable | Application-wide configuration | [Docs](https://docs.rs/paladin/latest/paladin/config/application_settings/struct.ApplicationSettings.html) |
| `LlmConfig` | `paladin::config::application_settings::LlmConfig` | 🟢 Stable | LLM provider configuration | [Docs](https://docs.rs/paladin/latest/paladin/config/application_settings/struct.LlmConfig.html) |
| `ServerConfig` | `paladin::config::application_settings::ServerConfig` | 🟢 Stable | HTTP server configuration | [Docs](https://docs.rs/paladin/latest/paladin/config/application_settings/struct.ServerConfig.html) |
| `DatabaseConfig` | `paladin::config::application_settings::DatabaseConfig` | 🟢 Stable | Database connection configuration | [Docs](https://docs.rs/paladin/latest/paladin/config/application_settings/struct.DatabaseConfig.html) |

### Error Types

All error enums follow `thiserror` patterns for consistent error handling. Located throughout the codebase.

| Type | Fully Qualified Path | Tier | Description | Documentation |
|------|---------------------|------|-------------|---------------|
| `PaladinError` | `paladin::application::services::paladin::error::PaladinError` | 🟢 Stable | Paladin execution errors | [Docs](https://docs.rs/paladin/latest/paladin/application/services/paladin/error/enum.PaladinError.html) |
| `BattalionError` | `paladin::core::platform::container::battalion::BattalionError` | 🟢 Stable | Battalion orchestration errors | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/battalion/enum.BattalionError.html) |
| `GarrisonError` | `paladin_ports::output::garrison_port::GarrisonError` | 🟢 Stable | Memory storage errors | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/garrison_port/enum.GarrisonError.html) |
| `ArsenalError` | `paladin::core::platform::container::arsenal::ArsenalError` | 🟢 Stable | Tool execution errors | [Docs](https://docs.rs/paladin/latest/paladin/core/platform/container/arsenal/enum.ArsenalError.html) |
| `CitadelError` | `paladin::application::errors::citadel_error::CitadelError` | 🟢 Stable | State persistence errors | [Docs](https://docs.rs/paladin/latest/paladin/application/errors/citadel_error/enum.CitadelError.html) |
| `LlmError` | `paladin_ports::output::llm_port::LlmError` | 🟢 Stable | LLM provider errors | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/llm_port/enum.LlmError.html) |
| `EmbeddingError` | `paladin_ports::output::embedding_port::EmbeddingError` | 🟢 Stable | Embedding generation errors | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/embedding_port/enum.EmbeddingError.html) |
| `SanctumError` | `paladin_ports::output::sanctum_port::SanctumError` | 🟢 Stable | Vector storage errors | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/sanctum_port/enum.SanctumError.html) |
| `FileStorageError` | `paladin_ports::output::file_storage_port::FileStorageError` | 🟢 Stable | File storage errors | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/file_storage_port/enum.FileStorageError.html) |
| `NotificationPortError` | `paladin_ports::output::notification_port::NotificationPortError` | 🟢 Stable | Notification delivery errors | [Docs](https://docs.rs/paladin/latest/paladin/application/ports/output/notification_port/enum.NotificationPortError.html) |
| `ConfigError` | `paladin::config::error::ConfigError` | 🟢 Stable | Configuration loading errors | [Docs](https://docs.rs/paladin/latest/paladin/config/error/enum.ConfigError.html) |

### Base Types

Generic framework primitives and patterns. Located in `src/core/base/`.

| Type | Fully Qualified Path | Tier | Description | Documentation |
|------|---------------------|------|-------------|---------------|
| `Node<T>` | `paladin::core::base::entity::node::Node` | 🟢 Stable | Generic entity wrapper with UUID and metadata | [Docs](https://docs.rs/paladin/latest/paladin/core/base/entity/node/struct.Node.html) |
| `Collection<T>` | `paladin::core::base::entity::collection::Collection` | 🟢 Stable | Generic collection type with metadata | [Docs](https://docs.rs/paladin/latest/paladin/core/base/entity/collection/struct.Collection.html) |
| `Field` | `paladin::core::base::entity::field::Field` | 🟢 Stable | Field definition with type information | [Docs](https://docs.rs/paladin/latest/paladin/core/base/entity/field/struct.Field.html) |
| `Message<T>` | `paladin::core::base::entity::message::Message` | 🟢 Stable | Generic message wrapper for events | [Docs](https://docs.rs/paladin/latest/paladin/core/base/entity/message/struct.Message.html) |

### Resilience Types

Fault-tolerance primitives for hardening agent execution. Located in `src/infrastructure/resilience/`.

> **Canonical path change (Milestone 6, Epic 4):** `CircuitBreaker` and `CircuitState` were relocated from
> `paladin::application::services::paladin::circuit_breaker` to `paladin::infrastructure::resilience::circuit_breaker`.
> The old path is retired and no longer resolves.

| Type | Fully Qualified Path | Tier | Description | Documentation |
|------|---------------------|------|-------------|---------------|
| `CircuitBreaker` | `paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker` | 🟢 Stable | Thread-safe circuit breaker for fault tolerance | [Docs](https://docs.rs/paladin/latest/paladin/infrastructure/resilience/circuit_breaker/struct.CircuitBreaker.html) |
| `CircuitState` | `paladin::infrastructure::resilience::circuit_breaker::CircuitState` | 🟢 Stable | Circuit breaker state (`Closed`, `Open`, `HalfOpen`) | [Docs](https://docs.rs/paladin/latest/paladin/infrastructure/resilience/circuit_breaker/enum.CircuitState.html) |

---

## Internal Implementation Details (Not Stable)

The following are **internal implementation details** and NOT part of the stable public API. These may change without notice in minor versions.

### Adapters (Infrastructure Layer)

All concrete adapter implementations in `src/infrastructure/adapters/` are internal:

**LLM Adapters:**
- `OpenAIAdapter`, `DeepSeekAdapter`, `AnthropicAdapter` → Use `LlmPort` trait instead
- `OpenAIEmbeddingAdapter` → Use `EmbeddingPort` trait instead

**Storage Adapters:**
- `InMemoryGarrison`, `SqliteGarrison` → Use `GarrisonPort` trait instead
- `QdrantSanctum`, `InMemorySanctum` → Use `SanctumPort` trait instead
- `FileCitadel` → Use `CitadelPort` trait instead

**Queue Adapters:**
- `RedisQueue`, `InMemoryQueue` → Use `QueuePort` trait instead

**File Storage Adapters:**
- `MinIOAdapter`, `LocalFileAdapter` → Use `FileStoragePort` trait instead

**Arsenal Adapters:**
- `MCPStdioAdapter`, `MCPStreamableHttpAdapter` (the retired `MCPSseAdapter` was never
  actually SSE — a mislabeled plain-HTTP-POST adapter, removed in Phase 12.1 D-02b) →
  Use `ArsenalPort` trait instead

**Why Internal?** Adapter implementations are infrastructure concerns. Library users should depend on port traits to remain decoupled from specific technologies.

**Migration Path**: Replace direct adapter usage with port traits in library code. Adapters are acceptable in application code and examples.

### Repositories (Data Access Layer)

All repository implementations in `src/infrastructure/repositories/` are internal:
- MySQL repositories (`src/infrastructure/repositories/mysql/`)
- SQLite repositories (`src/infrastructure/repositories/sqlite/`)

**Why Internal?** Repositories are data access implementation details hidden behind port traits or use case services.

### Managers (Service Coordinators)

Internal service managers in `src/core/manager/` are not public API:
- `Scheduler` - Task scheduling coordinator
- `QueueService` - Queue management service
- `EventManager` - Event distribution service

**Why Internal?** Managers are internal service coordinators. Use port traits or use case services instead.

### CLI (Binary Interface)

All CLI-related modules in `src/application/cli/` are internal to the binary and not exposed as library API.

**Why Internal?** CLI is a binary-specific interface, not meant for library consumption.

### Web Server (HTTP Interface)

All web server modules in `src/infrastructure/web/` are internal to the binary.

**Why Internal?** Web server is a binary-specific deployment concern.

---

## API Change Process

This section defines the process for proposing, reviewing, and implementing changes to the stable public API.

### Step 1: Proposal

1. **Open GitHub Issue** with the `api-change` label
2. **Template Required** (use `.github/ISSUE_TEMPLATE/api-change.md`)
3. **Include**:
   - Type: Addition / Breaking Change / Deprecation / Clarification
   - Motivation: Why is this change needed?
   - Impact: What code will break?
   - Alternatives: What other approaches were considered?
   - Migration: How will users migrate?

### Step 2: Discussion

1. **Community Review Period**: Minimum 7 days for breaking changes
2. **Maintainer Approval**: At least one maintainer must approve
3. **RFC Process**: Major breaking changes may require an RFC document

### Step 3: Implementation

1. **Branch Creation**: Create feature branch from `main`
2. **Code Changes**:
   - Implement the proposed change
   - Update rustdoc for all affected items
   - Add examples demonstrating new usage
3. **API Baseline Update**:
   ```bash
   ./scripts/extract-public-api.sh project/current-exports.txt
   git add project/current-exports.txt
   ```
4. **Documentation Updates**:
   - Update `STABLE_API.md` (this file)
   - Update `CHANGELOG.md` with entry
   - Update `MIGRATION.md` if breaking change
5. **Tests**:
   - All existing tests must pass
   - Add tests for new functionality
   - Doc tests must compile and pass

### Step 4: Review

1. **Pull Request** with completed checklist
2. **CI Verification**: All checks must pass
3. **Code Review**: At least one approval from maintainer
4. **API Diff Review**: Carefully review `cargo-public-api` diff

### Step 5: Merge and Release

1. **Merge to main** after approval
2. **Version Bump** according to semver
3. **Publish** to crates.io
4. **Release Notes** on GitHub

### API Change Checklist

- [ ] GitHub issue created with `api-change` label
- [ ] Community discussion period completed (7+ days for breaking)
- [ ] Maintainer approval obtained
- [ ] Implementation complete with rustdoc
- [ ] Examples added/updated
- [ ] API baseline regenerated (`extract-public-api.sh`)
- [ ] `STABLE_API.md` updated (this file)
- [ ] `CHANGELOG.md` entry added
- [ ] `MIGRATION.md` updated (if breaking)
- [ ] All tests passing (unit, integration, doc)
- [ ] CI checks passing (including API surface verification)
- [ ] Pull request reviewed and approved
- [ ] Version bumped per semver
- [ ] Published to crates.io
- [ ] Release notes created on GitHub

---

## Migration Guide for Breaking Changes

When we make breaking changes in a major version bump, we will:

> **Current deprecation state (dated 2026-08-06):** The lifecycle policy below stands and governs
> any future deprecation. Milestone 4 Epic 2's requirement to deprecate the existing transitional
> API surface was withdrawn by ADR-0022 (`.planning/decisions/0022-deprecation-requirement-withdrawal.md`)
> on 2026-08-06 — the epic's own tracking document named no candidate for deprecation, so
> `grep -rn '#\[deprecated' src crates` returns **0** today, and that is the recorded outcome, not
> an unfinished task. Where the policy below references named-version anchors (`v0.2.0`, `v0.3.0`),
> note that ADR-0022 restates the removal window as **"at least one minor version"** rather than a
> release that has already shipped, per the pre-1.0 versioning posture recorded in ADR-0008
> (`.planning/decisions/0008-workspace-version-0-7-0.md`).
> The `#[deprecated]`-attribute example below illustrates the *form* a future deprecation will
> take — it is not a record of an active one.

### Deprecation Lifecycle

1. **Announcement** (Version N):
   - Add `#[deprecated(since = "N", note = "use X instead")]` attribute
   - Update rustdoc with migration guidance
   - Add entry to CHANGELOG.md
   - Update MIGRATION.md with examples

2. **Support Period** (Version N through N+1):
   - Deprecated API remains functional
   - Compiler warnings guide users to alternatives
   - Documentation shows both old and new approaches

3. **Removal** (Version N+2):
   - Deprecated API removed in next major version
   - CHANGELOG.md documents removal
   - MIGRATION.md provides upgrade path

### Deprecation Example

```rust,ignore
// Version 0.5.0 - Original API
pub fn execute_paladin(paladin: &Paladin) -> Result<String, Error> {
    // ...
}

// Version 0.2.0 - Add new API, deprecate old
#[deprecated(since = "0.2.0", note = "use `PaladinPort::execute()` instead")]
pub fn execute_paladin(paladin: &Paladin) -> Result<String, Error> {
    // Old implementation still works
}

pub trait PaladinPort {
    fn execute(&self, paladin: &Paladin) -> Result<PaladinResult, PaladinError>;
}

// Version 1.0.0 - Remove deprecated API
// execute_paladin() function no longer exists
// Users must use PaladinPort::execute()
```

### Migration Resources

- **MIGRATION.md**: Step-by-step upgrade guides for each major version
- **CHANGELOG.md**: Detailed list of breaking changes
- **Release Notes**: Migration highlights on GitHub releases
- **Examples**: Updated examples in `examples/` directory
- **Documentation**: Rustdoc updated with new patterns

### Compatibility Shims

When possible, we provide compatibility shims during the deprecation period:

```rust,ignore
// Compatibility shim example
#[deprecated(since = "0.2.0", note = "use PaladinBuilder instead")]
pub fn create_paladin(name: &str, model: &str) -> Paladin {
    PaladinBuilder::new()
        .name(name)
        .model(model)
        .build()
        .expect("Failed to build Paladin")
}
```

### Version Upgrade Paths

- **0.1.x → 0.2.x**: TBD (no breaking changes yet)
- **0.x.y → 1.0.0**: Will be documented before 1.0.0 release

---

## Questions and Support

For questions about API stability:

### GitHub Issues

- **API Questions**: Open issue with `question` label
- **API Change Proposals**: Use `api-change` label
- **Bug Reports**: Use `bug` label
- **Feature Requests**: Use `enhancement` label

### Discussion Forums

- **GitHub Discussions**: [paladin-dev-env/discussions](https://github.com/DF3NDR/paladin-dev-env/discussions)
- **Topic Categories**:
  - General Questions
  - API Design
  - Migration Help
  - Show and Tell

### Maintainers

- **Primary Maintainer**: @DF3NDR
- **Response Time**: Typically within 48 hours for critical issues

### Related Documentation

- **[API Reference](stable-api.md)** - Current stable API surface
- **[CHANGELOG](https://github.com/DF3NDR/paladin-dev-env/blob/main/CHANGELOG.md)** - Version history and breaking changes
- **[Migration Guide](migration-guide.md)** - Migration guides between versions
- **[Contributing Guide](../contributing/development-setup.md)** - Contribution guidelines including API change process
- **[Deprecations Tracking](https://github.com/DF3NDR/paladin-dev-env/blob/main/CHANGELOG.md)** - The deprecation process is documented here; no deprecation is currently active (see ADR-0022, `.planning/decisions/0022-deprecation-requirement-withdrawal.md`)

### Documentation Links

- **Crate Documentation**: [docs.rs/paladin](https://docs.rs/paladin)
- **User Guides**: [User Guides](../user-guides/paladin-agents.md)
- **Architecture**: [Architecture Overview](../architecture/overview.md)
- **Examples**: [examples/](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples)

---

**Last Updated**: 2026-04-16
**Document Version**: 1.1
**Paladin Version: 0.5.0
**Maintainers**: @DF3NDR

---

## Versioning Policy

## Purpose

This document defines how Paladin versions its workspace crates and what constitutes a breaking change.

## Initial Versioning Strategy

Paladin uses lockstep versioning for the initial release line.

- Scope: all public crates in this workspace.
- Current baseline: 0.5.0.
- Milestone 7 target: 0.2.0 lockstep for publishable crates.
- Rule: a single release version is applied to all public crates in the same release cycle.

Public crates:

- paladin
- paladin-core
- paladin-ports
- paladin-battalion
- paladin-llm
- paladin-memory
- paladin-web
- paladin-notifications
- paladin-content
- paladin-storage

## Breaking Change Policy

Breaking changes require a coordinated lockstep release increment.

Examples of breaking changes:

- Removing or renaming a public type, trait, function, enum variant, or module path.
- Changing function signatures in a way that breaks callers.
- Changing trait method signatures or required methods.
- Changing feature flag semantics in a way that breaks existing consumers.
- Tightening configuration requirements without backward-compatible defaults.

Non-breaking changes:

- Additive APIs (new types, functions, optional feature flags).
- Internal refactoring that preserves public API behavior and signatures.
- Documentation-only improvements.

## Crate-Family Guidance

- paladin-core: domain model compatibility is high impact; treat model shape changes as potentially breaking.
- paladin-ports: trait contracts are compatibility-critical; changes are usually breaking.
- paladin-battalion: orchestration runtime APIs and strategy entrypoints should remain stable.
- paladin-llm: provider additions are additive; request/response contract changes may be breaking.
- paladin-memory: storage adapter behavior and query API changes may be breaking.
- paladin-web: externally consumed handler/middleware APIs should preserve compatibility.
- paladin-notifications: adapter trait behavior and config contracts should remain stable.
- paladin-content: use-case and adapter public APIs should preserve call signatures.
- paladin-storage: repository and migration public APIs should preserve compatibility.
- paladin facade: re-export paths and top-level developer ergonomics are compatibility-critical.

## Transition Criteria for Independent Versioning

Paladin may transition from lockstep to independent crate versioning after all criteria below are met:

- Stable dependency graph with low cross-crate churn across at least 2-3 release cycles.
- Per-crate changelog discipline is consistently maintained.
- Public API stability tiers are fully documented and regularly reviewed.
- CI pipeline supports dependency-aware, per-crate release automation.
- Release owners agree that independent cadence adds value without excessive coordination cost.

Until then, lockstep versioning remains the default policy.

## Dependency-Aware Publish Order

Use dependency-first publishing in this order:

1. paladin-core
2. paladin-ports
3. Leaf crates (paladin-battalion, paladin-llm, paladin-memory, paladin-web, paladin-notifications, paladin-content, paladin-storage)
4. paladin facade crate

This order is required because dry-run and publish validation for dependent crates requires published upstream dependencies.
