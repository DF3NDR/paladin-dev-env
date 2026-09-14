# Upgrading

This page is for an operator or library consumer moving a deployment or dependency from
**v0.9.x to v0.10.0**. It orients you to what changed and points you at the authoritative
record; it does not duplicate that record's full detail.

The authoritative, exhaustive record of every behavioral change, Rust API change, schema
migration, configuration surface, HTTP surface change and the full upgrade checklist is the
root [`MIGRATION.md`](https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md) file.
Read this page first for orientation, then consult `MIGRATION.md` for the complete §9.1–§9.8
detail, including worked code examples for each behavioral change.

## Behavioral changes

Every v0.10.0 change an operator can observe **without** touching their own code, condensed
from `MIGRATION.md` §9.1 to one line each. See that section for the full "who is affected" /
"required user action" detail and worked examples.

| ID | Change | Required action |
|---|---|---|
| M-B-01 | `EdgeCondition::Custom(name)` no longer silently evaluates to `true` when no evaluator is registered (BUG-01 fix) — an unregistered custom edge now fails graph validation before any node executes. | Register an evaluator for each custom condition name, or replace the condition with `Contains`/`Regex`/`Always`. |
| M-B-02 | Graceful shutdown: on SIGTERM/SIGINT the process now waits up to `shutdown_grace` (default 30s) for in-flight engine runs to halt before exiting. | Set `terminationGracePeriodSeconds` to at least `60` (twice the default grace) in every Deployment manifest. |
| M-B-03 | No behavioral change to the default policy — `tool_error_mode` names the v0.9 behavior (`FeedToModel`) rather than introducing a new one, and the fed-back error text is now redacted-then-bounded before the model sees it. | None required to keep today's behavior. Set `tool_error_mode = FailRun` to opt into failing the run on a tool error instead. |
| M-B-04 | Any graph executed through the new `WarEngine` writes one `Waypoint` (a full `Battlefield` snapshot) after every superstep by default. Legacy `Formation`/`Phalanx`/`Campaign`/`Commander` execution paths are completely unaffected — they write no Waypoints. | Only applies if you adopt the new `WarEngine`/`WarGraph` APIs: choose a `WaypointPort` backend and review `WaypointDurability` and `WaypointRetentionConfig`. |

## Upgrade checklist

One ordered, copy-pasteable checklist for upgrading a v0.9.0 deployment to v0.10.0, mirrored
from `MIGRATION.md` §9.8.

1. **Back up state.** Snapshot every state directory and database this deployment uses: the
   waypoint store (`SqliteWaypointStore`/`PostgresWaypointStore`'s backing file or database),
   the run store (`RunStoreConfig`'s SQLite file or PostgreSQL database), the Garrison SQLite
   database if used, and any Citadel state files. There is no destructive migration to reverse
   a bad upgrade against; a restored backup plus the v0.9.0 binary is the rollback path.
2. **Apply migrations — by starting the new binary, not a separate command.** Every migration
   this program added runs automatically at adapter construction via `sqlx::migrate!`; there is
   no `sqlx migrate run` step to invoke by hand. Start the new `paladin-server` binary once
   against the restored backup and confirm it comes up cleanly. The same automatic-migration
   mechanism applies to the PostgreSQL-backed adapters — no separate manual migration step is
   needed there either.
3. **Update config — nothing is required.** Every new v0.10 config surface defaults to today's
   behavior, proven by the `v0_9_config_boot` integration test: a v0.9 configuration file boots
   this binary with every new subsystem inert. No `config.yml` edit is required to preserve v0.9
   behavior; add a section only when actually adopting a new capability.
4. **Raise `terminationGracePeriodSeconds`.** Set it to at least `60` in every Deployment
   manifest before rolling out this upgrade (M-B-02). The shipped manifests
   (`k8s/deployment.yaml`, `k8s/server/deployment.yaml`, `k8s/server/worker-deployment.yaml`)
   already carry `terminationGracePeriodSeconds: 60`; a forked or hand-written manifest needs
   the same change.
5. **Register a custom evaluator for every `EdgeCondition::Custom` name in use.** M-B-01's fix
   makes an unregistered custom edge condition a validation failure, not a silent always-true.
   Register one via `CampaignExecutionService::with_evaluator("name", Arc::new(evaluator))` on
   the legacy execution path, or `WarEngine::with_edge_evaluator("name", Arc::new(evaluator))`
   on the `WarEngine` path, before calling `execute`/`start`.
6. **Deploy.** Roll out the new binary and manifests with the grace period and evaluator
   registrations from steps 4-5 already in place.
7. **Verify.** Run `paladin-cli setup-check --verbose` for an environment/toolchain/provider/
   service connectivity check; if the deployment uses the Maneuver flow DSL, run `paladin-cli
   maneuver validate` against its flow configuration; and run `paladin-cli eval run <glob>`
   against a representative scenario glob for a behavioral post-deploy check. `GraphCommands`
   exposes exactly one subcommand, `export` (`paladin-cli graph export --format mermaid|dot`),
   for inspecting a graph's structure — there is no separate command for a runtime probe.

## Full migration record

For every behavioral change's worked examples, the complete Rust API change register, schema
migrations, configuration and environment variable reference, and the HTTP API compatibility
notes, see the root
[`MIGRATION.md`](https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md) file.
