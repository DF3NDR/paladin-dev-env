//! [`Transience`] classifies whether a node-execution failure is worth
//! retrying (Doc 04 FT-FR-02, D-01).

use serde::{Deserialize, Serialize};

/// Whether a node-execution failure is transient (retrying has a reasonable
/// chance of succeeding), permanent (retrying would fail identically), or of
/// unknown transience (the classifier could not tell).
///
/// Deliberately three-valued and deliberately **not** `#[non_exhaustive]`
/// (D-01): every `RetryPredicate` arm in
/// [`crate::platform::container::aegis`] matches these three values
/// exhaustively, by design. A fourth value here would be a taxonomy change
/// to the whole fault-tolerance model -- something every existing
/// `RetryPredicate`/classifier match site would need to be revisited for --
/// not an ordinary additive change a downstream crate should silently
/// absorb via a wildcard arm. Keeping this exhaustive means the compiler
/// forces every match site to be revisited the day a fourth value is ever
/// proposed, rather than letting it compile silently with the new value
/// falling into an unrelated `_` arm.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::transience::Transience;
///
/// let classified = Transience::Transient;
/// assert_eq!(classified, Transience::Transient);
/// assert_ne!(classified, Transience::Permanent);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Transience {
    /// The failure is likely transient -- a network blip, a rate limit, a
    /// timeout -- so retrying the same operation again has a reasonable
    /// chance of succeeding.
    Transient,
    /// The failure is permanent: retrying the same operation again would
    /// fail identically (a validation error, an auth failure, a bug).
    Permanent,
    /// The failure's transience could not be classified confidently.
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_values_are_pairwise_distinct() {
        assert_ne!(Transience::Transient, Transience::Permanent);
        assert_ne!(Transience::Transient, Transience::Unknown);
        assert_ne!(Transience::Permanent, Transience::Unknown);
    }

    #[test]
    fn is_copy_and_hash() {
        use std::collections::HashSet;
        let a = Transience::Transient;
        let b = a; // Copy, not a move.
        let mut set: HashSet<Transience> = HashSet::new();
        set.insert(a);
        set.insert(b);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn round_trips_through_serde_json() {
        for value in [
            Transience::Transient,
            Transience::Permanent,
            Transience::Unknown,
        ] {
            let json = serde_json::to_string(&value).expect("serialize");
            let back: Transience = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(value, back);
        }
    }
}
