# Production Best Practices

Comprehensive checklist and guidelines for deploying Paladin in production environments.

## Table of Contents

- [Pre-Deployment Checklist](#pre-deployment-checklist)
- [Security](#security)
- [Performance](#performance)
- [Reliability](#reliability)
- [Monitoring](#monitoring)
- [Disaster Recovery](#disaster-recovery)
- [Cost Optimization](#cost-optimization)
- [Maintenance](#maintenance)

## Pre-Deployment Checklist

### Infrastructure

- [ ] **Compute resources** sized appropriately (CPU, memory)
- [ ] **High availability** configured (multiple replicas/zones)
- [ ] **Auto-scaling** enabled with appropriate thresholds
- [ ] **Load balancing** configured with health checks
- [ ] **Network policies** restrict unnecessary traffic
- [ ] **TLS/SSL** certificates configured and valid
- [ ] **DNS** properly configured with failover

### Configuration

- [ ] **Environment variables** properly set (no hardcoded secrets)
- [ ] **Configuration files** validated and tested
- [ ] **API keys** rotated and secured
- [ ] **Log levels** set appropriately (warn/error in prod)
- [ ] **Resource limits** configured (CPU, memory, connections)
- [ ] **Timeouts** set for all external calls
- [ ] **Rate limits** configured to prevent abuse

### Data

- [ ] **Database backups** automated and tested
- [ ] **Volume backups** scheduled and verified
- [ ] **Backup retention** policy defined (7d/30d/365d)
- [ ] **Disaster recovery** plan documented and tested
- [ ] **Data encryption** at rest and in transit
- [ ] **Access controls** properly configured

### Monitoring

- [ ] **Health checks** configured and responding
- [ ] **Metrics collection** enabled (Prometheus/Grafana)
- [ ] **Log aggregation** configured (ELK/Loki)
- [ ] **Alerting** rules defined for critical metrics
- [ ] **On-call rotation** established
- [ ] **Incident response** procedures documented
- [ ] **SLO/SLA** defined and monitored

### Testing

- [ ] **Load testing** performed at expected scale
- [ ] **Integration tests** passing in staging
- [ ] **Rollback procedure** tested
- [ ] **Canary deployment** strategy defined
- [ ] **Blue-green deployment** capability verified
- [ ] **Smoke tests** automated post-deployment

## Security

### Authentication & Authorization

> **Note:** the HTTP service host (`paladin-server`) has no OAuth2/Auth0 integration and no
> YAML-configurable `rbac.roles` scheme. Authentication is code-level, in
> `crates/paladin-web/src/agent_auth.rs`: an opaque server-issued **bearer token**
> (`Authorization: Bearer <token>`, verified via an injected `AuthPort`) checked first, falling
> back to an **`x-api-key` header** matched against a configured key map. There are exactly two
> roles (`crates/paladin-core/src/platform/container/user.rs:72-78`) — `Admin` and `User`
> (`#[default]`) — not three. `authorize_invoke(principal, allowed_roles)` enforces role checks
> per route; an empty `allowed_roles` list means any authenticated caller.

```
# Illustrative — there is no YAML config surface for this; it's wired in Rust when
# constructing AgentAuthConfig (token_verifier + api_keys map) and passed to the router.
Authorization: Bearer <opaque-token>     # verified by an injected AuthPort
x-api-key: <configured-key>              # fallback, looked up in AgentAuthConfig.api_keys
```

### API Key Management

```bash
# Rotate API keys regularly
OPENAI_API_KEY=$(vault kv get -field=api_key secret/openai)
DEEPSEEK_API_KEY=$(vault kv get -field=api_key secret/deepseek)

# Use separate keys for different environments
staging_key="sk-proj-staging-..."
production_key="sk-proj-prod-..."
```

### Network Security

```yaml
# Kubernetes NetworkPolicy
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: paladin-network-policy
spec:
  podSelector:
    matchLabels:
      app: paladin
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress-nginx
    ports:
    - protocol: TCP
      port: 8080
  egress:
  - to:
    - namespaceSelector: {}
    ports:
    - protocol: TCP
      port: 443  # HTTPS only
```

### Container Security

```dockerfile
# Use specific versions (not latest) — matches the live Dockerfile's builder stage
FROM rust:1.93-slim-bookworm AS builder

# Run as non-root user
USER paladin:paladin

# Read-only filesystem
docker run --read-only --tmpfs /tmp paladin

# Drop capabilities
docker run --cap-drop=ALL --cap-add=NET_BIND_SERVICE paladin
```

> **Security scanning:** `docker scan` was removed from the Docker CLI, and Snyk was
> evaluated and removed from this project on 2026-08-18 — it has no Rust coverage
> (`.github/instructions/security.instructions.md`, "Snyk was evaluated and removed"). Do not
> reintroduce either. Use the repo's actual tooling instead:
>
> ```bash
> make audit     # cargo-audit — vulnerable dependencies (RustSec advisory DB)
> make deny      # cargo-deny — licenses, bans, sources, advisories
> make security  # both of the above
> make sbom      # cargo-cyclonedx — dependency inventory (SBOM)
> ```

### Secrets Management

```bash
# Use external secrets managers
# Kubernetes External Secrets
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: paladin-secrets
spec:
  secretStoreRef:
    name: aws-secrets-manager
  target:
    name: paladin-secrets
  data:
  - secretKey: openai-api-key
    remoteRef:
      key: paladin/prod/openai-api-key

# HashiCorp Vault
vault kv put secret/paladin/prod \
  openai_api_key=sk-... \
  deepseek_api_key=...
```

## Performance

### Resource Allocation

```yaml
# Production resource configuration
resources:
  requests:
    cpu: 1000m      # 1 CPU guaranteed
    memory: 2Gi     # 2GB guaranteed
  limits:
    cpu: 4000m      # 4 CPU max
    memory: 8Gi     # 8GB max (OOM if exceeded)

# Horizontal Pod Autoscaler
autoscaling:
  enabled: true
  minReplicas: 5
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

### Connection Pooling

There is no `RedisConfig`/generic connection-pool-size field in this codebase — Redis
connectivity is configured via `QueueConfig` (`src/config/queue.rs:8-17`), which has no
`pool_size`/`idle_timeout`/`url` fields:

```rust,ignore
// Configure Redis (queue) via the real QueueConfig
let queue_config = QueueConfig {
    redis_host: "redis".into(),
    redis_port: 6379,
    redis_password: None,
    redis_db: 0,
    connection_timeout: Some(5),
    key_prefix: Some("paladin:queue".into()),
    max_retries: Some(3),
    enable_priority_queues: Some(true),
};

// Configure MinIO via the real FileStorageConfig
let file_storage_config = FileStorageConfig {
    minio_endpoint: "minio:9000".into(),
    minio_access_key: "minioadmin".into(),
    minio_secret_key: "minioadmin".into(),
    minio_bucket: "paladin-files".into(),
    minio_secure: Some(false),
    connection_timeout: Some(10),
    ..Default::default()
};
```

### Caching Strategy

> **Note:** there is no generic Redis-backed response cache and no `garrison.cache_embeddings`/
> `cache_ttl` field in this codebase (`GarrisonSettings`, `crates/paladin-memory/src/config/
> garrison.rs:11-26`, has exactly seven fields: `garrison_type`, `path`, `max_entries`,
> `max_tokens`, `tokenizer`, `eviction_strategy`, `preserve_recent_count` — no caching knobs).
> Garrison's own bounded-memory eviction is the closest real mechanism:

```yaml
garrison:
  garrison_type: "sqlite"
  max_entries: 1000
  max_tokens: 8000
  eviction_strategy: "importance_based"  # "importance_based" | "fifo" | "sliding_window"
  preserve_recent_count: 10
```

### LLM Optimization

> **Note:** there is no `model_routing` or `batching` config surface. Per-provider timeout and
> retry are real fields on `LlmProviderConfig` (`crates/paladin-llm/src/config/llm.rs:9-22`):

```yaml
llm:
  default_provider: "openai"
  openai:
    api_key: "${OPENAI_API_KEY}"
    default_model: "gpt-4"
    timeout_seconds: 30
    max_retries: 3
```

## Reliability

### Health Checks

The shipped routes are `/health` (liveness) and `/ready` (readiness) —
`crates/paladin-web/src/health.rs:34-35`; there is no `/health/live` or `/health/ready`.

```yaml
# Liveness probe (restart if fails) — always 200 once the process is up, no dependency checks
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 30
  periodSeconds: 10
  timeoutSeconds: 5
  failureThreshold: 3

# Readiness probe (remove from load balancer if fails) — shallow: 200 once the agent
# registry is built and serving, no network I/O against dependencies
readinessProbe:
  httpGet:
    path: /ready
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 5
  timeoutSeconds: 3
  failureThreshold: 3
  successThreshold: 1
```

### Graceful Shutdown

On SIGTERM/SIGINT, `paladin-server` cancels a `ShutdownCoordinator` shared with every
in-flight superstep run and waits up to a configured grace window for those runs to finish
before the process exits (HITL-04, D-21/D-22) — matching the live pattern at
`src/bin/paladin-server.rs` (`shutdown_signal`/`drain_on_shutdown`; `axum::Server` was removed
in Axum 0.7+, this workspace pins axum 0.8.4, and the current API binds a listener directly
and passes it to `axum::serve`):

```rust,ignore
use paladin::config::engine::EngineConfig;
use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use std::time::Duration;
use tokio::signal;

async fn wait_for_termination_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}

// Cancels the coordinator's root token, then waits <= grace for every registered
// in-flight run to drain -- or skips the wait entirely when APP_ENGINE_GRACEFUL_SHUTDOWN=false
// (the M-B-02 disable switch for legacy-only deployments).
async fn shutdown_signal(coordinator: ShutdownCoordinator, grace: Duration, graceful: bool) {
    wait_for_termination_signal().await;
    if graceful {
        let outcome = coordinator.cancel_and_wait(grace).await;
        tracing::info!("graceful shutdown drain complete: {outcome:?}");
    } else {
        coordinator.token().cancel();
    }
}

// In main:
let mut engine_config = EngineConfig::default();
engine_config.apply_env_overrides();
let coordinator = ShutdownCoordinator::new();
let listener = tokio::net::TcpListener::bind(&addr).await?;
axum::serve(listener, app.into_make_service())
    .with_graceful_shutdown(shutdown_signal(
        coordinator,
        Duration::from_secs(engine_config.shutdown_grace_secs),
        engine_config.graceful_shutdown,
    ))
    .await?;
```

**The rule: `terminationGracePeriodSeconds` must be at least twice the configured
`APP_ENGINE_SHUTDOWN_GRACE_SECS`.** With the 30-second default grace, that is 60 seconds — the
value both `k8s/server/deployment.yaml` and `k8s/deployment.yaml` ship — not the 30 seconds
this page previously showed, which would let the kubelet's SIGKILL deadline land while the
process is still mid-drain. If you raise `APP_ENGINE_SHUTDOWN_GRACE_SECS`, raise
`terminationGracePeriodSeconds` to at least twice that new value too.

| Env var | Default | Meaning |
|---|---|---|
| `APP_ENGINE_SHUTDOWN_GRACE_SECS` | `30` | Seconds the process waits, after SIGTERM/SIGINT, for in-flight superstep runs to finish before giving up on the stragglers. |
| `APP_ENGINE_GRACEFUL_SHUTDOWN` | `true` | Set to `false` to restore the legacy no-wait behavior (the M-B-02 disable switch). |

```yaml
# Kubernetes graceful termination — 60s = 2x the 30s default APP_ENGINE_SHUTDOWN_GRACE_SECS
spec:
  terminationGracePeriodSeconds: 60
  containers:
  - lifecycle:
      preStop:
        exec:
          command: ["/bin/sh", "-c", "sleep 15"]
```

### Circuit Breakers

Paladin ships a first-party circuit breaker (`src/infrastructure/resilience/circuit_breaker.rs`)
— there is no `circuit_breaker` external crate dependency in this workspace.

```rust,ignore
// Implement circuit breakers for external services
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;

// new(failure_threshold, success_threshold, timeout) — positional args, no Config struct
let llm_breaker = CircuitBreaker::new(5, 2, Duration::from_secs(60));

async fn call_llm_with_breaker(prompt: &str) -> Result<Response, PaladinError> {
    // call_async takes the future directly; returns Err(PaladinError::CircuitBreakerOpen)
    // immediately when the breaker is open, without invoking the future at all.
    llm_breaker.call_async(llm_client.generate(prompt)).await
}
```

### Retry Logic

There is no `backoff` crate dependency in this workspace. Retry/backoff is a first-party
policy — `RetryPolicy` (`crates/paladin-core/src/platform/container/battalion/mod.rs`) plus
the helper functions in `crates/paladin-battalion/src/retry.rs`:

```rust,ignore
// Implement exponential backoff using the shipped RetryPolicy
use paladin_battalion::retry::{calculate_retry_delay, should_retry};
use paladin_core::platform::container::battalion::RetryPolicy;

let policy = RetryPolicy {
    max_attempts: 3,
    base_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(10),
    exponential_backoff: true,
    jitter: true,
};

async fn call_with_retry<F, T>(policy: &RetryPolicy, f: F) -> Result<T, PaladinError>
where
    F: Fn() -> Result<T, PaladinError>,
{
    let mut attempt = 0;
    loop {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) if should_retry(policy, attempt) => {
                tokio::time::sleep(calculate_retry_delay(policy, attempt)).await;
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Monitoring

> **Scope note:** this codebase has no Prometheus/metrics-exporter crate dependency and no
> `/metrics` HTTP handler wired up anywhere (confirmed: `grep -rn 'metrics' crates/paladin-web/
> src/` finds no route; the shipped routes are `/health` and `/ready`,
> `crates/paladin-web/src/health.rs`). Every metric name and alert rule below is illustrative —
> none are actually exported today. `Dockerfile:68`/`k8s/deployment.yaml` reserve port 9090 for
> a future Prometheus endpoint that does not exist yet.

### Key Metrics

```yaml
# Illustrative — not currently exported. Also note: this project is Rust, not Go, so a real
# implementation would never emit `go_goroutines` (removed below; it does not apply here).
metrics:
  - paladin_requests_total          # Total requests
  - paladin_request_duration_seconds  # Request latency
  - paladin_errors_total            # Error count
  - paladin_active_paladins         # Active Paladins
  - garrison_entries_total          # Memory entries
  - arsenal_tool_calls_total        # Tool invocations

# System metrics
  - process_cpu_seconds_total       # CPU usage
  - process_resident_memory_bytes   # Memory usage

# External dependencies
  - llm_api_calls_total             # LLM API calls
  - llm_api_duration_seconds        # LLM latency
  - redis_operations_total          # Redis ops
  - minio_operations_total          # MinIO ops
```

### Alerting Rules

```yaml
# Illustrative — depends on the metrics pipeline above, which is not implemented yet.
# Prometheus alerting rules
groups:
- name: paladin
  interval: 30s
  rules:
  - alert: HighErrorRate
    expr: rate(paladin_errors_total[5m]) > 0.05
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High error rate detected"

  - alert: HighLatency
    expr: histogram_quantile(0.95, paladin_request_duration_seconds) > 2
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "High P95 latency (>2s)"

  - alert: PodCrashLooping
    expr: rate(kube_pod_container_status_restarts_total[15m]) > 0
    for: 15m
    labels:
      severity: critical
    annotations:
      summary: "Pod is crash looping"
```

### Logging Best Practices

```rust,ignore
// Structured logging with tracing
use tracing::{info, warn, error, instrument};

#[instrument(skip(paladin), fields(paladin_id = %paladin.id))]
async fn execute_paladin(paladin: &Paladin, input: &str) -> Result<PaladinResult> {
    info!("Starting paladin execution");

    match paladin.execute(input).await {
        Ok(result) => {
            info!(
                loops_used = result.loops_used,
                output_length = result.content.len(),
                "Paladin execution completed successfully"
            );
            Ok(result)
        }
        Err(e) => {
            error!(error = %e, "Paladin execution failed");
            Err(e)
        }
    }
}
```

> **Note:** there is no `logging:` YAML section, no file-output target, and no rotation config.
> Logging is controlled by two env vars, read directly by `SystemLogAdapterConfig`
> (`src/infrastructure/adapters/logs/system_log_adapter.rs:33-65`): `RUST_LOG` (level, e.g.
> `warn`/`info`/`debug`) and `SYSTEM_LOG_FORMAT` (`json` or `text`). Output goes to stdout;
> log rotation/aggregation is left to the orchestrator (e.g. a `docker-compose.yml`
> `logging.driver: "json-file"` block, as shown in
> [docker.md's Production Deployment section](docker.md#production-deployment), or a
> cluster-level log collector).

```bash
RUST_LOG=warn          # info in staging, warn in production
SYSTEM_LOG_FORMAT=json
```

## Disaster Recovery

### Backup Strategy

```bash
# Automated backups
# 1. Database backups
0 2 * * * /scripts/backup-garrison-db.sh

# 2. Volume snapshots
kubectl exec -n paladin deployment/backup -- \
  /scripts/snapshot-volumes.sh

# 3. Configuration backups
kubectl get all,cm,secrets -n paladin -o yaml > backup-$(date +%Y%m%d).yaml
```

### Recovery Testing

```bash
# Quarterly disaster recovery drill
1. Simulate complete cluster failure
2. Restore from backups
3. Verify data integrity
4. Measure RTO (Recovery Time Objective)
5. Measure RPO (Recovery Point Objective)
6. Document lessons learned
```

### Multi-Region Deployment

```yaml
# Deploy to multiple regions
regions:
  - name: us-east-1
    primary: true
    replicas: 5
  - name: eu-west-1
    primary: false
    replicas: 3
  - name: ap-southeast-1
    primary: false
    replicas: 3

# Cross-region replication
replication:
  garrison: async  # Eventual consistency
  citadel: sync    # Strong consistency for checkpoints
```

## Cost Optimization

### Resource Right-Sizing

```bash
# Analyze actual usage
kubectl top pods -n paladin
kubectl describe hpa paladin -n paladin

# Adjust based on metrics
resources:
  requests:
    cpu: 800m    # Reduced from 1000m
    memory: 1.5Gi  # Reduced from 2Gi
```

### Auto-Scaling Policies

```yaml
# Aggressive scale-down for cost savings
autoscaling:
  scaleDown:
    stabilizationWindowSeconds: 600  # 10 minutes
    policies:
    - type: Percent
      value: 50
      periodSeconds: 300
```

### Spot Instances

```yaml
# Use spot instances for non-critical workloads
nodeSelector:
  kubernetes.io/lifecycle: spot

tolerations:
- key: spot
  operator: Equal
  value: "true"
  effect: NoSchedule
```

## Maintenance

### Update Strategy

```yaml
# Rolling update configuration
strategy:
  type: RollingUpdate
  rollingUpdate:
    maxSurge: 1        # One extra pod during update
    maxUnavailable: 0  # Zero downtime
```

### Maintenance Windows

```bash
# Schedule maintenance during low-traffic periods
# Example: Sundays 2-4 AM UTC
0 2 * * 0 /scripts/maintenance.sh
```

### Dependency Updates

```bash
# Regular dependency updates
dependabot.yml:
  version: 2
  updates:
    - package-ecosystem: "cargo"
      directory: "/"
      schedule:
        interval: "weekly"
      open-pull-requests-limit: 10
```

## Checklist Summary

Use this checklist before each production deployment:

```markdown
## Pre-Deployment
- [ ] All tests passing (unit, integration, e2e)
- [ ] Code review completed and approved
- [ ] `make security` passed (`cargo-audit` + `cargo-deny`, no high/critical advisories)
- [ ] Performance benchmarks within acceptable range
- [ ] Documentation updated
- [ ] Changelog updated

## Deployment
- [ ] Backup current state
- [ ] Deploy to staging first
- [ ] Run smoke tests in staging
- [ ] Deploy to production using rolling update
- [ ] Monitor metrics during rollout
- [ ] Verify health checks passing

## Post-Deployment
- [ ] Run smoke tests in production
- [ ] Check error rates and latency
- [ ] Verify auto-scaling working
- [ ] Confirm backups running
- [ ] Update runbook if needed
- [ ] Notify stakeholders of successful deployment
```

## Next Steps

- **[Monitoring](../operations/monitoring.md)** - Detailed monitoring setup
- **[Troubleshooting](../operations/troubleshooting.md)** - Common issues and solutions
- **[Performance Tuning](../operations/performance-tuning.md)** - Optimization guide
