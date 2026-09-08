/**
 * SDK smoke test: the TypeScript (`typescript-fetch`) client generated from
 * `crates/paladin-web/openapi.json` (D-49, PLAT-FR-17) against a live
 * `paladin-server` booted by `run.sh` on the all-InMemory/SQLite profile.
 *
 * Exercises: list assistants (`GET /v1/assistants`, expects a non-empty `items`
 * array) -> submit a run for a code-registered agent (`POST /v1/runs`, expects a
 * `run_id`) -> poll `GET /v1/runs/{run_id}` until a terminal status or 30s.
 *
 * A TERMINAL status is not a PASS. Only `SUCCESS_STATUS` ("completed") is success
 * (PLAT-06 `precision`) -- the comparison is exact and case-sensitive, never a
 * prefix, substring or truthiness check. Any other terminal status (`failed`,
 * `halted`, `cancelled`) is a failure that names the status and prints the run's
 * own `error` text; a run still non-terminal at the poll deadline is a failure
 * that names the last observed status. Exits non-zero (via a thrown error
 * surfacing as an unhandled rejection) on ANY deviation -- this script's own exit
 * code is `sdk-clients`'s real TypeScript-side gate (prohibition P1: the job must
 * not be able to go green on a client that failed to install, list nothing,
 * submitted nothing, or observed a run that did not actually work).
 *
 * The generated package is installed separately, from its local build directory,
 * by `run.sh` (via `npm install --no-save` after `npm ci` -- see that file and
 * `package.json`'s own module docs for why this cannot be a `file:` dependency in
 * a committed lockfile) -- see this file's own module docs for why field access
 * below tolerates either a typed model instance or a plain object: this test
 * suite cannot run the generator locally (no Java, no Docker -- see
 * `27-18-SUMMARY.md`) to pin the exact generated shape.
 */

import { AssistantsApi, Configuration, RunsApi } from "paladin-sdk";

const BASE_URL = process.env.PALADIN_SMOKE_BASE_URL ?? "http://127.0.0.1:18080";
const API_KEY = process.env.PALADIN_SMOKE_API_KEY ?? "sdk-smoke-test-key";
const AGENT_ID = process.env.PALADIN_SMOKE_AGENT_ID ?? "sdk-smoke-agent";
const POLL_TIMEOUT_MS = 30_000;

// The single success status string (Task 3, PLAT-06 `precision`). A terminal
// status is not a pass on its own -- only this exact, case-sensitive value is.
const SUCCESS_STATUS = "completed";

// Every status that stops polling -- decides WHEN polling ends, never WHETHER
// the run succeeded. Success/failure is decided exclusively by
// `evaluateTerminalStatus` against `SUCCESS_STATUS`.
const TERMINAL_STATUSES = new Set(["completed", "failed", "halted", "cancelled"]);

function fail(message: string): never {
  // eslint-disable-next-line no-console
  console.error(`SMOKE FAIL: ${message}`);
  process.exit(1);
}

/**
 * The success decision, extracted into its own function -- mirrors
 * `smoke.py`'s `evaluate_terminal_status` so the two scripts' logs read
 * alike. A terminal status of exactly `SUCCESS_STATUS` is success (returns
 * normally). Any other terminal status -- or a non-terminal/unknown status
 * observed at the poll deadline -- is a failure that names both the observed
 * status and the run's own `error` text.
 */
function evaluateTerminalStatus(status: unknown, error: unknown): void {
  if (status === SUCCESS_STATUS) {
    return;
  }
  fail(`run did not reach '${SUCCESS_STATUS}' -- observed status ${String(status)}, error: ${String(error)}`);
}

function field(obj: unknown, name: string): unknown {
  if (obj && typeof obj === "object" && name in (obj as Record<string, unknown>)) {
    return (obj as Record<string, unknown>)[name];
  }
  return undefined;
}

function buildConfiguration(): Configuration {
  return new Configuration({
    basePath: BASE_URL,
    // The generated `Configuration` accepts the apiKey value directly for a
    // header-based `apiKey` security scheme (`components.securitySchemes.api_key`
    // in `crates/paladin-web/openapi.json`) -- the standard `typescript-fetch`
    // generator convention. `headers` is set too, belt-and-braces, so this
    // script's own correctness does not depend on that wiring alone.
    apiKey: API_KEY,
    headers: { "X-API-Key": API_KEY },
  });
}

async function listAssistants(config: Configuration): Promise<void> {
  const api = new AssistantsApi(config);
  const response = await api.listAssistants();
  const items = field(response, "items");
  if (!Array.isArray(items)) {
    fail(`GET /v1/assistants response has no 'items' array: ${JSON.stringify(response)}`);
  }
  // eslint-disable-next-line no-console
  console.log(`list assistants: ok, ${(items as unknown[]).length} item(s)`);
}

async function submitRun(config: Configuration): Promise<string> {
  const api = new RunsApi(config);
  const response = await api.submitRun({
    submitRunRequest: { assistantId: AGENT_ID, input: {} },
  });
  const runId = field(response, "runId") ?? field(response, "run_id");
  if (!runId) {
    fail(`POST /v1/runs response has no 'runId'/'run_id': ${JSON.stringify(response)}`);
  }
  // eslint-disable-next-line no-console
  console.log(`submit run: ok, run_id=${runId}`);
  return String(runId);
}

async function pollUntilTerminal(config: Configuration, runId: string): Promise<void> {
  const api = new RunsApi(config);
  const deadline = Date.now() + POLL_TIMEOUT_MS;
  let lastStatus: unknown = undefined;
  let lastError: unknown = undefined;
  while (Date.now() < deadline) {
    const response = await api.getRun({ runId });
    lastStatus = field(response, "status");
    lastError = field(response, "error");
    if (typeof lastStatus === "string" && TERMINAL_STATUSES.has(lastStatus)) {
      // eslint-disable-next-line no-console
      console.log(`poll run: terminal status '${lastStatus}' reached`);
      evaluateTerminalStatus(lastStatus, lastError);
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  // Deadline reached with no terminal status observed -- PLAT-06 `boundary`:
  // the deadline itself is a failure, never a silent pass.
  evaluateTerminalStatus(lastStatus, lastError);
}

async function main(): Promise<void> {
  const config = buildConfiguration();
  await listAssistants(config);
  const runId = await submitRun(config);
  await pollUntilTerminal(config, runId);
  // eslint-disable-next-line no-console
  console.log("SDK smoke (TypeScript): all checks passed.");
}

main().catch((error) => {
  fail(`unhandled error: ${error instanceof Error ? error.stack : String(error)}`);
});
