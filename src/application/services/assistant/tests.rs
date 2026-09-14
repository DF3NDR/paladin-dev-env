//! Assistant module test suite (PLAT-04): validator cases, service
//! create/version/list/delete, immutability, and the freeze-at-submit proof
//! end-to-end through [`super::resolver::StoredAssistantResolver`] and
//! `services::run::submission::RunSubmissionService`.

use std::sync::Arc;

use paladin_battalion::engine::registries::EngineRegistries;
use paladin_core::platform::container::assistant::{
    AssistantDefinition, AssistantId, AssistantKind,
};
use paladin_ports::input::assistant_admin_port::{
    AssistantAdminError, AssistantAdminPort, PublishAssistant,
};
use paladin_storage::assistant::in_memory::InMemoryAssistantRepository;

use super::resolver::{ChainedResolver, StoredAssistantResolver};
use super::service::AssistantService;
use super::validator::{AssistantValidator, Validated};

fn validator() -> AssistantValidator {
    AssistantValidator::new(Arc::new(EngineRegistries::new()))
}

fn valid_agent_body() -> serde_json::Value {
    serde_json::json!({
        "name": "Researcher",
        "model": "gpt-4",
        "system_prompt": "You research topics.",
        "temperature": 0.5,
        "stop_words": ["STOP"],
    })
}

fn valid_workflow_body() -> serde_json::Value {
    serde_json::json!({
        "schema_version": "1",
        "entry": ["review"],
        "nodes": [{
            "id": "review",
            "kind": "gate",
            "gate": {
                "parley": "approval",
                "prompt_template": "Approve?",
                "on_expire": { "type": "fail_run" },
                "output_field": "approved"
            },
            "defer": false
        }],
        "edges": [{ "from": "review", "to": "review" }],
        "schema": {
            "fields": [{
                "name": "approved",
                "kind": "boolean",
                "reducer": "last_write",
                "default": false,
                "required": false
            }]
        }
    })
}

// --- AssistantValidator --------------------------------------------------

#[test]
fn empty_agent_body_is_missing_field() {
    let err = validator()
        .validate(&AssistantDefinition {
            kind: AssistantKind::Agent,
            body: serde_json::json!({}),
        })
        .unwrap_err();
    assert!(
        err.iter()
            .any(|v| v.path == "/" && v.code == "missing_field")
    );
}

#[test]
fn null_definition_body_is_missing_field() {
    let err = validator()
        .validate(&AssistantDefinition {
            kind: AssistantKind::Workflow,
            body: serde_json::Value::Null,
        })
        .unwrap_err();
    assert!(err.iter().any(|v| v.code == "missing_field"));
}

#[test]
fn agent_with_empty_system_prompt_is_rejected() {
    let mut body = valid_agent_body();
    body["system_prompt"] = serde_json::json!("");
    let err = validator()
        .validate(&AssistantDefinition {
            kind: AssistantKind::Agent,
            body,
        })
        .unwrap_err();
    assert!(
        err.iter()
            .any(|v| v.path == "/system_prompt" && v.code == "empty")
    );
}

#[test]
fn agent_with_empty_name_is_rejected() {
    let mut body = valid_agent_body();
    body["name"] = serde_json::json!("  ");
    let err = validator()
        .validate(&AssistantDefinition {
            kind: AssistantKind::Agent,
            body,
        })
        .unwrap_err();
    assert!(err.iter().any(|v| v.path == "/name" && v.code == "empty"));
}

#[test]
fn agent_with_out_of_range_temperature_is_rejected() {
    let mut body = valid_agent_body();
    body["temperature"] = serde_json::json!(3.5);
    let err = validator()
        .validate(&AssistantDefinition {
            kind: AssistantKind::Agent,
            body,
        })
        .unwrap_err();
    assert!(
        err.iter()
            .any(|v| v.path == "/temperature" && v.code == "out_of_range")
    );
}

#[test]
fn agent_with_zero_timeout_is_rejected() {
    let mut body = valid_agent_body();
    body["timeout_seconds"] = serde_json::json!(0);
    let err = validator()
        .validate(&AssistantDefinition {
            kind: AssistantKind::Agent,
            body,
        })
        .unwrap_err();
    assert!(
        err.iter()
            .any(|v| v.path == "/timeout_seconds" && v.code == "out_of_range")
    );
}

#[test]
fn agent_body_with_credential_field_at_any_depth_is_forbidden() {
    let mut body = valid_agent_body();
    body["nested"] = serde_json::json!({ "auth": { "api_key": "sk-secret" } });
    let err = validator()
        .validate(&AssistantDefinition {
            kind: AssistantKind::Agent,
            body,
        })
        .unwrap_err();
    assert!(
        err.iter().any(|v| v.code == "credential_field_forbidden"),
        "expected a credential_field_forbidden violation, got {err:?}"
    );
}

#[test]
fn agent_body_with_nonempty_tools_or_middleware_is_unsupported_in_v0_10() {
    let mut body = valid_agent_body();
    body["tools"] = serde_json::json!(["search"]);
    let err = validator()
        .validate(&AssistantDefinition {
            kind: AssistantKind::Agent,
            body,
        })
        .unwrap_err();
    assert!(
        err.iter()
            .any(|v| v.path == "/tools" && v.code == "unsupported_in_v0_10")
    );
}

#[test]
fn valid_agent_definition_yields_a_runnable_paladin() {
    let result = validator()
        .validate(&AssistantDefinition {
            kind: AssistantKind::Agent,
            body: valid_agent_body(),
        })
        .unwrap();
    assert!(matches!(result, Validated::Agent(_, _)));
}

#[test]
fn valid_workflow_definition_compiles() {
    let result = validator()
        .validate(&AssistantDefinition {
            kind: AssistantKind::Workflow,
            body: valid_workflow_body(),
        })
        .unwrap();
    assert!(matches!(result, Validated::Workflow(_)));
}

#[test]
fn workflow_with_unregistered_edge_evaluator_is_rejected() {
    let mut body = valid_workflow_body();
    body["edges"][0]["condition"] = serde_json::json!({ "type": "custom", "name": "nope" });
    let err = validator()
        .validate(&AssistantDefinition {
            kind: AssistantKind::Workflow,
            body,
        })
        .unwrap_err();
    assert!(
        err.iter().any(|v| v.code == "unregistered_edge_evaluator"),
        "expected unregistered_edge_evaluator, got {err:?}"
    );
}

#[test]
fn workflow_body_with_credential_field_is_forbidden() {
    let mut body = valid_workflow_body();
    body["secret_stash"] = serde_json::json!({ "token": "sk-secret" });
    let err = validator()
        .validate(&AssistantDefinition {
            kind: AssistantKind::Workflow,
            body,
        })
        .unwrap_err();
    assert!(err.iter().any(|v| v.code == "credential_field_forbidden"));
}

// --- AssistantService (create/version/list/delete, D-31, D-29) ----------

fn service() -> (AssistantService, Arc<InMemoryAssistantRepository>) {
    let repository = Arc::new(InMemoryAssistantRepository::new());
    let service = AssistantService::new(
        Arc::clone(&repository)
            as Arc<dyn paladin_ports::output::assistant_repository_port::AssistantRepositoryPort>,
        Arc::new(validator()),
    );
    (service, repository)
}

#[tokio::test]
async fn service_create_persists_a_valid_definition() {
    let (service, _repository) = service();
    let id = AssistantId::new("svc-a").unwrap();
    let version = service
        .create(
            &id,
            PublishAssistant {
                definition: AssistantDefinition {
                    kind: AssistantKind::Agent,
                    body: valid_agent_body(),
                },
                created_by: Some("alice".to_string()),
                note: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(version.version, 1);

    let fetched = service.get(&id).await.unwrap().unwrap();
    assert_eq!(fetched.latest, 1);
}

#[tokio::test]
async fn service_create_with_invalid_definition_persists_nothing() {
    let (service, _repository) = service();
    let id = AssistantId::new("svc-invalid").unwrap();
    let err = service
        .create(
            &id,
            PublishAssistant {
                definition: AssistantDefinition {
                    kind: AssistantKind::Agent,
                    body: serde_json::json!({}),
                },
                created_by: None,
                note: None,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, AssistantAdminError::Invalid { violations } if !violations.is_empty()));
    assert!(
        service.get(&id).await.unwrap().is_none(),
        "nothing must be persisted"
    );
}

#[tokio::test]
async fn service_publish_version_appends_and_lists_ascending() {
    let (service, _repository) = service();
    let id = AssistantId::new("svc-versions").unwrap();
    service
        .create(
            &id,
            PublishAssistant {
                definition: AssistantDefinition {
                    kind: AssistantKind::Workflow,
                    body: valid_workflow_body(),
                },
                created_by: None,
                note: None,
            },
        )
        .await
        .unwrap();
    let v2 = service
        .publish_version(
            &id,
            PublishAssistant {
                definition: AssistantDefinition {
                    kind: AssistantKind::Workflow,
                    body: valid_workflow_body(),
                },
                created_by: None,
                note: Some("second cut".to_string()),
            },
        )
        .await
        .unwrap();
    assert_eq!(v2.version, 2);

    let page = service.list_versions(&id, 10, None).await.unwrap();
    assert_eq!(
        page.items.iter().map(|v| v.version).collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[tokio::test]
async fn service_list_orders_assistants_ascending_by_id() {
    let (service, _repository) = service();
    for id in ["svc-b", "svc-a", "svc-c"] {
        service
            .create(
                &AssistantId::new(id).unwrap(),
                PublishAssistant {
                    definition: AssistantDefinition {
                        kind: AssistantKind::Agent,
                        body: valid_agent_body(),
                    },
                    created_by: None,
                    note: None,
                },
            )
            .await
            .unwrap();
    }
    let page = service.list(10, None, false).await.unwrap();
    let ids: Vec<String> = page
        .items
        .iter()
        .map(|a| a.assistant_id.to_string())
        .collect();
    assert_eq!(ids, vec!["svc-a", "svc-b", "svc-c"]);
}

#[tokio::test]
async fn service_delete_soft_deletes_but_versions_stay_readable() {
    let (service, _repository) = service();
    let id = AssistantId::new("svc-delete").unwrap();
    service
        .create(
            &id,
            PublishAssistant {
                definition: AssistantDefinition {
                    kind: AssistantKind::Agent,
                    body: valid_agent_body(),
                },
                created_by: None,
                note: None,
            },
        )
        .await
        .unwrap();

    service.delete(&id).await.unwrap();

    let assistant = service.get(&id).await.unwrap().unwrap();
    assert!(assistant.is_deleted());
    assert!(service.get_version(&id, 1).await.unwrap().is_some());

    // publish_version on a soft-deleted assistant must fail NotFound.
    let err = service
        .publish_version(
            &id,
            PublishAssistant {
                definition: AssistantDefinition {
                    kind: AssistantKind::Agent,
                    body: valid_agent_body(),
                },
                created_by: None,
                note: None,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, AssistantAdminError::NotFound { .. }));
}

/// Named exactly `assistant_version_immutable` for 27-VALIDATION.md: proves that
/// publishing a second version never rewrites the first -- `get_version(id, 1)` after a
/// `publish_version` call still returns the ORIGINAL definition, unchanged, and the port
/// itself has no method that could express an update (see
/// `crates/paladin-ports/src/input/assistant_admin_port.rs`'s own `grep -c 'async fn
/// update'` acceptance criterion, which this test complements behaviorally).
#[tokio::test]
async fn assistant_version_immutable() {
    let (service, _repository) = service();
    let id = AssistantId::new("svc-immutable").unwrap();
    let original_body = valid_agent_body();
    service
        .create(
            &id,
            PublishAssistant {
                definition: AssistantDefinition {
                    kind: AssistantKind::Agent,
                    body: original_body.clone(),
                },
                created_by: None,
                note: None,
            },
        )
        .await
        .unwrap();

    let mut second_body = valid_agent_body();
    second_body["system_prompt"] = serde_json::json!("A completely different prompt.");
    service
        .publish_version(
            &id,
            PublishAssistant {
                definition: AssistantDefinition {
                    kind: AssistantKind::Agent,
                    body: second_body,
                },
                created_by: None,
                note: None,
            },
        )
        .await
        .unwrap();

    let v1 = service.get_version(&id, 1).await.unwrap().unwrap();
    assert_eq!(
        v1.definition.body, original_body,
        "version 1 must be unchanged"
    );

    // version 0 and any version above latest (2) are "nothing here", never an error.
    assert!(service.get_version(&id, 0).await.unwrap().is_none());
    assert!(service.get_version(&id, 3).await.unwrap().is_none());
}

// --- Freeze-at-submit end-to-end (D-30) ----------------------------------

/// Named exactly `submit_without_version_freezes_latest` per the plan's own action text:
/// a run submitted with `version: None` against a stored assistant freezes the resolved
/// version at submit time; publishing a NEW version afterward must never retroactively
/// change what the already-persisted run reports.
#[tokio::test]
async fn submit_without_version_freezes_latest() {
    use crate::application::services::run::resolver::CodeWorkflowResolver;
    use crate::application::services::run::submission::RunSubmissionService;
    use paladin_core::platform::container::run::RunId;
    use paladin_ports::input::run_submission_port::{RunSubmissionPort, SubmitRun};
    use paladin_ports::output::run_repository_port::RunRepositoryPort;
    use paladin_storage::run::in_memory::InMemoryRunRepository;
    use paladin_storage::run_queue::in_memory::InMemoryRunQueue;

    let (service, assistant_repository) = service();
    let id = AssistantId::new("freeze-latest").unwrap();
    service
        .create(
            &id,
            PublishAssistant {
                definition: AssistantDefinition {
                    kind: AssistantKind::Workflow,
                    body: valid_workflow_body(),
                },
                created_by: None,
                note: None,
            },
        )
        .await
        .unwrap();

    let stored_repo_port: Arc<
        dyn paladin_ports::output::assistant_repository_port::AssistantRepositoryPort,
    > = Arc::clone(&assistant_repository) as _;
    let stored_resolver = Arc::new(StoredAssistantResolver::new(
        Arc::clone(&stored_repo_port),
        Arc::new(validator()),
    ));
    let code_resolver = Arc::new(CodeWorkflowResolver::new());
    let resolver: Arc<dyn crate::application::services::run::resolver::AssistantResolver> =
        Arc::new(ChainedResolver::new(stored_resolver, code_resolver));

    let run_repository: Arc<dyn RunRepositoryPort> =
        Arc::new(InMemoryRunRepository::new().with_assistants(Arc::clone(&assistant_repository)));
    let queue = Arc::new(InMemoryRunQueue::new());
    let submission = RunSubmissionService::new(Arc::clone(&run_repository), queue, resolver);

    let accepted = submission
        .submit(SubmitRun {
            assistant_id: "freeze-latest".to_string(),
            version: None,
            thread_id: None,
            input: serde_json::json!({}),
            webhook: None,
            requested_by: None,
        })
        .await
        .unwrap();
    let run_id: RunId = accepted.run_id;

    let run_after_submit = run_repository.get(&run_id).await.unwrap().unwrap();
    assert_eq!(run_after_submit.assistant.version, 1);

    // Publish v2 AFTER the run was submitted.
    service
        .publish_version(
            &id,
            PublishAssistant {
                definition: AssistantDefinition {
                    kind: AssistantKind::Workflow,
                    body: valid_workflow_body(),
                },
                created_by: None,
                note: None,
            },
        )
        .await
        .unwrap();

    let run_after_publish = run_repository.get(&run_id).await.unwrap().unwrap();
    assert_eq!(
        run_after_publish.assistant.version, 1,
        "the already-persisted run must still report v1, never the newly-published v2"
    );
}
