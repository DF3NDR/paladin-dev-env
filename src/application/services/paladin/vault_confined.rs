//! Re-export shim for [`ConfinedVault`] (Doc 05 RT-04, D-20/D-21).
//!
//! The real implementation lives in
//! [`paladin_ports::output::vault_confined`], not here — see that module's
//! own rustdoc for why. This shim exists purely for source compatibility
//! with this crate's own `application::services::paladin::vault_confined`
//! path (and the facade prelude's re-export), so code written against the
//! path this plan originally sketched keeps compiling.

pub use paladin_ports::output::vault_confined::ConfinedVault;
