Phase 36 Plan 11 -- examples/README.md gallery-index completion evidence
==========================================================================

Task 1: EX-01 and EX-122 -- Getting Started currency + drifted PaladinResult snippet fields
---------------------------------------------------------------------------------------------

EX-01 (D-22) -- minimum Rust version:

  Checked against: Cargo.toml line 18, `rust-version = "1.88"`.

  Before: "- Rust 1.70 or later"                (examples/README.md, Getting Started block)
  After:  "- Rust 1.88 or later"

EX-01 (D-22) -- "Run with Specific Features" block feature-name currency check:

  Block (examples/README.md, Running Examples section) names two features:
    - `redis-queue`
    - `s3-storage`

  Checked against Cargo.toml [features] table:
    - `redis-queue = ["paladin-storage/redis-queue"]`  -- present, valid.
    - `s3-storage = ["paladin-storage/s3"]`             -- present, valid.

  Result: both feature names already exist in the root Cargo.toml. No correction needed.

EX-122 (D-21) -- drifted PaladinResult field names:

  Checked against: crates/paladin-core/src/platform/container/execution_result.rs
  lines 49-64 -- the real PaladinResult fields are `output` (String), `usage`
  (TokenUsage, with its own `.total_tokens`), and `execution_time_ms` (u64).
  examples/basic_paladin.rs itself already reads the correct field names
  (`result.output`, `result.usage.total_tokens`, `result.execution_time_ms`).

  Fix 1 -- Basic Paladin Examples snippet (line ~74):
    Before: `println!("{}", response.content);`
    After:  `println!("{}", response.output);`

  Fix 2 -- Logging and Observability advanced snippet (lines ~1350-1351):
    Before:
      tokens = response.token_usage.total_tokens,
      duration = ?response.execution_time,
    After:
      tokens = response.usage.total_tokens,
      duration = ?response.execution_time_ms,

  Fix 3 -- Building a Custom Example snippet (line ~1455, found while checking
  "the rest of the page" per the plan's own instruction):
    Before: `println!("{}", response.content);`
    After:  `println!("{}", response.output);`

  Verification:
    $ grep -cE 'response\.content|response\.token_usage|response\.execution_time([^_]|$)' examples/README.md
    0
    $ RUSTV=$(grep -m1 '^rust-version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/'); grep -q "Rust $RUSTV" examples/README.md && echo OK
    OK (RUSTV=1.88)

  Both pre-existing snippet blocks (Basic Paladin Examples, Logging and
  Observability) are still present -- corrected in place, not deleted.

Deviation (Rule 1, auto-fixed): pre-existing Demonstrates-count inconsistency
------------------------------------------------------------------------------

While preparing for Task 3's both-directions Demonstrates-line-count check, found
that `### [cli_configs/maneuver.yaml](cli_configs/maneuver.yaml)` (a non-.rs
Configuration Examples entry, pre-existing before this plan) carried a
`**Demonstrates:**` bold-line prefix that none of its five sibling
`cli_configs/*.yaml` sections use -- they all use a plain prose sentence instead.
This one stray Demonstrates line made the file's total Demonstrates-line count
(39) exceed the .rs on-disk file count (38 pre-plan) by one BEFORE this plan even
started, which would have made Task 3's own count-equality invariant
unsatisfiable after adding exactly one Demonstrates line per newly-documented
.rs file. Fixed by removing the stray `**Demonstrates:**` prefix and folding its
text into the section's plain descriptive sentence, matching every sibling yaml
section's format -- no content removed, no snippet block touched, still inside
examples/README.md.

  Before:
    ### [cli_configs/maneuver.yaml](cli_configs/maneuver.yaml) 🆕
    **Demonstrates:** Complete Maneuver YAML configuration

    Shows full configuration template for Maneuver Battalion with all options.

  After:
    ### [cli_configs/maneuver.yaml](cli_configs/maneuver.yaml) 🆕

    Complete Maneuver YAML configuration template for Maneuver Battalion with all options.


Task 2: EX-121 -- sections for the eleven previously unlisted programs
-------------------------------------------------------------------------

Eleven on-disk programs with zero README mentions before this plan:
commander_council.rs, commander_grove.rs, conclave_expert_panel.rs,
council_discussion.rs, document_processing.rs, grove_routing.rs,
http_service_host.rs, paladin_with_rag.rs, vision_analysis.rs,
vision_battalion.rs, war_engine_memory_baseline.rs.

New ## sections added (house shape: `### [name.rs](name.rs)`, **Demonstrates:**
line sourced from each program's own header comment, one sentence, a fenced
run command, **Key concepts:**, no snippet block):

  - ## Vision                                        -- vision_analysis.rs, vision_battalion.rs
  - ## Document Processing                            -- document_processing.rs
  - ## HTTP Service Host                              -- http_service_host.rs
  - ## RAG & Retrieval                                -- paladin_with_rag.rs (sanctum_rag_retrieval.rs joins in Task 3)
  - ## Commander Strategies (Council / Grove / Conclave) -- commander_council.rs, commander_grove.rs,
                                                          conclave_expert_panel.rs, council_discussion.rs,
                                                          grove_routing.rs
  - war_engine_memory_baseline.rs -- joined the EXISTING "## Performance Benchmarking
    Examples" heading (not a new heading), immediately after muster_baseline.rs's section.

Matching Table-of-Contents bullets added, in document order, between the
"State Management Examples" and "Performance Benchmarking Examples" bullets
(for the four Task-2 headings placed before Performance Benchmarking) and the
"Performance Benchmarking Examples" and "Configuration Examples" bullets (for
the Task-3 headings placed after it).

Feature-gated run commands checked against Cargo.toml [[example]] declarations:
  - vision_analysis:     required-features = ["vision", "llm-openai"]  -> `--features "vision,llm-openai"` (matches)
  - vision_battalion:    required-features = ["vision", "llm-openai"]  -> `--features "vision,llm-openai"` (matches)
  - document_processing: required-features = ["content-processing"]   -> `--features content-processing` (matches)
  - http_service_host:   required-features = ["web-server"]           -> `--features web-server` (matches)

Verification (both-directions cross-check after Task 2, before Task 3's remaining
thirteen programs were documented):
  $ grep -oE '^### \[[a-zA-Z0-9_]+\.rs\]' examples/README.md | sed 's/^### \[//; s/\]$//' | sort -u > /tmp/36-11-listed.txt
  $ printf '%s\n' commander_council.rs commander_grove.rs conclave_expert_panel.rs \
      council_discussion.rs document_processing.rs grove_routing.rs http_service_host.rs \
      paladin_with_rag.rs vision_analysis.rs vision_battalion.rs war_engine_memory_baseline.rs \
      | sort -u > /tmp/36-11-eleven.txt
  $ comm -23 /tmp/36-11-eleven.txt /tmp/36-11-listed.txt
  (empty -- all eleven now listed)

  $ grep -c 'war_engine_memory_baseline' examples/README.md
  2   (section header + run-command mention -- sits under Performance Benchmarking Examples)

  $ git status --porcelain
   M examples/README.md
  (no file outside examples/README.md and this evidence file modified)


Task 3: sections for the fourteen new programs, both-directions cross-check, single commit
-----------------------------------------------------------------------------------------------

token_economy_commissary.rs already had its section from plan 36-01 (Token Economy
Examples). The remaining thirteen new programs each got a new ## heading and
`### [name.rs](name.rs)` section, Demonstrates line wording matching each
program's own D-24 closure-table capability naming from its originating plan's
SUMMARY (36-06 through 36-10):

  - ## WarEngine Configuration & Checkpoints  -- war_engine_configuration.rs        (EX-62..66, EX-80)
  - ## Control Flow & Dynamic Routing         -- control_flow_dynamic_routing.rs    (EX-67..70)
  - ## Human-in-the-Loop                      -- human_in_the_loop_gate.rs          (EX-71..73)
  - ## Graceful Shutdown                      -- graceful_shutdown.rs               (EX-74..76)
  - ## Agent Runtime & Middleware             -- agent_runtime_middleware.rs        (EX-83..87, EX-89)
  - ## Structured Output                      -- structured_output_schema.rs        (EX-88, EX-90)
  - ## RAG & Retrieval (joined)               -- sanctum_rag_retrieval.rs           (EX-116..120)
  - ## Platform API                           -- platform_api_client.rs             (EX-77..79, EX-91..95, EX-98, EX-99, EX-104, EX-110)
                                                  webhook_receiver.rs                (EX-96, EX-97)
  - ## Node-Result Cache                      -- node_result_cache.rs               (EX-81, EX-82)
  - ## Observability & Tracing                -- observability_tracing.rs           (EX-100..102, EX-108)
                                                  observability_otel_export.rs       (EX-103)
  - ## Evaluation                             -- eval_scenarios_demo.rs             (EX-105..107)

Gated run commands checked against Cargo.toml [[example]] declarations:
  - platform_api_client:       required-features = ["web-server", "dev-ui"] -> `--features "web-server,dev-ui"` (matches)
  - webhook_receiver:          required-features = ["web-server"]           -> `--features "web-server"` (matches)
  - node_result_cache:         required-features = ["redis-cache"]          -> `--features "redis-cache"` (matches; cargo build, not run -- D-16)
  - observability_otel_export: required-features = ["otel"]                 -> `--features "otel"` (matches; cargo build, not run -- D-16)

External-service prerequisites stated in the section text:
  - node_result_cache.rs: "Needs a running Redis server (`make services-up`); build-only in CI"
  - observability_otel_export.rs: "Needs a reachable OTLP/HTTP collector ...; build-only in CI"

No new section (Task 2 or Task 3) carries a fenced ```rust``` snippet block --
confirmed:
  $ sed -n '1270,1787p' examples/README.md | grep -n '```rust'
  (no output)

Both-directions cross-check (final, after all 24 sections added):
  $ ls examples/*.rs | xargs -n1 basename | sort -u > /tmp/36-11-ondisk.txt
  $ wc -l /tmp/36-11-ondisk.txt
  62 /tmp/36-11-ondisk.txt

  $ grep -oE '^### \[[a-zA-Z0-9_]+\.rs\]' examples/README.md | sed 's/^### \[//; s/\]$//' | sort -u > /tmp/36-11-listed.txt
  $ wc -l /tmp/36-11-listed.txt
  62 /tmp/36-11-listed.txt

  $ comm -23 /tmp/36-11-ondisk.txt /tmp/36-11-listed.txt   (on disk, not listed)
  (empty)

  $ comm -13 /tmp/36-11-ondisk.txt /tmp/36-11-listed.txt   (listed, not on disk)
  (empty)

  Both differences are empty -- the gallery index is complete in both directions.

Demonstrates-line-count check:
  $ grep -c '^\*\*Demonstrates:\*\*' examples/README.md
  62
  $ wc -l < /tmp/36-11-ondisk.txt
  62
  (equal)

Full on-disk list (62 files) -- identical to the listed-section list below,
byte for byte:
agent_handoffs.rs
agent_runtime_middleware.rs
arsenal_stdio_tools.rs
arsenal_streamable_http_tools.rs
autonomous_full_config.rs
autonomous_planning.rs
autonomous_prompt_generation.rs
basic_paladin.rs
battalion_checkpoint_recovery.rs
campaign_workflow.rs
chain_of_command_delegation.rs
citadel_autosave.rs
citadel_restore.rs
commander_auto.rs
commander_basic.rs
commander_council.rs
commander_full_config.rs
commander_grove.rs
commander_with_metadata_export.rs
conclave_expert_panel.rs
control_flow_dynamic_routing.rs
council_discussion.rs
document_processing.rs
dynamic_temperature.rs
eval_scenarios_demo.rs
formation_sequential.rs
garrison_in_memory.rs
garrison_persistent.rs
garrison_semantic_search.rs
graceful_shutdown.rs
grove_routing.rs
herald_custom_formatter.rs
herald_json_output.rs
herald_markdown_output.rs
herald_streaming.rs
http_service_host.rs
human_in_the_loop_gate.rs
llm_provider_selection.rs
maneuver_basic.rs
maneuver_dynamic_flow.rs
maneuver_nested_flow.rs
muster_baseline.rs
node_result_cache.rs
observability_otel_export.rs
observability_tracing.rs
paladin_with_config.rs
paladin_with_rag.rs
paladin_with_sanctum.rs
phalanx_parallel.rs
platform_api_client.rs
sanctum_adapter_migration.rs
sanctum_basic_inmemory.rs
sanctum_configuration.rs
sanctum_qdrant_production.rs
sanctum_rag_retrieval.rs
structured_output_schema.rs
token_economy_commissary.rs
vision_analysis.rs
vision_battalion.rs
war_engine_configuration.rs
war_engine_memory_baseline.rs
webhook_receiver.rs

Full listed-section list (62 headers): identical set, confirmed via
`diff /tmp/36-11-ondisk.txt /tmp/36-11-listed.txt` producing no output.

git diff scope check:
  $ git status --porcelain
   M examples/README.md
  (plus this evidence file, untracked before this plan's commit; no other file
  touched by this plan)

Closing commit:
  docs(36): complete the examples gallery index (EX-01, EX-121 and EX-122 and the new program sections)
