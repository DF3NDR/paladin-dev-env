# Phase 54: First-Run Usability - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-31
**Phase:** 54-first-run-usability
**Mode:** `--auto` — every question resolved to the recommended option without prompting; single
pass (no re-read for gaps). No SPEC.md for this phase.
**Areas discussed:** Placeholder refusal and honest examples; Version-string gate and the currency
sweep; Changelog, ADR and count corrections; API surface snapshot and the node-cache guide

`[--auto] Selected all gray areas: Placeholder refusal and honest examples, Version-string gate and
the currency sweep, Changelog/ADR/count corrections, API surface snapshot and node-cache guide.`

---

## Placeholder refusal and honest examples (FIRST-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Exact `${NAME}` only | Mirror `paladin-llm`'s `placeholder_name` grammar | |
| Any value containing `${` | Fail closed on partial templates and lowercase names too | ✓ |

**Choice:** `[auto] Detection rule → "Any value containing '${'" (recommended)`
**Notes:** A generated secret never contains `${`; a bare `$VAR` is still accepted.

| Option | Description | Selected |
|--------|-------------|----------|
| Name the entry only | `http.auth.api_keys[<name>]`, never the value or the variable name | ✓ |
| Echo the variable name | Friendlier, but prints part of the key value | |

**Choice:** `[auto] Message → "Name the entry only" (recommended)` — house never-print rule.

| Option | Description | Selected |
|--------|-------------|----------|
| Commented entries showing `key: ""` | `api_keys: []`; an uncommented, unedited entry is refused as empty | ✓ |
| Uncommented `key: ""` entries | Refuses even a dev run with `enabled: false` until the entries are deleted | |
| Dummy literal (`change-me`) | A loadable placeholder; SC1 forbids it | |

**Choice:** `[auto] config.example.yml shape → "Commented entries showing key: \"\"" (recommended)`

| Option | Description | Selected |
|--------|-------------|----------|
| ConfigMap kept, keys commented | Honest, but the k8s first run always fails closed with no path to a working one | |
| initContainer `envsubst` | Expansion moved into the pod spec; adds an image and a script | |
| Server config delivered as a Secret | One file holding literal keys belongs in a Secret; the server reads exactly one file | ✓ |

**Choice:** `[auto] k8s delivery → "Config as a Secret" (recommended)`
**Notes:** Rejected: a `key_env` field (a new secret-handling path, out by the milestone decision).

---

## Version-string gate and the currency sweep (FIRST-02, FIRST-03, FIRST-04)

| Option | Description | Selected |
|--------|-------------|----------|
| Copyable pattern classes | Workspace-crate dependency pins, `Current version`, k8s labels/first-party image tags, `openapi.json` | ✓ |
| Every `X.Y.Z` string | 220 hits in 44 `docs/src` files, mostly history | |

**Choice:** `[auto] Gate scope → "Copyable pattern classes" (recommended)`

| Option | Description | Selected |
|--------|-------------|----------|
| Per-file exemption list with reasons | Migration guide, ADR-history page; tested both ways | ✓ |
| Inline exemption markers | Finer-grained, but scattered and easy to cargo-cult | |

**Choice:** `[auto] Exemptions → "Per-file list with reasons" (recommended)`

| Option | Description | Selected |
|--------|-------------|----------|
| Version tag in manifests; build target tags it | `docker-build-server` also tags `paladin-server:<version>` | ✓ |
| `:latest` in manifests | Not version-held; pull-policy pitfalls for local images | |

**Choice:** `[auto] Image tags → "Version tag, build target tags it" (recommended)`
**Notes:** Found by the scout: `make docker-build-server` tags only `:latest` while the manifests pull
`:0.10.0` / `:0.6.0`.

| Option | Description | Selected |
|--------|-------------|----------|
| Check-only gate | Phase 58's bump PR edits what the gate lists; release.yml's CI-conclusion clause blocks a red tag | ✓ |
| Add a `--write` rewrite mode now | A new capability beyond FIRST-03 | |
| Template labels (Kustomize/Helm) | C4's "consider"; more tooling than the gate needs | |

**Choice:** `[auto] Bump path → "Check-only; rewrite deferred" (recommended)`

---

## Changelog, ADR and count corrections (FIRST-05, FIRST-06, FIRST-07)

| Option | Description | Selected |
|--------|-------------|----------|
| Count-free wording | The scripts enumerate via `cargo metadata`; a number can only go stale | ✓ |
| Correct the number | Eleven/twelve today, stale again on the next crate | |

**Choice:** `[auto] Counts → "Count-free wording" (recommended)`

| Option | Description | Selected |
|--------|-------------|----------|
| Delete the dangling `[Unreleased]:` definition | No other 0.3-0.11 heading has a link definition | ✓ |
| Retarget to `DF3NDR/paladin-dev-env/compare/v0.11.0...HEAD` | Needs an edit every release | |

**Choice:** `[auto] Footer link → "Delete" (recommended)`

Carried without a choice: crate changelog correction under a new `[Unreleased]` anchor (published
`[0.11.0]` sections frozen); ADR-0056 amended in place with a dated note; `cargo-release 1.1.7` from
`47-CI-EVIDENCE.md` rows 13-14.

---

## API surface snapshot and the node-cache guide (FIRST-08, FIRST-09)

Carried without a choice (prove-then-lock and milestone decisions B10/B12):
- `make api-surface` on the pinned `nightly-2026-09-20` first; regenerate only on drift; never
  hand-edit; reconcile the `UnsupportedParameter` / `check_reasoning` wording if those names reach
  the facade only through glob re-exports.
- The node-cache guide is `docs/src/user-guides/fault-tolerance.md`'s cache section; it states that
  the server attaches no cache, documents `NODE_CACHE_KEY_VERSION = "k1"`, and documents the
  `key_prefix` rotation for a provider-level effort change.

---

## Claude's Discretion

- Exact error wording; the script's internals and exemption-list layout; the Secret's layout
  (same Secret or a second one, `items` or `subPath`, deleting `configmap.yaml` or leaving a
  pointer); the evidence file name; plan and commit grouping; whether WINDOWS row 62 is closed here
  or handed to Phase 57.

## Deferred Ideas

- A `--write` mode or a bump helper for the version gate.
- Refusing `${…}` in other credential fields that are not expanded (the treasurer webhook
  `secret`).
- `${VAR}` expansion or a `key_env` indirection for HTTP auth keys (out by milestone decision).
- A boot `warn!` for `node_cache.enabled: true` on `paladin-server` (PACE-07 horizon).
- Templated k8s labels and tags.
- The four keyword-only todo matches (score 0.6), not folded, on the Phase 53 precedent.
