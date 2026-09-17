# Port Trait Documentation Template

This template defines the standard rustdoc structure for all Port Traits in the Paladin framework. Following this template ensures consistency, completeness, and professional-grade API documentation.

---

## Structure Overview

```rust,ignore
//! # Port Name
//!
//! Brief one-sentence description of the port's purpose.
//!
//! ## Purpose
//!
//! Detailed explanation of:
//! - What problem this port solves
//! - When to use this port vs alternatives
//! - How it fits into the hexagonal architecture
//!
//! ## Hexagonal Architecture
//!
//! This port is an **output port** (or **input port**) in the application layer.
//! It defines the interface for [specific domain operation], allowing the core
//! domain logic to remain independent of infrastructure concerns.
//!
//! **Adapter Implementations:**
//! - `AdapterName1` - Description of when to use
//! - `AdapterName2` - Description of when to use
//!
//! ## Thread Safety
//!
//! All implementations must be `Send + Sync` to support concurrent async operations.
//! Methods may be called from multiple tasks simultaneously.
//!
//! ## Error Handling
//!
//! Operations return `Result<T, ErrorType>` where:
//! - `ErrorType` is defined in this module
//! - Errors should be recoverable where possible
//! - See [`ErrorType`] documentation for error categories
//!
//! ## Examples
//!
//! ### Basic Usage
//!
//! ```rust
//! use paladin_ports::output::port_name::PortTrait;
//!
//! async fn example(port: &dyn PortTrait) -> Result<(), Box<dyn std::error::Error>> {
//!     // Example showing the most common use case
//!     let result = port.method(args).await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Custom Implementation
//!
//! ```rust
//! use paladin_ports::output::port_name::{PortTrait, ErrorType};
//! use async_trait::async_trait;
//!
//! struct CustomAdapter {
//!     // Adapter-specific fields
//! }
//!
//! #[async_trait]
//! impl PortTrait for CustomAdapter {
//!     async fn method(&self, args: Type) -> Result<ReturnType, ErrorType> {
//!         // Custom implementation
//!         Ok(result)
//!     }
//! }
//! ```
//!
//! ### Advanced Usage
//!
//! ```rust
//! // Example showing more complex scenarios:
//! // - Error handling patterns
//! // - Composing with other ports
//! // - Performance considerations
//! ```
//!
//! ## Implementation Notes
//!
//! ### Performance Considerations
//! - Describe any performance characteristics
//! - Recommended batch sizes
//! - Caching strategies
//!
//! ### Best Practices
//! - How to implement this port correctly
//! - Common pitfalls to avoid
//! - Testing recommendations
//!
//! ## Related Ports
//!
//! - [`RelatedPort1`] - How it relates
//! - [`RelatedPort2`] - How it relates
//!
//! ## See Also
//!
//! - [Module documentation](crate::application::ports)
//! - [Architecture guide](../../docs/Design/Design_and_Architecture.md)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Errors that can occur during [operation] operations
///
/// Each variant represents a specific failure mode with detailed context.
/// All errors implement `std::error::Error` via `thiserror`.
#[derive(Debug, Error)]
pub enum ErrorType {
    /// Brief description of when this error occurs
    ///
    /// # Examples
    ///
    /// ```
    /// // Example showing when this error is returned
    /// ```
    #[error("User-friendly error message: {0}")]
    VariantName(String),

    /// Another error variant with documentation
    #[error("Error message")]
    AnotherVariant,
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/// Request type for [operation]
///
/// Describe the structure and its purpose.
///
/// # Fields
///
/// - `field1`: Description and constraints
/// - `field2`: Description and valid values
///
/// # Examples
///
/// ```
/// use paladin_ports::output::port_name::RequestType;
///
/// let request = RequestType {
///     field1: value,
///     field2: value,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestType {
    /// Field documentation with constraints
    pub field1: Type,

    /// Another field with detailed docs
    pub field2: Type,
}

/// Response type for [operation]
///
/// Describe what information is returned and its significance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseType {
    /// Field documentation
    pub field1: Type,
}

// ============================================================================
// PORT TRAIT
// ============================================================================

/// Port trait for [domain operation]
///
/// This trait defines the core interface for [what it does]. All implementations
/// must provide these operations.
///
/// # Async Model
///
/// All methods are async to support non-blocking I/O. Implementations should
/// use `tokio` or compatible runtime.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`. Methods may be called concurrently
/// from multiple tasks.
///
/// # Lifecycle
///
/// Describe any initialization, cleanup, or state management requirements.
///
/// # Examples
///
/// See [module-level documentation](self) for complete examples.
#[async_trait]
pub trait PortTrait: Send + Sync {
    /// Brief one-line description of method
    ///
    /// Detailed description of:
    /// - What the method does
    /// - When to use it
    /// - What happens internally
    ///
    /// # Parameters
    ///
    /// - `param1`: Description, constraints, valid values
    /// - `param2`: Description and purpose
    ///
    /// # Returns
    ///
    /// Returns `Result<ReturnType, ErrorType>` where:
    /// - `Ok(value)` on success - describe what value represents
    /// - `Err(error)` on failure - list specific error variants
    ///
    /// # Errors
    ///
    /// - [`ErrorType::Variant1`] - When this specific error occurs
    /// - [`ErrorType::Variant2`] - When this specific error occurs
    ///
    /// # Thread Safety
    ///
    /// This method is safe to call concurrently from multiple tasks.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_ports::output::port_name::PortTrait;
    ///
    /// async fn example(port: &dyn PortTrait) -> Result<(), Box<dyn std::error::Error>> {
    ///     let result = port.method_name(args).await?;
    ///     // Use result
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Implementation Notes
    ///
    /// Guidance for implementers:
    /// - Performance characteristics
    /// - Edge cases to handle
    /// - Testing recommendations
    async fn method_name(&self, param1: Type, param2: Type) -> Result<ReturnType, ErrorType>;
}

// ============================================================================
// HELPER TYPES & UTILITIES
// ============================================================================

/// Helper type or utility struct with full documentation
///
/// Describe its purpose and relationship to the port.
#[derive(Debug, Clone)]
pub struct HelperType {
    /// Field documentation
    pub field: Type,
}
```

---

## Checklist for Each Port Trait

- [ ] **Module-level documentation** (`//!`)
  - [ ] Brief one-sentence summary
  - [ ] Purpose section (2-3 paragraphs)
  - [ ] Hexagonal architecture explanation
  - [ ] Thread safety notes
  - [ ] Error handling overview
  - [ ] At least 2 examples (basic + custom implementation)
  - [ ] Implementation notes section
  - [ ] Related ports with intra-doc links

- [ ] **Error type documentation**
  - [ ] Each variant documented
  - [ ] When each error occurs
  - [ ] Example triggering each error (if applicable)

- [ ] **Request/Response types**
  - [ ] Struct purpose documented
  - [ ] Each field documented with constraints
  - [ ] Usage example for complex types

- [ ] **Trait documentation**
  - [ ] Trait purpose and responsibilities
  - [ ] Async model explanation
  - [ ] Thread safety guarantees
  - [ ] Lifecycle notes (if applicable)

- [ ] **Method documentation**
  - [ ] Brief description
  - [ ] Detailed behavior explanation
  - [ ] Parameters section with constraints
  - [ ] Returns section with success/error cases
  - [ ] Errors section listing specific variants
  - [ ] Thread safety notes
  - [ ] At least 1 usage example
  - [ ] Implementation notes for complex methods

- [ ] **Cross-references**
  - [ ] Links to related ports
  - [ ] Links to related domain types
  - [ ] Links to implementation examples

- [ ] **Code examples compile**
  - [ ] All examples use valid imports
  - [ ] Examples demonstrate actual usage
  - [ ] Examples are tested via `cargo test --doc`

---

## Documentation Quality Standards

### Language & Tone
- Use clear, concise language
- Write in present tense
- Use active voice
- Avoid jargon unless defined
- Assume reader understands Rust but not the domain

### Content Requirements
- Explain "why" not just "what"
- Provide context for design decisions
- Include when NOT to use something
- Anticipate questions and answer them
- Give concrete examples

### Code Examples
- Keep examples focused and minimal
- Show real-world usage patterns
- Include error handling
- Use descriptive variable names
- Add comments explaining non-obvious steps

### Formatting
- Use proper rustdoc markdown
- Use intra-doc links for types: \[`TypeName`\]
- Use section headers: `# Section`
- Use bullet lists for multiple items
- Use code blocks with language hints: \`\`\`rust

---

## Testing Documentation

All code examples must compile:

```bash
# Test all doc examples
cargo test --doc --all-features

# Test specific module's docs
cargo test --doc --package paladin --lib paladin_ports::output::llm_port
```

---

## References

- [Rust API Guidelines - Documentation](https://rust-lang.github.io/api-guidelines/documentation.html)
- [rustdoc Book](https://doc.rust-lang.org/rustdoc/)
- [How to Write Documentation](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)
