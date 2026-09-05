//! Parley Port — Triggering a Resume Without Naming the Engine (HITL-05, D-25)
//!
//! RED state (Phase 24 Plan 10, Task 1): tests reference `ParleyPort`,
//! `ResumeAccepted` and `ParleyError`, which do not exist yet. See the
//! GREEN commit that follows for the full implementation and rustdoc.

use chrono::Utc;

use paladin_core::platform::container::parley::ParleyId;
use paladin_core::platform::container::waypoint::{GraphFingerprint, ThreadId};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn parley_port_is_object_safe() {
        let _: Option<Arc<dyn ParleyPort>> = None;
    }

    fn sample_parley_id() -> ParleyId {
        ParleyId::new()
    }

    #[test]
    fn parley_error_covers_every_validation_case() {
        fn label(err: &ParleyError) -> &'static str {
            match err {
                ParleyError::ThreadNotFound(_) => "thread_not_found",
                ParleyError::GraphNotRegistered { .. } => "graph_not_registered",
                ParleyError::ThreadNotAwaitingInput { .. } => "thread_not_awaiting_input",
                ParleyError::UnknownParleyId { .. } => "unknown_parley_id",
                ParleyError::ParleyAlreadyAnswered { .. } => "parley_already_answered",
                ParleyError::ResponseShapeInvalid { .. } => "response_shape_invalid",
                ParleyError::ParleyExpired { .. } => "parley_expired",
            }
        }

        let thread = ThreadId::new("t1").unwrap();
        let parley_id = sample_parley_id();
        let cases = vec![
            (
                ParleyError::ThreadNotFound(thread.clone()),
                "thread_not_found",
            ),
            (
                ParleyError::GraphNotRegistered {
                    fingerprint: GraphFingerprint::from_canonical_bytes(b"g1"),
                },
                "graph_not_registered",
            ),
            (
                ParleyError::ThreadNotAwaitingInput {
                    thread: thread.clone(),
                    status: "Running".to_string(),
                },
                "thread_not_awaiting_input",
            ),
            (
                ParleyError::UnknownParleyId { parley_id },
                "unknown_parley_id",
            ),
            (
                ParleyError::ParleyAlreadyAnswered { parley_id },
                "parley_already_answered",
            ),
            (
                ParleyError::ResponseShapeInvalid {
                    parley_id,
                    reason: "bad shape".to_string(),
                },
                "response_shape_invalid",
            ),
            (
                ParleyError::ParleyExpired {
                    parley_id,
                    expires_at: Utc::now(),
                },
                "parley_expired",
            ),
        ];

        for (err, expected) in &cases {
            assert_eq!(label(err), *expected);
        }
        assert_eq!(cases.len(), 7, "every ParleyError variant must be covered");
    }

    #[test]
    fn parley_error_display_names_the_parley_id() {
        let parley_id = sample_parley_id();

        let unknown = ParleyError::UnknownParleyId { parley_id };
        assert!(unknown.to_string().contains(&parley_id.to_string()));

        let already = ParleyError::ParleyAlreadyAnswered { parley_id };
        assert!(already.to_string().contains(&parley_id.to_string()));

        let shape_invalid = ParleyError::ResponseShapeInvalid {
            parley_id,
            reason: "must be a string".to_string(),
        };
        assert!(shape_invalid.to_string().contains(&parley_id.to_string()));
        assert!(shape_invalid.to_string().contains("must be a string"));

        let expired = ParleyError::ParleyExpired {
            parley_id,
            expires_at: Utc::now(),
        };
        assert!(expired.to_string().contains(&parley_id.to_string()));
    }

    #[test]
    fn resume_accepted_carries_thread_and_state_handle() {
        let thread = ThreadId::new("resume-accepted-thread").unwrap();
        let accepted = ResumeAccepted::new(thread.clone());
        assert_eq!(accepted.thread_id(), &thread);
        assert_eq!(accepted.state_handle(), &thread);
    }
}
