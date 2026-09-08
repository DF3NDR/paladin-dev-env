# WarGraphDoc — the Workflow Assistant Document Format

**Since:** v0.10.0 (Doc 06, plan 27-05)
**Crate:** `paladin-battalion` (`paladin_battalion::engine::graph_doc`)
**Schema version field:** `schema_version` (currently `"1"`)

`WarGraphDoc` is the JSON document an admin authors, `POST /assistants` (Phase 27's Platform
API) persists, and a Workflow assistant's stored version is. It is the serde/schemars mirror
of the executable `WarGraph` the engine actually runs: every document that compiles is
guaranteed to run, because compiling a document **is** validating it.

## Compile is validation

`WarGraphDoc::compile(&EngineRegistries) -> Result<WarGraph, CompileError>` is the only way a
document becomes an executable graph. It:

1. Resolves every named reference the document makes — a `custom` edge condition, a `custom`
   retry predicate, a `custom` error handler, a `registered` output schema — against the
   caller's `EngineRegistries`.
2. Builds the corresponding `WarGraph`, node-by-node, in document order.
3. Calls `WarGraph::validate` on the fully-built graph before returning it.

Nothing partially built is ever returned. Every failure — an unknown node, a duplicate node
id, an unregistered name, an unsupported node kind, or a structural validation failure — is a
typed `CompileError` variant naming the offending node, edge, or name. There is no silent
drop and no bare string error.

## The three node kinds (v0.10 boundary)

A document's `nodes[].kind` may be exactly one of:

| `kind` | Compiles to | Body field |
|---|---|---|
| `"paladin"` | `NodeSpec::Paladin` | `nodes[].paladin` |
| `"gate"` | `NodeSpec::Gate` | `nodes[].gate` |
| `"workflow"` | `NodeSpec::Battalion` (a nested `WarGraphDoc`, compiled recursively) | `nodes[].workflow` |

**`kind: "function"` — or any other string — is explicitly unsupported.** A document cannot
name arbitrary Rust behavior: `NodeSpec::Function` exists in the runtime graph type, but there
is no document field that resolves to it, and no name in `EngineRegistries` ever resolves to
one. This is a deliberate v0.10 limitation, not an oversight — it closes the elevation-of-
privilege path a document-authored "call this Rust function" field would otherwise open
(a document is admin-authored JSON over HTTP; a code-registered assistant is a deployment-time
decision, reviewed like any other code change). A document naming an unsupported kind still
**parses** — the wire format accepts any `kind` string — but fails `WarGraphDoc::compile` with
a typed `CompileError::UnsupportedNodeKind { kind }` naming the rejected string. If you need
custom Rust logic, register a code-defined assistant instead of trying to express it as a
document.

A `workflow` node's nested document recurses through the SAME `compile` machinery, bounded to
8 levels deep (`CompileError::NestingTooDeep`) so a pathological document cannot make
compilation (or, transitively, execution) unboundedly expensive.

## Registry-resolved names

Four vocabularies in a document are names, resolved against the process's `EngineRegistries`
at compile time — never silently accepted, never silently dropped:

| Document field | Resolves against | Unresolved error |
|---|---|---|
| `edges[].condition.custom.name` | `EngineRegistries.edge_evaluators` | `CompileError::UnregisteredEdgeEvaluator` |
| `aegis.retry.retry_on.custom.name` | `EngineRegistries.retry_predicates` | `CompileError::UnregisteredRetryPredicate` |
| `aegis.on_error.custom.name` | `EngineRegistries.error_handlers` | `CompileError::UnregisteredErrorHandler` |
| `paladin.output_schema.registered.name` | `EngineRegistries.output_schemas` | `CompileError::UnregisteredOutputSchema` |

An edge condition of `always`, `contains`, or `regex` needs no registry entry — only `custom`
does.

## `schema_version`

Every document carries a `schema_version` field, currently required to equal `"1"`
(`WARGRAPH_DOC_SCHEMA_VERSION`). A document persisted under a future schema version will bump
this constant alongside a reader shim; until then, any other value is a typed
`CompileError::UnknownSchemaVersion`.

## Example: an approval-gate document

The shape a Paladin-drafts / human-approves loop takes as a document (abbreviated; see
`crates/paladin-battalion/tests/fixtures/graph_docs/approval_gate.json` for the full,
executable fixture):

```json
{
  "schema_version": "1",
  "entry": ["writer"],
  "nodes": [
    {
      "id": "writer",
      "kind": "paladin",
      "paladin": {
        "name": "Writer",
        "model": "gpt-4",
        "system_prompt": "Draft a short reply about {topic}.",
        "input_template": "{topic}",
        "output_field": "draft"
      }
    },
    {
      "id": "review",
      "kind": "gate",
      "gate": {
        "parley": "approval",
        "prompt_template": "Approve this draft? {draft}",
        "on_expire": { "type": "fail_run" },
        "output_field": "approved"
      }
    }
  ],
  "edges": [
    { "from": "writer", "to": "review" },
    { "from": "review", "to": "writer", "condition": { "type": "contains", "value": "false" } },
    { "from": "review", "to": "review", "condition": { "type": "contains", "value": "true" } }
  ],
  "schema": {
    "fields": [
      { "name": "topic", "kind": "string", "reducer": "last_write", "default": "cats" },
      { "name": "draft", "kind": "string", "reducer": "last_write" },
      { "name": "approved", "kind": "boolean", "reducer": "last_write", "default": false }
    ]
  }
}
```

`limits` and `default_aegis` are both optional — an absent `limits` falls back to the engine's
own defaults (50 supersteps, 25 node visits, no run timeout, 100 Muster tasks), and an absent
`default_aegis` means no graph-wide fault-tolerance policy.

## The JSON Schema

`WarGraphDoc`'s JSON Schema is **derived**, not hand-written: `schemars::schema_for!(WarGraphDoc)`
generates it directly from the Rust type, so the schema can never drift from what `compile`
actually accepts. The generated schema is checked in as a golden file at
`docs/schemas/wargraph-doc.schema.json` (repo-root-relative; not linked from this page as a
clickable URL — it lives outside `docs/src`, and mdBook's linkcheck runs in strict
`warning-policy = "error"` mode, so a relative link that would resolve outside the book's own
root is intentionally avoided here in favor of the plain path above).

If you change `WarGraphDoc` or any of its sub-document types, regenerate the golden file:

```bash
UPDATE_WARGRAPH_SCHEMA=1 cargo test -p paladin-battalion --test graph_doc_round_trip \
  wargraph_doc_schema_matches_golden
```

Then commit the regenerated `docs/schemas/wargraph-doc.schema.json` alongside your code
change — `wargraph_doc_schema_matches_golden` fails the build otherwise, catching schema drift
at test time rather than at review time.

## Fingerprint stability

`WarGraph::fingerprint()` — the identity a stored Waypoint's `graph_fingerprint` is checked
against on `resume` (ENG-FR-14) — is proven stable not just within one process, but across a
**real OS process boundary**: the same document, compiled in two independently-spawned
processes, yields the identical fingerprint string. This matters because Rust's `HashMap` uses
a per-process random seed (`RandomState`) — a canonical encoding that accidentally leaked a
map's iteration order into the hashed bytes would produce a DIFFERENT fingerprint every time
the process restarted, silently breaking every `resume` call after a deploy. The proof itself
spawns a genuine second process (`std::env::current_exe()`) rather than simulating one
in-process, so this failure mode cannot hide behind a same-process round trip.
