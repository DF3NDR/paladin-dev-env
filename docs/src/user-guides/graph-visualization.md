# Graph Visualization

**Since:** v0.10.0 (Phase 28, PRD 07)

Two CLI commands and one admin-gated dev page turn a `WarGraphDoc`/`WarGraph`, an execution
history, or both, into a diagram: `paladin-cli graph export` renders a graph's static shape,
`paladin-cli run export` renders a thread's *execution overlay* on top of that shape, and the
`dev-ui` inspector page renders the same overlay in a browser.

## Exporting a graph's static shape

```bash
paladin-cli graph export --format mermaid path/to/graph.json
```

renders a `WarGraphDoc` (JSON or YAML) to Mermaid on stdout — pipe-friendly, no colour, byte-exact
to what `to_mermaid` produces. `--format dot` renders the same shape as a Graphviz `digraph`
instead. Either an explicit file or a stored assistant works:

```bash
paladin-cli graph export --format mermaid --assistant my-workflow@3
paladin-cli graph export --format dot my-graph.yaml --out diagram.dot
```

`--assistant <id>[@<version>]` resolves a `Workflow`-kind assistant's stored document through the
configured `RunStoreConfig` backend (SQLite locally, Postgres by URL) — the same store the server
resolves against, reached through ports only, never HTTP. `--out <path>` writes to a file with a
short confirmation instead of stdout.

## Exporting a run's execution overlay

```bash
paladin-cli run export --thread <thread-id>
```

renders the SAME graph shape *plus* an execution overlay: which nodes actually ran, how many
times, with what outcome, and which edges actually fired versus were merely evaluated. Two
optional flags narrow or redirect resolution:

```bash
paladin-cli run export --run <run-id>                      # derive thread + graph from a run row
paladin-cli run export --thread <thread-id> --waypoint <id> # cap history to one Waypoint
paladin-cli run export --thread <thread-id> --graph my-graph.json  # explicit graph document
```

The command prints two lines identifying the resolved overlay **source** and graph **resolution**
ahead of the diagram, so you always know which of the two mixed-fidelity paths below produced what
you're looking at.

### Overlay source: Waypoints vs. persisted trace

- **Waypoint history** (always available) — fired edges are *derived* from each superstep's
  `completed` → next superstep's `vanguard` transition. Evaluated-but-not-fired edges are never
  visible on this path (there is nothing in a Waypoint recording "this edge was checked and lost").
- **Persisted trace** (the upgrade, requires `trace.persist: true` — see the [observability
  page](../operations/observability.md)) — fired *and* evaluated-but-not-fired edges are read
  directly from `TraceEvent::EdgeEvaluated`, exact rather than derived.

`run export` prefers the trace source whenever non-empty rows exist for the thread, falling back
to Waypoint history otherwise.

### Graph resolution order

1. `--graph <file>`, if given.
2. The run's assistant version's stored `WarGraphDoc`, when the thread belongs to a known run.
3. **Observed-only** — no static graph resolves (no `--graph`, no run, or the assistant is
   `Agent`-kind). The diagram is built from only the nodes and edges actually seen, titled
   `(observed nodes only — no graph document available)`. This is expected and acceptable: the
   acceptance question ("which branch fired and why did node X run 3 times") is answered by the
   overlay itself, not by unexecuted nodes.

## The badge and outcome-colour legend

Every rendered node carries a guillemet-quoted kind badge (`«paladin»`, `«function»`, `«gate»`,
`«workflow»`, `«worker»`), frozen by the golden fixtures under
`crates/paladin-battalion/tests/golden/export/`. `Gate` nodes render as a diamond; `Workflow`
nodes render as a nested subgraph cluster; a worker-template node and a deferred (Muster
aggregator) node both render dashed.

On an execution overlay, a visited node is additionally coloured by its **last** visit's outcome:

| Outcome | Meaning |
|---|---|
| `success` | the attempt completed normally |
| `failed` | the attempt's last recorded outcome was a failure |
| `parleyed` | the node raised a Parley (a Gate suspension) |
| `skipped` | the attempt was skipped (e.g. by a shutdown deadline) |
| `cache_hit` | the outcome was served from the node cache, not executed |

A node visited more than once carries a `×N` badge; every visited node's label additionally
carries `<duration>ms · <tokens>tok` — a cache-hit visit's duration/token figures are `None` by
construction (never a stale or zero number) and render as a dash instead. A fired edge renders
bold; an evaluated-but-not-fired edge (trace source only) renders dotted.

## The `dev-ui` inspector page

`GET /v1/dev-ui/threads/{id}` renders the same `RunInspectorPort::inspect` view as
`run export`, as a static, admin-gated HTML page: the diagram, a per-node visit summary, a
fired/evaluated-edge list per superstep, and the full superstep table. It requires:

- The `dev-ui` feature (`crates/paladin-web`'s **first** `[features]` section, off by default,
  absent from `full`) compiled in.
- An authenticated request satisfying the SAME `require_auth` + `require_admin` middleware pair
  the rest of the crate's admin routes use — it exposes state field names and the run's own
  structure, which is operator information.

The page has no build pipeline: one static HTML template (`include_str!`-ed), the
`InspectorView` JSON embedded verbatim into a typed `<script type="application/json">` element
(with `</` and `<!--` escaped so the payload can never break out of its own tag), and an inline
vanilla-JS renderer. It makes exactly one external request beyond the page itself — importing the
Mermaid ESM module the diagram is rendered with.

### `mermaid_url` for air-gapped hosts

Mermaid is loaded from `web_server.dev_ui.mermaid_url`, which defaults to the jsDelivr
`mermaid@11` ESM bundle CDN URL. It is deliberately **not vendored** into the crate — a
multi-megabyte JS asset in a published crate's `include` list is a cost nobody asked for. An
air-gapped operator points this at a local mirror instead:

```yaml
web_server:
  dev_ui:
    mermaid_url: "https://internal-mirror.example.com/mermaid/11/mermaid.esm.min.mjs"
```

or via the environment:

```bash
export APP_WEB_SERVER_DEV_UI_MERMAID_URL="https://internal-mirror.example.com/mermaid/11/mermaid.esm.min.mjs"
```

If the configured URL cannot be loaded (a 5-second timeout, or a load failure), the diagram panel
falls back to showing the raw Mermaid source text — the other three panels (node visits, fired
edges, superstep table) render independently and are unaffected.
