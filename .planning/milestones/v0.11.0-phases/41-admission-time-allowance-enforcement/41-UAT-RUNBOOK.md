# Phase 41 operator UAT runbook: allowance admission and warn path

Companion to `41-UAT.md`. Covers UAT test 1 (the operator walkthrough) end to end on a laptop,
and says what to tick for tests 2, 3 and 4. Everything here was taken from the Phase 41 code and
docs on branch `claude/laughing-dirac-e0h2ax`; nothing is assumed.

Revised 2026-10-05 after the first real walkthrough (recorded in `41-UAT.md`). The first draft
could not boot the server: it lacked the waypoint store variables, the `api_key` field of the
`llm.ollama` block and the `herald:` section, and its `run_traces` check needed the trace wiring
fix in `build_run_api`. Those corrections are folded in below.

Time: about 45 minutes once the binaries are built. Scope: one developer machine, SQLite, no
Docker, no Kubernetes.

---

## 0. Decide first: live LLM tokens or not

The walkthrough has two halves, and only one of them ever calls an LLM.

| Half | What it proves | Needs an LLM call? |
|---|---|---|
| A. Refusal | an exhausted key gets `429 allowance_exhausted` and nothing is persisted | **No.** The refusal happens before anything runs. The ledger is seeded with one SQL row. |
| B. Warn path | at 80% one admitted run yields one notice row, one operator webhook, one trace event, one herald line, and a second admission yields nothing | **Yes, exactly one admitted run executes**, which makes real model calls. |

Spend reaches the ledger only through priced model calls: with `treasurer.pricing` empty no call
is priced and the balance never moves. So Half B has two viable shapes; pick one before starting.

### Option 1 (recommended): no live tokens, local Ollama

- Build the server with `--features web-server,llm-ollama` (Ollama is not in the default feature
  set) and run a small local model. The walkthrough was run with `qwen2.5:0.5b` (397 MB, fine on
  CPU); any model you have works if the three places that name it below agree.
- Point the walkthrough agent at provider `ollama`.
- Give that model a **pretend** price in `treasurer.pricing` (`prompt: "100.00"`,
  `completion: "100.00"`, dollars per million tokens). The price is an accounting figure the
  ledger multiplies by token counts; it costs you nothing but makes spend visible. Do not go
  higher: one "Say hello" run used about 175 tokens, so at `1000.00` it cost 0.17 to 0.18 USD and
  two runs on top of the 0.80 seed crossed the 1.00 ceiling (the second admission in §5.7 only
  just stayed under it). At `100.00` a run costs about 0.02 USD.
- Nothing leaves the machine. No provider account, quota or billing is involved.

### Option 2: live provider (OpenAI, Anthropic, DeepSeek, Gemini, xAI, ...)

Only choose this if you specifically want the walkthrough to exercise a real provider adapter.
Then confirm, before you start, on the provider's platform:

1. **The key is live and funded.** The account must have prepaid credit or an active billing method
   and must not be at a monthly spend cap. A `402`, `429 insufficient_quota` or an org-level block
   from the provider shows up in Paladin as a failed run, which never settles spend, so Half B
   silently produces no warning and the test reads as a false failure.
2. **The key may use the model you configure.** Some orgs gate models per project or tier.
3. **Rate limits leave headroom.** One run makes one to a handful of calls; the default agent
   `max_loops` keeps it small, but a key already near its RPM/TPM limit will fail the run.
4. **The price table matches the model id exactly.** `treasurer.pricing.<model>` is keyed by the
   model string the agent sends; a mismatch means the call is unpriced and the balance does not
   move. Copy the provider's published per-million-token figures.
5. **Budget the real cost.** With pretend-free pricing you cannot "spend 2.50 for real" cheaply;
   instead set the walkthrough ceiling small (see §4, Option 2 variant) so one or two short calls
   cross it. Expect well under $0.05 of provider spend for the whole walkthrough with a small
   prompt and a mid-tier model.
6. **Export the key in the shell that starts the server** (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`,
   etc.). Never write it into `config.yml`; the loader does not expand `${VAR}` placeholders, so a
   placeholder would be sent to the provider literally and rejected.

If a live run fails for any provider reason, Half A is unaffected; only Half B needs the rerun.

---

## 1. Prerequisites

- Rust toolchain (the repo pins the version via `rust-toolchain` or CI; `cargo 1.97` was used for
  this phase), `sqlite3` CLI, `curl`, `python3`, `jq`, `openssl`.
- A checkout of branch `claude/laughing-dirac-e0h2ax` that includes the trace wiring fix in
  `src/infrastructure/web/run_api_wiring.rs` (`build_run_api` calls `with_trace_config` and
  `with_run_trace_port`). Without it `trace.persist` is ignored and §5.5 finds no rows.
- Ports: 8080 for `paladin-server`, 9099 for the webhook receiver (any free port works).
- For Option 1: Ollama running at `http://localhost:11434` with a pulled model
  (`ollama serve` in its own terminal, then `ollama pull qwen2.5:0.5b`). The Ollama installer
  needs `zstd`; on Debian run `sudo apt-get update && sudo apt-get install -y zstd` first.

Build both binaries once (the server image's own build line is `--features web-server`; add the
Ollama adapter for Option 1):

```bash
cd <checkout>
cargo build --release --bin paladin-server --features web-server,llm-ollama   # Option 1
# cargo build --release --bin paladin-server --features web-server            # Option 2
cargo build --release --bin paladin-cli --features cli
```

`paladin-cli treasury spend` opens the same SQLite file the server uses, selected by the same
`APP_RUN_STORE_*` variables, so keep one shell environment for both.

---

## 2. Configuration

Create `uat41/config.yml` (start from `config.example.yml` and set these sections; leave the rest
at the example's values). Replace the example's `agents:` list entirely: every listed agent's
provider must be buildable at boot, and the example's agents name providers with no key here.

```yaml
server:
  host: "127.0.0.1"
  port: 8080

http:
  auth:
    enabled: true
    api_keys:
      - key: "uat41-ci-runner-secret"      # Phase 41 UAT only; rotate afterwards
        name: "ci-runner"
        role: "admin"                      # admins are bound like any other key (D-09)
        tenant: "acme"
      - key: "uat41-control-secret"
        name: "svc-control"
        role: "user"
        tenant: "acme"                     # same tenant, NO allowance entry: the control key
      - key: "uat41-warn-secret"       # Half B's key
        name: "warn-key"
        role: "user"
        tenant: "acme"

llm:
  default_provider: "ollama"               # Option 2: "openai" (or your provider)
  ollama:
    api_key: ""                            # required field; the config does not load without it
    base_url: "http://localhost:11434/v1"
    default_model: "qwen2.5:0.5b"
    timeout_seconds: 60

agents:
  - id: "walkthrough"
    provider: "ollama"                     # Option 2: your provider
    model: "qwen2.5:0.5b"                  # Option 2: a real model id, e.g. "gpt-4o-mini"
    system_prompt: "Answer in one short sentence."
    allowed_roles: ["admin", "user"]

treasurer:
  currency: "USD"
  pricing:
    "qwen2.5:0.5b":                        # must equal the agent's model string exactly
      prompt: "100.00"                     # pretend price, USD per 1M tokens (Option 1)
      completion: "100.00"
  allowance:
    warn_at: 80
    webhook:
      url: "http://127.0.0.1:9099/hook"    # loopback: needs webhooks.allow_private below
    api_keys:
      ci-runner:
        period: "1h"
        amount: "2.50"
      warn-key:                            # Half B
        period: "1h"
        amount: "1.00"

trace:
  log_sink: true                           # trace events go to the log at target paladin::trace
  persist: true                            # and to the run_traces table (readable with sqlite3)

herald:                                    # without this section no herald line is printed (§5.6);
  default_formatter: "json"                # every field below is required
  json:
    pretty: true
    include_metadata: true
  markdown:
    include_colors: true
    heading_level: 2
  table:
    max_column_width: 60
    border_style: "rounded"
```

The adapters read their settings from the environment, not from the `llm.<provider>` block: for
Ollama that is `OLLAMA_BASE_URL` (default `http://localhost:11434/v1`), so the block above only
has to load.

Boot coherence will stop the server, naming the path, if `ci-runner` is not a key name under
`http.auth.api_keys`, if any key under `treasurer:` is misspelled, or if the run store is disabled.
`treasurer.pricing` keys are **model ids**, so for Option 2 use the exact id the provider expects.

Environment for the server shell (the run store, waypoint store, webhooks and secret are
env-only; the run queue already defaults to in-memory):

```bash
cd uat41
export PALADIN_CONFIG=./config.yml
export APP_RUN_STORE_BACKEND=sqlite
export APP_RUN_STORE_PATH=sqlite://./runs.db
export APP_WAYPOINT_STORE_BACKEND=sqlite                    # required whenever a run store is set
export APP_WAYPOINT_STORE_SQLITE_PATH=sqlite://./waypoints.db
export APP_WEBHOOKS_ALLOW_PRIVATE=true                     # loopback receiver
export APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET=op-secret-41  # signs operator notices
export RUST_LOG=info,paladin::trace=info,paladin::herald=info
# Option 2 only: export OPENAI_API_KEY=sk-...   (or the provider's variable)
```

---

## 3. Start the receiver and the server

Terminal 1, the operator webhook receiver (records raw body bytes so the HMAC can be verified):

```bash
mkdir -p uat41/hooks && cd uat41/hooks
python3 - <<'EOF'
import http.server, itertools, time
count = itertools.count(1)                 # two deliveries in one second must not share a file
class H(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        n = int(self.headers.get("Content-Length", 0)); body = self.rfile.read(n)
        stamp = f"{time.strftime('%Y%m%dT%H%M%S')}-{next(count):03d}"
        open(f"delivery-{stamp}.body", "wb").write(body)
        open(f"delivery-{stamp}.headers", "w").write(str(self.headers))
        print("POST", self.path, dict(self.headers), body.decode(), flush=True)
        self.send_response(200); self.end_headers()
    def log_message(self, *a): pass
http.server.HTTPServer(("127.0.0.1", 9099), H).serve_forever()
EOF
```

Terminal 2, the server (same env as §2):

```bash
cd uat41
../target/release/paladin-server 2>&1 | tee server.log
```

Expected on boot: no `treasurer.allowance` error; `curl -s localhost:8080/health` answers 200.
If boot fails naming `treasurer.allowance.webhook.url`, `APP_WEBHOOKS_ALLOW_PRIVATE` is unset.
If it fails with "no waypoint store is wired", the two `APP_WAYPOINT_STORE_*` variables are unset.

---

## 4. Half A: the refusal (no LLM call)

### 4.1 Seed the ledger to exactly the ceiling

The ledger balance is the sum of `amount_nanos` over rows in the window. Insert one settle row
for `ci-runner` worth 2.50 USD, attributed now. The timestamp must use the RFC 3339 form the
server itself stores (`YYYY-MM-DDTHH:MM:SS+00:00`), because the window predicate is a text
comparison.

```bash
cd uat41
sqlite3 runs.db <<'SQL'
INSERT INTO treasury_ledger
  (entry_id, kind, tenant_id, api_key_id, reservation_id, run_id, superstep, attempt,
   amount_nanos, charged_nanos, currency, model_breakdown, attributed_at, recorded_at, schema_version)
VALUES
  ('uat41-seed-' || lower(hex(randomblob(8))), 'settle', 'acme', 'ci-runner', NULL,
   'uat41-seed-run', 0, 1,
   2500000000, 2500000000, 'USD', '{}',
   strftime('%Y-%m-%dT%H:%M:%S','now') || '+00:00',
   strftime('%Y-%m-%dT%H:%M:%S','now') || '+00:00', 'v1');
SQL
```

`2500000000` nano-units is 2.5000 USD. Do this inside one UTC hour; if the top of the hour is
less than five minutes away, wait for it (the window is `[floor(now/1h), +1h)`).

### 4.2 Confirm the balance through the CLI (store clock, same file)

```bash
WINDOW_START=$(date -u +%Y-%m-%dT%H:00:00Z)
../target/release/paladin-cli treasury spend --api-key ci-runner --since "$WINDOW_START" --group-by api-key
```

Expected: one row for `ci-runner`, `2.5000 USD`. Record this figure.

### 4.3 Submit a run as ci-runner: expect 429

```bash
curl -s -i -X POST localhost:8080/v1/runs \
  -H 'X-API-Key: uat41-ci-runner-secret' -H 'Content-Type: application/json' \
  -d '{"assistant_id":"walkthrough","input":{"input":"Say hello"}}' | tee refusal.txt
```

Tick each of these against `refusal.txt`:

- [ ] status line `HTTP/1.1 429`
- [ ] header `retry-after: N` where `N` is a whole number of seconds and equals the seconds left
      in the current UTC hour (within a few seconds of `3600 - (date -u +%s) % 3600`)
- [ ] body `error.code == "allowance_exhausted"`
- [ ] `error.details` has exactly `scope` (`api_key`), `kind` (`window`), `balance`
      (`2.5000 USD`), `ceiling` (`2.5000 USD`), `window_start`, `window_end` (one hour apart)
- [ ] the raw body contains neither `acme`, nor `ci-runner`, nor the key value
- [ ] `balance` in the body equals the CLI figure from 4.2

The agent's task text is `input.input`; with a bare `"input":{}` the model is sent the literal
text `{}`.

Nothing was persisted:

```bash
curl -s localhost:8080/v1/runs -H 'X-API-Key: uat41-ci-runner-secret' | jq '.items | length'   # 0
sqlite3 runs.db 'select count(*) from runs;'                                                     # 0
```

### 4.4 Control: a key with no allowance in the same tenant is admitted

```bash
curl -s -o /dev/null -w '%{http_code}\n' -X POST localhost:8080/v1/runs \
  -H 'X-API-Key: uat41-control-secret' -H 'Content-Type: application/json' \
  -d '{"assistant_id":"walkthrough","input":{"input":"Say hello"}}'
```

Expected `202`. This run **does execute** and calls the model (Option 1: Ollama, free; Option 2:
one small live call). If you want Half A to make zero model calls, skip 4.4; it is a control, not
a success criterion. Note the run's spend lands under `svc-control`, never under `ci-runner`.

---

## 5. Half B: the warn path (exactly one admitted run executes)

Half B uses its own key, `warn-key` (already in the §2 config with a 1.00 USD hourly allowance),
so Half A's seed does not interfere and no restart is needed. In the shell you run the
commands from, set `WARN_KEY` to the `key` value of the `warn-key` entry in `uat41/config.yml`
(`read -r WARN_KEY` and paste it); the commands below send it as the `X-API-Key` header.

### 5.1 Seed to exactly 80% of the ceiling

```bash
sqlite3 runs.db <<'SQL'
INSERT INTO treasury_ledger
  (entry_id, kind, tenant_id, api_key_id, reservation_id, run_id, superstep, attempt,
   amount_nanos, charged_nanos, currency, model_breakdown, attributed_at, recorded_at, schema_version)
VALUES
  ('uat41-warn-' || lower(hex(randomblob(8))), 'settle', 'acme', 'warn-key', NULL,
   'uat41-warn-seed', 0, 1,
   800000000, 800000000, 'USD', '{}',
   strftime('%Y-%m-%dT%H:%M:%S','now') || '+00:00',
   strftime('%Y-%m-%dT%H:%M:%S','now') || '+00:00', 'v1');
SQL
```

0.8000 USD of a 1.0000 USD ceiling is the 80% crossing; `warn_at: 80` fires at `>=`.

Option 2 variant (live provider, no seed): set `amount: "0.01"` and `warn_at: 50` instead, and let
two or three real short runs cross 50%; then the warning appears on whichever run crosses it.
The assertions below are the same, only the figures differ.

### 5.2 Submit one run as warn-key: expect 202 and let it finish

```bash
RUN_JSON=$(curl -s -X POST localhost:8080/v1/runs \
  -H "X-API-Key: $WARN_KEY" -H 'Content-Type: application/json' \
  -d '{"assistant_id":"walkthrough","input":{"input":"Say hello"}}')
echo "$RUN_JSON"; RUN_ID=$(echo "$RUN_JSON" | jq -r .run_id); THREAD_ID=$(echo "$RUN_JSON" | jq -r .thread_id)
until curl -s localhost:8080/v1/runs/$RUN_ID -H "X-API-Key: $WARN_KEY" | jq -e '.status|test("completed|failed|halted|cancelled")' >/dev/null; do sleep 2; done
curl -s localhost:8080/v1/runs/$RUN_ID -H "X-API-Key: $WARN_KEY" | jq '{status, cost}'
```

- [ ] the submit answered `202` (a warning never blocks the run, D-15)
- [ ] the run reaches `completed`. If it reaches `failed`, read `error`: for Option 2 this is
      where a provider key, quota or model-access problem surfaces; fix it and rerun 5.1 to 5.2
      with a fresh key name.

### 5.3 One durable notice row

```bash
sqlite3 -header runs.db "select scope_kind, limit_kind, balance_nanos, ceiling_nanos, warn_at, run_id from treasury_notices where api_key_id='warn-key';"
```

- [ ] exactly one row: `api_key`, `window`, `800000000`, `1000000000`, `80`, `run_id == $RUN_ID`

### 5.4 One signed operator webhook, twelve keys

The delivery drain polls every few seconds; wait up to 15 s, then in Terminal 1's directory:

```bash
ls delivery-*.body | wc -l                                   # 1
B=$(ls delivery-*.body | tail -1); H=${B%.body}.headers
jq -S 'keys' "$B"
grep -iE 'x-paladin-(event|signature|delivery)' "$H"
printf 'sha256=%s\n' "$(openssl dgst -sha256 -hmac 'op-secret-41' "$B" | awk '{print $2}')"
```

- [ ] exactly one delivery
- [ ] keys are exactly: `api_key_id, balance, ceiling, event, kind, run_id, scope, tenant_id,
      timestamp, warn_at, window_end, window_start` (twelve, the option-b set)
- [ ] `event == "allowance_warning"`, `run_id == $RUN_ID`, `tenant_id == "acme"`,
      `api_key_id == "warn-key"` (the name, not the secret)
- [ ] `X-Paladin-Event: allowance_warning`; `X-Paladin-Signature` equals the HMAC you computed
- [ ] the body contains no key value, no run input and no `op-secret-41`
- [ ] `curl -s localhost:8080/v1/runs/$RUN_ID/webhook-deliveries -H "X-API-Key: $WARN_KEY"`
      lists nothing: operator notices are not the caller's

### 5.5 One trace event, before RunStarted

```bash
grep -c '"kind":"allowance_warning"' server.log                                         # 1 (log sink)
sqlite3 runs.db "select seq, json_extract(record,'$.kind') from run_traces where thread_id='$THREAD_ID' order by seq;"
```

Match on `"kind":"allowance_warning"`: the herald line of §5.6 also contains the bare string
`"allowance_warning"` (as a JSON key), so the looser pattern counts 2. Expected rows:
`1|allowance_warning`, `2|run_started`, `3|node_started`, `4|node_finished`, `5|run_finished`.

- [ ] exactly one `allowance_warning` record for the run, and its `seq` is lower than the
      `run_started` record's

### 5.6 One herald line

```bash
grep -c 'allowance:' server.log      # 1, on the paladin::herald summary of this run
grep 'allowance:' server.log
```

- [ ] exactly one `allowance:` line, naming the percent and figures, no tenant id and no key name

### 5.7 Second admission in the same window: nothing new

Repeat 5.2 on a **new thread** (omit `thread_id`, as above) and let it finish, then re-run 5.3,
5.4, 5.5 and 5.6.

- [ ] the second run answered `202` and completed (still under the ceiling)
- [ ] `treasury_notices` still has one row; `notices_for_run` of the second run is empty
- [ ] the receiver still has exactly one delivery
- [ ] no second `allowance_warning` trace record, and the second herald summary has no
      `allowance:` line

The second run's trace rows start at `run_started`, with no `allowance_warning` row.

If the second run's spend pushes the balance to or past 1.0000 USD, a **third** submit answers
`429` with `balance >= ceiling`, which is Half A again and is fine to note.

---

## 6. Recording the result

In `41-UAT.md`:

- **Test 1** `result:` passed when every box in §4 and §5 is ticked; otherwise `issue`, with the
  failing box and the raw response or log line pasted under `## Gaps`.
- **Test 2** (CI gates) is read from GitHub, not from this machine: on the pushed branch, the
  `coverage` job reports a line figure at or above 82 percent and the `postgres-integration`
  job's log contains no `SKIP:` lines. Passed or issue accordingly.
- **Test 3** (the D-05 over-admission race) and **Test 4** (the two backstop truths) are
  acceptance reads: open `.planning/decisions/0056-allowance-admission-model.md`, confirm the
  Decision section records the race and its Phase 42 closure and the claim-then-insert crash
  window, and mark passed if you accept them as designed.

Then `/gsd-verify-work 41` walks the file and marks the phase complete when all four pass.

---

## 7. Cleanup

```bash
# stop both terminals, then
rm -rf uat41/runs.db* uat41/waypoints.db* uat41/hooks uat41/server.log
```

Rotate or delete the `uat41-*` API keys and the `op-secret-41` webhook secret; they appear in this
runbook and in `uat41/config.yml`. Nothing in the repository was changed by the walkthrough.

---

## Appendix: why the figures are what they are

- `Retry-After` is `window_end - evaluated_at` in whole seconds, both from the ledger store's
  clock (`strftime('now')` in SQLite), never the web process clock (D-01, C8).
- `balance` and `ceiling` are display strings from `format_cost` with four decimals.
- The window is tumbling and UTC-epoch aligned, so a key can spend up to twice its allowance
  across one boundary; this is accepted and documented (ADR-0056).
- The notice row is claimed at admission, before the run row is inserted, and discarded on
  `abandon`; a crash between claim and insert can lose one window's notice, never duplicate it
  (UAT test 4).
