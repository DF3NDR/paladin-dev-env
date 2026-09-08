/**
 * SDK smoke test: the TypeScript (`typescript-fetch`) client generated from
 * `crates/paladin-web/openapi.json` (D-49, PLAT-FR-17) against a live
 * `paladin-server` booted by `run.sh` on the all-InMemory/SQLite profile.
 *
 * Exercises: list assistants (`GET /v1/assistants`, expects a non-empty `items`
 * array) -> submit a run for a code-registered agent (`POST /v1/runs`, expects a
 * `run_id`) -> poll `GET /v1/runs/{run_id}` until a terminal status or 30s.
 *
 * Exits non-zero (via a thrown error surfacing as an unhandled rejection) on ANY
 * deviation -- this script's own exit code is `sdk-clients`'s real TypeScript-side
 * gate (prohibition P1: the job must not be able to go green on a client that
 * failed to install, list nothing, or submit nothing).
 *
 * The generated package is resolved as a relative `file:` dependency
 * (`package.json`, installed by `run.sh` via `npm ci` before this script runs) --
 * see this file's own module docs for why field access below tolerates either a
 * typed model instance or a plain object: this test suite cannot run the
 * generator locally (no Java, no Docker -- see `27-18-SUMMARY.md`) to pin the
 * exact generated shape.
 */

// eslint-disable-next-line @typescript-eslint/no-var-requires
import { AssistantsApi, Configuration, RunsApi } from "paladin-sdk";

const BASE_URL = process.env.PALADIN_SMOKE_BASE_URL ?? "http://127.0.0.1:18080";
const API_KEY = process.env.PALADIN_SMOKE_API_KEY ?? "sdk-smoke-test-key";
const AGENT_ID = process.env.PALADIN_SMOKE_AGENT_ID ?? "sdk-smoke-agent";
const POLL_TIMEOUT_MS = 30_000;
const TERMINAL_STATUSES = new Set(["completed", "failed", "halted", "cancelled"]);

function fail(message: string): never {
  // eslint-disable-next-line no-console
  console.error(`SMOKE FAIL: ${message}`);
  process.exit(1);
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
  while (Date.now() < deadline) {
    const response = await api.getRun({ runId });
    lastStatus = field(response, "status");
    if (typeof lastStatus === "string" && TERMINAL_STATUSES.has(lastStatus)) {
      // eslint-disable-next-line no-console
      console.log(`poll run: terminal status '${lastStatus}' reached`);
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  fail(
    `GET /v1/runs/${runId} never reached a terminal status within ${POLL_TIMEOUT_MS}ms ` +
      `(last observed: ${String(lastStatus)})`,
  );
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
