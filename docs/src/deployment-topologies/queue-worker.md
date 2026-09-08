# Queue / Worker (Distributed)

Decouple *requesting* an agent run from *executing* it: **producers enqueue jobs** onto a
Redis-backed queue, and a pool of **workers** dequeue and run them. This gives you
horizontal scale, backpressure under load, retries, and fault isolation — a slow or failing
worker doesn't block producers.

> The example below is compiled code pulled from the `paladin-doc-examples` crate via mdBook
> `{{#include}}`. The Redis calls compile but are not executed by the check gate, so it stays
> in sync with the `RedisQueueAdapter` API without needing a live Redis.

> **Prerequisites:** Run `make dev` (starts Redis) first, and enable the `redis-queue`
> feature on `paladin-storage`.

## When to choose it

- **Choose it when** you need scale-out across workers/hosts, backpressure for bursty load,
  automatic retries, or isolation between job execution and your request path.
- **Look elsewhere when** load is low and in-process execution suffices
  ([embedded library](embedded-library.md)), or you only need synchronous request/response
  ([HTTP service host](http-service-host.md)).

## Producer and worker

The producer enqueues a typed `AgentJob`; the worker dequeues it (as generic JSON), runs the
agent through a `PaladinExecutionService`, and marks the item complete:

```rust
{{#include ../../../crates/doc-examples/src/queue_worker.rs:queue}}
```

Run many workers — in this process via several `tokio` tasks, or as separate processes across
hosts — all pulling from the same queue. `start_processing` / `complete_processing` /
`fail_processing` track each item's lifecycle, and failures can retry up to the configured
limit.

## Configuring the queue

`RedisQueueConfig` is typically populated from `config.yml`:

```yaml
queue:
  redis_host: "localhost"
  redis_port: 6379
  redis_db: 0
  connection_timeout: 30
  key_prefix: "paladin:queue"
  max_retries: 3
```

## Run server: producer API + worker replicas

The Platform API (v0.10, [`docs/src/api-reference/platform-api.md`](../api-reference/platform-api.md))
is this same queue/worker shape applied to `POST /runs`: `paladin-server`'s API replicas are the
producer — `POST /runs` performs one repository insert and one queue enqueue, with no engine work
on the request path — and a separate `paladin-worker` Deployment is the consumer, dequeuing and
driving `WarEngine` through `RunWorkerPool`.
[`k8s/server/worker-deployment.yaml`](https://github.com/DF3NDR/paladin-dev-env/blob/main/k8s/server/worker-deployment.yaml)
is a worked example: same image as the API Deployment, `APP_RUN_STORE_BACKEND=postgres`,
`APP_RUN_QUEUE_BACKEND=redis` and `APP_RUN_WORKER_CONCURRENCY=4` set so its pods only consume
work — see [`k8s/README.md`](https://github.com/DF3NDR/paladin-dev-env/tree/main/k8s#worker-replicas-platform-api-v010)
for the manifest and the Secret it needs.

A few properties of this split are worth stating plainly rather than assuming:

- **The API replicas and the worker replicas share the SAME run store and the SAME Redis queue.**
  There is exactly one source of truth for a run's status (the store) and exactly one dispatch
  path onto a worker (the queue) — an API pod answering `GET /runs/{id}` and a worker pod driving
  that same run are reading/writing the identical row, never a per-pod copy.
- **Cancellation is cross-instance by construction.** `POST /runs/{id}/cancel` writes a durable
  flag through the shared repository first; a worker on ANY instance — not necessarily the one
  that happens to be running that thread — observes the flag at the next superstep boundary via a
  `CancellationProbe`. Scaling worker replicas does not weaken cancellation.
- **The in-process auth token store is still single-replica-scoped (ADR-0041), and a worker/API
  split does not change that.** `paladin-worker` pods never serve the `/v1` auth-gated routes at
  all, so they add no new exposure to this limitation — it is entirely a property of however many
  `paladin-server` API replicas are running `http.auth.bearer_token.enabled: true` at once. Run
  ONE API replica if you need that credential path, or terminate auth upstream (a gateway/ingress
  issuing its own tokens) if you need more than one — the static-API-key path
  (`http.auth.api_keys`, the shipped default) has no such limitation, since the keys are
  byte-identical in every pod.

## See also

- Standing up Redis and the adapter in detail —
  [Redis Queue Adapter Setup](../appendix/redis-queue-adapter-setup.md).
- Each worker is itself an [embedded](embedded-library.md) agent host; a worker can also run a
  [Battalion](battalion-orchestration.md) as its unit of work.

---

← Back to [Choosing a topology](overview.md)
