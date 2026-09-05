//! The progress-report handle a node execution beats to prove it is still
//! making progress (Doc 04 FT-FR-09, D-18, D-19; plan 25-09).
//!
//! [`HeartbeatHandle`] lives in `paladin-core` -- beside every other port
//! value type (ADR-0016) -- because BOTH the port that reports progress
//! (`paladin_ports::output::paladin_port::PaladinPort::execute_observed`)
//! and the engine that watches for it (`paladin_battalion::engine`'s idle
//! timer) must name the same type, and `paladin-ports` can never depend on
//! `paladin-battalion`. `paladin_battalion::engine::heartbeat` re-exports
//! it under the engine's own path; there is exactly one definition.
//!
//! # Why a `tokio::sync::watch` sender (and not an atomic timestamp)
//!
//! RESEARCH.md weighed two internals: an `Arc<AtomicU64>` of the last beat's
//! nanos, which an idle timer would have to POLL, versus an `Arc`-shared
//! `watch::Sender`, which gives the timer a `changed()` future to AWAIT.
//! The watch primitive wins because it makes the idle timer a plain
//! `tokio::time::timeout(idle, rx.changed())` loop -- no polling interval to
//! tune, no busy loop, and under `tokio::time::pause` the timer is driven
//! purely by the virtual clock, so FT-FR-09's "chunk every 100 ms survives,
//! stall 300 ms fails" test is deterministic. The value carried is a plain
//! beat COUNTER (not an `Instant`), so the handle is clock-agnostic and
//! `beats()` doubles as the observation surface a test asserts on.
//!
//! # The derive constraint
//!
//! `paladin_battalion::engine::node::NodeContext` derives `Debug, Clone,
//! PartialEq`, and that derive set is load-bearing. A `watch::Sender` is
//! neither `PartialEq` nor usefully `Debug`, so this newtype supplies a
//! MANUAL `PartialEq` under which any two handles compare equal (a handle
//! is an opaque progress channel, never part of a context's identity) and
//! a manual `Debug` that prints an opaque placeholder.

use std::fmt;
use std::sync::Arc;

use tokio::sync::watch;

/// An `Arc`-shared progress channel a node execution beats to reset its
/// `TimeoutPolicy::idle_timeout` (D-18, D-19).
///
/// Cheap to clone (one `Arc` bump), cheap to beat (one `watch::Sender::send_modify`
/// -- a no-op in effect when nothing is subscribed, which is exactly the
/// case for a node with no `idle_timeout`), and safe to beat from any task.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::heartbeat::HeartbeatHandle;
///
/// let handle = HeartbeatHandle::new();
/// assert_eq!(handle.beats(), 0);
///
/// let shared = handle.clone();
/// shared.beat();
/// shared.beat();
/// assert_eq!(handle.beats(), 2, "clones share one counter");
///
/// // Any two handles compare equal: the handle is never part of identity.
/// assert_eq!(handle, HeartbeatHandle::new());
/// ```
#[derive(Clone)]
pub struct HeartbeatHandle {
    inner: Arc<watch::Sender<u64>>,
}

impl HeartbeatHandle {
    /// A fresh handle with zero beats and no subscriber.
    pub fn new() -> Self {
        let (sender, _receiver) = watch::channel(0u64);
        Self {
            inner: Arc::new(sender),
        }
    }

    /// Report progress: increments the beat counter and wakes every
    /// subscribed idle timer. Never fails and never blocks -- with no
    /// subscriber (a node without an `idle_timeout`) this is a cheap
    /// counter bump and nothing more.
    pub fn beat(&self) {
        self.inner
            .send_modify(|beats| *beats = beats.wrapping_add(1));
    }

    /// How many times [`HeartbeatHandle::beat`] has been called on this
    /// handle (or any clone of it) so far.
    pub fn beats(&self) -> u64 {
        *self.inner.borrow()
    }

    /// Subscribe to future beats. The returned receiver's `changed()`
    /// future resolves on the NEXT beat after this call (the current count
    /// is marked seen), which is exactly what an idle timer needs: "reset
    /// me when progress is observed", never "was there ever progress".
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.inner.subscribe()
    }
}

impl Default for HeartbeatHandle {
    fn default() -> Self {
        Self::new()
    }
}

// Any two handles compare equal (D-18): a `HeartbeatHandle` is an opaque
// progress channel, never a value that distinguishes one `NodeContext`
// from another. This is what keeps `NodeContext: PartialEq` derivable.
impl PartialEq for HeartbeatHandle {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for HeartbeatHandle {}

// An opaque placeholder: the channel's internals are not meaningful to a
// reader, and printing a `watch::Sender` would expose nothing useful
// anyway. Keeps `NodeContext: Debug` derivable.
impl fmt::Debug for HeartbeatHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HeartbeatHandle(..)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beats_are_counted_across_clones() {
        let handle = HeartbeatHandle::new();
        let clone = handle.clone();
        clone.beat();
        handle.beat();
        assert_eq!(handle.beats(), 2);
        assert_eq!(clone.beats(), 2);
    }

    #[test]
    fn handles_always_compare_equal_and_debug_is_opaque() {
        let a = HeartbeatHandle::new();
        let b = HeartbeatHandle::new();
        b.beat();
        assert_eq!(a, b);
        assert_eq!(format!("{a:?}"), "HeartbeatHandle(..)");
    }

    #[tokio::test]
    async fn a_subscriber_wakes_on_the_next_beat_only() {
        let handle = HeartbeatHandle::new();
        handle.beat();
        let mut rx = handle.subscribe();
        // The beat BEFORE subscribing is already seen: `changed()` must not
        // resolve until a later beat.
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), rx.changed())
                .await
                .is_err(),
            "no pending change right after subscribing"
        );
        handle.beat();
        assert!(
            rx.changed().await.is_ok(),
            "the next beat wakes the subscriber"
        );
        assert_eq!(*rx.borrow_and_update(), 2);
    }
}
