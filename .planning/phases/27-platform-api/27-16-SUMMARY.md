---
phase: 27-platform-api
plan: 16
subsystem: docs
tags: [mdbook, kubernetes, platform-api, docs-only, sse, webhooks, schedules, ssrf]

requires:
  - phase: 27-platform-api (plan 05)
    provides: "WarGraphDoc — the Workflow Assistant Document Format page this page cross-links"
  - phase: 27-platform-api (plan 13)
    provides: "SSRF guard, HMAC signature scheme, retry/backoff schedule, webhook payload shape documented here"
  - phase: 27-platform-api (plan 14)
    provides: "ScheduleAdminPort HTTP surface, cron/timezone/thread_strategy/on_missed semantics documented here"
provides:
  - "docs/src/api-reference/platform-api.md — the Platform API user/operator reference: run status machine, submit/cancel/stream, threads, assistants, schedules, webhooks, pagination, auth/scopes, configuration"
  - "k8s/server/worker-deployment.yaml — the worker-replica Deployment example PRD 06 §6 owes"
  - "queue-worker.md's 'Run server: producer API + worker replicas' section and kubernetes.md's pointer to the same manifest"
  - "parley-and-chronicle.md's resume response updated with run_id (D-21) and a cross-link to the new page"
affects: []

tech-stack:
  added: []
  patterns:
    - "Documentation-as-deliverable: X-08 and D-26/D-42 are satisfied by NAMING limitations plainly (degraded SSE ordering has no cross-path guarantee and may coalesce supersteps; DNS-rebinding resolve-then-connect pinning is not implemented) rather than by implying coverage the code does not have"

key-files:
  created:
    - docs/src/api-reference/platform-api.md
    - k8s/server/worker-deployment.yaml
  modified:
    - docs/src/SUMMARY.md
    - docs/src/deployment-topologies/queue-worker.md
    - docs/src/deployment/kubernetes.md
    - docs/src/user-guides/parley-and-chronicle.md
    - k8s/README.md

key-decisions:
  - "Documented the run/thread routes 27-15 adds concurrently (list/cancel/webhook-deliveries on runs; list/get/fork/delete on threads) directly from 27-15-PLAN.md and PRD 06 §2.1, since this plan runs in a parallel worktree whose base predates 27-15's merge — the openapi.json cross-check the plan's own acceptance criteria describe can only run correctly on the post-merge tree; this worktree's own verification instead used the plan's literal <verify> block (SUMMARY link presence, rebinding/coalesce wording, X-Paladin-Signature, the stream route string) plus a full route-string grep against the page text."
  - "k8s/server/worker-deployment.yaml references a Secret named `paladin-secrets` (run-db-url/redis-url keys) rather than extending the sibling k8s/server/secret.yaml.example's `paladin-server-secrets` — this plan's own action text names `paladin-secrets` literally, and the illustrative root-level k8s/deployment.yaml + k8s/secret.yaml.example already establish that name as the repo's generic pattern for a Postgres/Redis-backed deployment (as opposed to k8s/server/secret.yaml.example's provider-key-only paladin-server-secrets, which the worker Deployment also references for OPENAI_API_KEY). k8s/README.md documents both."
  - "Cross-repo links from docs/src pages to files outside docs/ (k8s/server/worker-deployment.yaml, k8s/README.md) use full GitHub blob/tree URLs, following the existing house pattern in docs/src/deployment-topologies/http-service-host.md:130 (`https://github.com/DF3NDR/paladin-dev-env/tree/main/k8s/server`) — book.toml's `follow-web-links = false` means mdbook-linkcheck does not verify these targets, exactly like the existing precedent; a first attempt using a relative filesystem path (`../../../k8s/README.md`) failed mdbook's linkcheck because that resolver only walks paths inside docs/src."

requirements-completed: [PLAT-05, PLAT-06]

coverage:
  - id: D1
    description: "docs/src/api-reference/platform-api.md documents the run status machine (mermaid, exact D-02 edges), every endpoint with its auth tier and pagination, the seven SSE wire events and the degraded mode's stated limitation (no ordering guarantee, may coalesce supersteps), assistants (append-only, no PUT, synthetic code entries), schedules (5/6-field cron, UTC/IANA, strategies, on_missed, skipped_ticks), and webhooks (payload, signature verification recipe, retry table, SSRF rejection list, allow_private, DNS-rebinding limitation)"
    requirement: "PLAT-06"
    verification:
      - kind: other
        ref: "grep -q 'platform-api.md' docs/src/SUMMARY.md && grep -qi 'rebinding' docs/src/api-reference/platform-api.md && grep -q 'X-Paladin-Signature' docs/src/api-reference/platform-api.md && grep -q '/v1/runs/{run_id}/stream' docs/src/api-reference/platform-api.md (the plan's own <verify> block, all four true)"
        status: pass
      - kind: other
        ref: "route-string grep for /v1/runs, /v1/runs/{run_id}/stream, /v1/runs/{run_id}/cancel, /v1/runs/{run_id}/webhook-deliveries, /v1/threads/{thread_id}/fork, /v1/assistants, /v1/schedules — all present"
        status: pass
      - kind: other
        ref: "grep -c 'APP_RUN_STORE_BACKEND\\|APP_WEBHOOKS_ALLOW_PRIVATE\\|APP_SCHEDULES_ENABLED' docs/src/api-reference/platform-api.md == 3"
        status: pass
    human_judgment: false
  - id: D2
    description: "The queue/worker topology page and the Kubernetes docs carry a worker-replica example manifest reading the APP_RUN_* env vars, and neither implies the in-process auth token store is multi-replica safe"
    requirement: "PLAT-06"
    verification:
      - kind: other
        ref: "python3 -c \"import yaml; d=list(yaml.safe_load_all(open('k8s/server/worker-deployment.yaml'))); assert all(x['kind']=='Deployment' for x in d if x)\" — pass"
        status: pass
      - kind: other
        ref: "grep -q 'worker-deployment' docs/src/deployment-topologies/queue-worker.md && grep -ci 'single-replica\\|ADR-0041' docs/src/deployment-topologies/queue-worker.md >= 1 && grep -q 'worker-deployment' docs/src/deployment/kubernetes.md && grep -q 'worker-deployment' k8s/README.md — all pass"
        status: pass
    human_judgment: false
  - id: D3
    description: "mdbook build succeeds under warning-policy = \"error\" — every link on the new/edited pages resolves"
    requirement: "PLAT-06"
    verification:
      - kind: other
        ref: "cd docs && mdbook build (after `mdbook-mermaid install .` regenerated the gitignored local mermaid assets, per 27-05's own documented precedent) — exit 0, 'No broken links found', re-run after each of the four edited/created pages"
        status: pass
    human_judgment: false

duration: ~1h10min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 16: Platform API Documentation and Worker-Replica K8s Example Summary

**The mdBook page PRD 06 and X-08 owe (`docs/src/api-reference/platform-api.md`) plus the
worker-replica Kubernetes manifest PRD 06 §6 owes (`k8s/server/worker-deployment.yaml`), with
every named limitation (SSE degraded-mode ordering, webhook DNS-rebinding, the in-process
auth-store's single-replica scope under a worker/API split) stated plainly rather than implied
away.**

## Performance

- **Duration:** ~1h10min
- **Started:** 2026-09-08 (worktree base `9f288751f4`)
- **Completed:** 2026-09-08
- **Tasks:** 2 (both `type="auto"`)
- **Files modified:** 7 (2 created, 5 modified)

## Accomplishments

- `docs/src/api-reference/platform-api.md` documents the whole Platform API end-to-end: the
  status machine as a mermaid diagram with the exact D-02 edges; `POST /runs` (202 semantics,
  `409 thread_busy` including `AwaitingInput` counting as busy per D-18, with the resume remedy
  named in the error body); cancel semantics (`Halted` Waypoint vs. `Cancelled` run — the two
  vocabularies stated as deliberately different); the seven frozen SSE wire events with their
  payload shapes, `mode`/`dropped`, the 15s heartbeat, and the degraded-mode paragraph naming its
  no-ordering-guarantee and superstep-coalescing limitation verbatim; threads (list/get/history/
  state/resume/fork/delete); assistants (envelope shape, the two kinds, publish-time validation's
  machine-readable `details` violation format, immutability enforced by "no `update` method and no
  `PUT` route exists", `latest` frozen at submit inside one transaction, synthetic
  `source: "code"` entries gated by `assistants.expose_code_registry`, and the first-page-only
  synthetic-merge limitation named plainly); schedules (5/6-field cron, UTC/IANA timezone,
  `thread_strategy`, `on_missed`, `skipped_ticks`, the `claim_tick` restart/replica-safety
  mechanism); webhooks (payload JSON example, the HMAC-SHA256 verification recipe over raw bytes
  in pseudo-code, the 1/2/4/8/16s retry table capped at 60s, the SSRF rejection list including the
  cloud metadata address always-rejected rule, `webhooks.allow_private`, no-redirects, and the
  DNS-rebinding limitation stated as documented-not-implemented); pagination (`limit` 1..=100
  default 20, opaque cursor, the cursor-walk-is-not-a-snapshot caveat); the two-tier
  invocation-vs-registry authorization model; every new config struct's env vars and defaults,
  naming `assistants.expose_code_registry`'s on-by-default posture as the one deliberate exception
  to X-09; and a deployment pointer to the queue/worker topology page. Linked from
  `docs/src/SUMMARY.md` under "API Reference".
- `k8s/server/worker-deployment.yaml`: a `paladin-worker` Deployment (`replicas: 2`, same image as
  `k8s/server/deployment.yaml`) with `APP_RUN_STORE_BACKEND=postgres`,
  `APP_RUN_STORE_URL_ENV=PALADIN_RUN_DB_URL`, `APP_RUN_QUEUE_BACKEND=redis`,
  `APP_RUN_QUEUE_URL_ENV=PALADIN_REDIS_URL`, `APP_RUN_WORKER_CONCURRENCY=4`,
  `APP_RUN_WORKER_LEASE_SECONDS=60`, and `APP_SCHEDULES_ENABLED=true` set on this one Deployment
  only (a comment explains `claim_tick`'s conditional-update makes every one of its replicas
  racing the same tick safe by construction — exactly one wins). `terminationGracePeriodSeconds:
  60` (2x the default `APP_ENGINE_SHUTDOWN_GRACE_SECS=30`, mirroring `deployment.yaml`'s own rule).
  Secrets referenced from a `paladin-secrets` Secret (`run-db-url`, `redis-url` keys) plus the
  existing `paladin-server-secrets` for the provider key. No literal secret values anywhere in the
  file (`grep -cE 'sk-|password:'` is `0`).
- `docs/src/deployment-topologies/queue-worker.md` gains a "Run server: producer API + worker
  replicas" section: the API replicas and worker replicas share the same run store and queue;
  cancellation stays cross-instance regardless of which instance is actually running a thread; and
  scaling worker replicas does **not** change ADR-0041's scope — that limitation is a property of
  how many API replicas serve `bearer_token`-authenticated routes, since worker pods never serve
  `/v1` routes at all.
- `docs/src/deployment/kubernetes.md`'s Deployment section gains a matching "Worker replicas
  (Platform API, v0.10)" pointer to the same manifest and the same ADR-0041 restatement.
- `docs/src/user-guides/parley-and-chronicle.md`'s resume-response documentation gains `run_id`
  (D-21) on the `202` example and prose distinguishing the durable-enqueue path (a run row exists)
  from the pre-run-server in-process-spawn fallback (`run_id: null`), plus a cross-link to the new
  Platform API page.
- `k8s/README.md` documents the `paladin-secrets` Secret's required keys and restates that the
  worker/API split does not change the existing auth-store scaling guidance.

## Task Commits

Each task was committed atomically:

1. **Task 1: `docs/src/api-reference/platform-api.md` and the SUMMARY entry** - `a73a7462` (docs)
2. **Task 2: Worker-replica example (k8s) and topology/Kubernetes/parley page updates** -
   `424ec5b0` (docs)

**Plan metadata:** this file's own commit (docs: complete plan) — committed alongside this
SUMMARY per worktree execution mode.

## Files Created/Modified

- `docs/src/api-reference/platform-api.md` — the Platform API reference page (new).
- `docs/src/SUMMARY.md` — links the new page under "API Reference".
- `k8s/server/worker-deployment.yaml` — the worker-replica Deployment example (new).
- `k8s/README.md` — documents the `paladin-secrets` Secret and the worker/API auth-scope note.
- `docs/src/deployment-topologies/queue-worker.md` — "Run server: producer API + worker replicas"
  section.
- `docs/src/deployment/kubernetes.md` — "Worker replicas (Platform API, v0.10)" pointer in the
  Deployment section.
- `docs/src/user-guides/parley-and-chronicle.md` — resume response gains `run_id`; cross-link to
  the new page.

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **Documented 27-15's concurrently-landing routes from the plan and PRD, not from this
   worktree's own `openapi.json`.** This plan runs in a parallel worktree whose base predates
   27-15's merge, so `crates/paladin-web/openapi.json` here does not yet carry `GET /runs`,
   `POST /runs/{id}/cancel`, `GET /runs/{id}/webhook-deliveries`, `GET /threads`,
   `GET /threads/{id}`, `POST /threads/{id}/fork` or `DELETE /threads/{id}`. Per the orchestrator's
   explicit instruction, these are documented from `27-15-PLAN.md`'s own `<behavior>` text and PRD
   06 §2.1 as real, shipping surface — not marked as unimplemented or hedged as "coming soon." The
   plan's own full acceptance criteria include a Python cross-check against `openapi.json` for
   every `/v1/runs*`/`/v1/threads*`/`/v1/assistants*`/`/v1/schedules*` path; that specific check
   can only run correctly once this wave's worktrees merge (both 27-15's routes and this page need
   to exist in the same tree). This worktree instead ran the plan's literal `<verify>` block (four
   greps, all pass) plus its own route-string presence check against the seven named routes (all
   present in the page text) as the executable proof available inside this isolated worktree.
2. **`paladin-secrets` (not `paladin-server-secrets`) is the Secret name the worker manifest
   references for DB/queue credentials.** The plan's own action text names `paladin-secrets`
   literally, and it is already the repo's established name for a Postgres/Redis-backed
   deployment's credentials (the illustrative root-level `k8s/deployment.yaml` +
   `k8s/secret.yaml.example`) — as opposed to `k8s/server/secret.yaml.example`'s
   `paladin-server-secrets`, which is provider-key-only and which the worker Deployment ALSO
   references (for `OPENAI_API_KEY`) rather than duplicating those keys under a new name.
   `k8s/README.md` documents both Secrets' required keys.
3. **Cross-repo doc links use full GitHub URLs, not relative filesystem paths.** A first attempt
   at linking `queue-worker.md` to `k8s/README.md` via a relative path (`../../../k8s/README.md`)
   failed `mdbook build`'s linkcheck — that resolver only walks paths inside `docs/src`. Switched
   to the exact pattern `docs/src/deployment-topologies/http-service-host.md:130` already
   established (`https://github.com/DF3NDR/paladin-dev-env/tree/main/k8s/server`), which
   `book.toml`'s `follow-web-links = false` setting means linkcheck does not attempt to verify —
   consistent with the existing precedent rather than a new convention.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Local `mdbook build` failed on missing gitignored mermaid assets**
- **Found during:** Task 1's first `cd docs && mdbook build` verification run
- **Issue:** `mermaid.min.js`/`mermaid-init.js` are gitignored, generated-at-build-time assets
  (per `.gitignore`'s own comment) that this fresh worktree checkout did not yet have on disk —
  `mdbook build` failed with "Unable to copy ... mermaid.min.js ... No such file or directory".
  This is the exact same environment gap 27-05's own SUMMARY documented and resolved the same way.
- **Fix:** Ran `mdbook-mermaid install .` once (from `docs/`) to regenerate the local assets; no
  files from that command were staged or committed (confirmed via `git status --short` showing
  only this plan's own edited/created files after the commit).
- **Files modified:** none (build-environment fix only, no tracked file changed).
- **Verification:** `mdbook build` exits `0` with "No broken links found" after the install step,
  re-run after each subsequent page edit in Task 2.
- **Committed in:** N/A (no tracked-file change; the fix is a local, gitignored artifact).

**2. [Rule 1 - Bug] A relative-path cross-repo doc link failed linkcheck**
- **Found during:** Task 2, first `mdbook build` after adding the `queue-worker.md` → `k8s/README.md`
  link
- **Issue:** `[k8s/README.md](../../../k8s/README.md#worker-replicas-platform-api-v010)` resolved
  correctly on the filesystem but `mdbook-linkcheck`'s local-file resolver only walks paths inside
  `docs/src`, reporting it as broken (`1 broken links found`, exit `101`).
- **Fix:** Replaced with a full GitHub `tree`/`blob` URL, matching
  `http-service-host.md`'s own existing precedent for the identical problem (linking to a file
  outside `docs/src`), which `book.toml`'s `follow-web-links = false` setting exempts from
  verification.
- **Files modified:** `docs/src/deployment-topologies/queue-worker.md`
- **Verification:** `mdbook build` exits `0`, "No broken links found."
- **Committed in:** `424ec5b0` (Task 2 commit — caught before commit, no separate fix commit
  needed)

---

**Total deviations:** 2 auto-fixed (1 Rule 3 blocking build-environment fix with no tracked-file
change, 1 Rule 1 bug fix to a doc link). **Impact on plan:** Neither changed this plan's scope or
architecture; both were necessary to reach the plan's own stated `mdbook build` success criterion.

## Issues Encountered

None beyond the two auto-fixed deviations above.

## Known Stubs

None introduced by this plan. This plan is documentation-only; it names two PRE-EXISTING
limitations from earlier plans in this phase (the SSE degraded-mode ordering/coalescing behavior,
27-10; the webhook DNS-rebinding gap, 27-13) as required by their own `<must_haves>` — these are
not new stubs, they are accurate descriptions of already-landed, already-documented-in-code
behavior, cross-referenced here for the reader-facing page.

## User Setup Required

None — no external service configuration required. `mdbook build` and the YAML/route-string
verification all run locally with no Docker dependency.

## Next Phase Readiness

- The Platform API page is ready to be the target of any future phase's route additions — its own
  Configuration table and route tables are structured so a new subsystem is an additional row/
  section, not a rewrite.
- `k8s/server/worker-deployment.yaml` is ready for `src/bin/paladin-server.rs`'s eventual
  `RunWorkerPool`/`ScheduleService` wiring (named as owed to a later plan — 27-04's, 27-11's and
  27-13's own SUMMARYs all point at "27-17" as that wiring plan) to make the manifest's env vars
  actually take effect at runtime; the manifest documents the intended shape against the config
  surface that already exists (`src/config/{run_store,run_queue,run_worker,schedules}.rs`),
  consistent with how `docs/src/api-reference/platform-api.md`'s own Configuration section
  describes those same structs.
- The full openapi.json-vs-page cross-check named in this plan's own acceptance criteria (every
  `/v1/runs*`/`/v1/threads*`/`/v1/assistants*`/`/v1/schedules*` path appearing verbatim in the
  page) should be re-run once this wave's worktrees (this plan + 27-15) merge, since 27-15's routes
  are documented here from its plan text rather than from a locally-regenerated `openapi.json`.
- No blockers. `mdbook build` passes clean with `warning-policy = "error"`; the k8s manifest
  parses as valid YAML with the expected `kind: Deployment`; no literal secret values appear
  anywhere in the new manifest.

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `docs/src/api-reference/platform-api.md`
- FOUND: `docs/src/SUMMARY.md`
- FOUND: `k8s/server/worker-deployment.yaml`
- FOUND: `k8s/README.md`
- FOUND: `docs/src/deployment-topologies/queue-worker.md`
- FOUND: `docs/src/deployment/kubernetes.md`
- FOUND: `docs/src/user-guides/parley-and-chronicle.md`

**Commits verified to exist (git log --oneline):**
- FOUND: `a73a7462` docs(27-16): add Platform API mdBook page and SUMMARY entry
- FOUND: `424ec5b0` docs(27-16): add worker-replica k8s manifest and topology/deployment docs

**Verification commands re-run and confirmed passing:**
- `grep -q 'platform-api.md' docs/src/SUMMARY.md` → match
- `grep -qi 'rebinding' docs/src/api-reference/platform-api.md` → match
- `grep -q 'X-Paladin-Signature' docs/src/api-reference/platform-api.md` → match
- `grep -q '/v1/runs/{run_id}/stream' docs/src/api-reference/platform-api.md` → match
- Route-string presence for `/v1/runs`, `/v1/runs/{run_id}/stream`, `/v1/runs/{run_id}/cancel`,
  `/v1/runs/{run_id}/webhook-deliveries`, `/v1/threads/{thread_id}/fork`, `/v1/assistants`,
  `/v1/schedules` → all present, no `MISSING` lines
- `grep -ciE 'rebinding' docs/src/api-reference/platform-api.md` → `1`
- `grep -ci 'coalesce' docs/src/api-reference/platform-api.md` → `1`
- `grep -c 'X-Paladin-Signature' docs/src/api-reference/platform-api.md` → `1`
- `grep -c 'allow_private' docs/src/api-reference/platform-api.md` → `3`
- `grep -c 'APP_RUN_STORE_BACKEND\|APP_WEBHOOKS_ALLOW_PRIVATE\|APP_SCHEDULES_ENABLED' docs/src/api-reference/platform-api.md` → `3`
- `python3 -c "import yaml; d=list(yaml.safe_load_all(open('k8s/server/worker-deployment.yaml'))); print(all(x['kind']=='Deployment' for x in d if x))"` → `True`
- `grep -c 'APP_RUN_WORKER_CONCURRENCY' k8s/server/worker-deployment.yaml` → `1`
- `grep -c 'terminationGracePeriodSeconds' k8s/server/worker-deployment.yaml` → `1`
- `grep -cE 'sk-|password:' k8s/server/worker-deployment.yaml` → `0`
- `grep -c 'worker-deployment' docs/src/deployment-topologies/queue-worker.md` → `1`
- `grep -c 'worker-deployment' docs/src/deployment/kubernetes.md` → `1`
- `grep -c 'worker-deployment' k8s/README.md` → `2`
- `grep -ci 'single-replica\|ADR-0041' docs/src/deployment-topologies/queue-worker.md` → `1`
- `grep -c 'run_id' docs/src/user-guides/parley-and-chronicle.md` → `5`
- `grep -c 'platform-api.md' docs/src/user-guides/parley-and-chronicle.md` → `1`
- `cd docs && mdbook build` → exit `0`, "No broken links found" (re-run after every page edit)
- `git status --short` → clean after each commit; only this plan's declared files touched

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*
