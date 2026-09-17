# Troubleshooting Guide

Common issues, diagnostic procedures, and solutions for Paladin deployments.

## Table of Contents

- [Diagnostic Tools](#diagnostic-tools)
- [Common Issues](#common-issues)
- [Performance Issues](#performance-issues)
- [Configuration Issues](#configuration-issues)
- [Deployment Issues](#deployment-issues)
- [Integration Issues](#integration-issues)
- [Getting Help](#getting-help)

## Diagnostic Tools

### Check Application Status

There is no `/metrics` endpoint in this codebase — `prometheus` is not a dependency anywhere in
the workspace and no `/metrics` route is registered (see `monitoring.md`'s scope note).
`opentelemetry` **is** a real, optional workspace dependency behind the `otel` Cargo feature (off
by default) — it exports OTLP trace spans, not Prometheus metrics, so it does not change this
page's no-`/metrics`-route conclusion; see [Distributed Tracing](monitoring.md#distributed-tracing)
and [Observability](observability.md) for that shipped path. A port `8081` claim was also
previously fabricated here; `Dockerfile:68` exposes `8080` (app) and `9090` (reserved for
metrics, unused). Only `/health` and `/ready` exist (`crates/paladin-web/src/health.rs`).

```bash
# Check liveness
curl http://localhost:8080/health

# Check readiness (agents loaded; shallow check, no network I/O)
curl http://localhost:8080/ready

# View logs
kubectl logs -f deployment/paladin -n paladin

# Check pod status
kubectl describe pod <pod-name> -n paladin
```

### Enable Debug Logging

There is no `logging:` YAML key anywhere in `Settings` (`grep -n logging src/config/settings.rs`
→ 0 hits; see `logging.md`'s scope note for the real facade — `log` + `env_logger`, not
`tracing`).

```bash
# Set environment variable (the real, only supported mechanism)
export RUST_LOG=debug,paladin=trace
```

### Collect Diagnostic Information

```bash
# System information
uname -a
rustc --version
cargo --version

# Application logs
kubectl logs deployment/paladin -n paladin --tail=1000 > paladin.log

# Configuration
kubectl get cm paladin-config -o yaml > config.yaml
```

## Common Issues

### 1. Paladin Execution Fails

**Symptoms:**
- `PaladinError::ExecutionError`
- Empty or truncated responses
- Timeout errors

**Diagnosis:**
```bash
# Check logs for error details
kubectl logs deployment/paladin | grep ERROR

# Check readiness — /health reports only {"status":"ok"}, no per-component detail;
# `jq .components.llm` below was fabricated (crates/paladin-web/src/health.rs)
curl http://localhost:8080/ready
```

**Solutions:**

**A. Invalid API Key**
```yaml
# Fix: Update secret with valid key
kubectl create secret generic paladin-secrets \
  --from-literal=openai-api-key="sk-..." \
  --dry-run=client -o yaml | kubectl apply -f -
```

**B. Model Not Found**
```rust,ignore
// Fix: Use valid model name
let paladin = PaladinBuilder::new(llm_port)
    .model("gpt-4")  // Not "gpt-4-invalid"
    .build()?;
```

**C. Rate Limiting**

**Corrected 2026-08-24:** `max_retries`/`timeout_seconds` exist on `LlmProviderConfig`
(`crates/paladin-llm/src/config/llm.rs:9-22`) but nested under the named provider block, not
flat under `llm:`; there is no `retry_delay` field anywhere.

```yaml
# Fix: Add retry logic and timeout on the specific provider block
llm:
  openai:
    max_retries: 3
    timeout_seconds: 60
```

### 2. High Memory Usage

**Symptoms:**
- OOMKilled pods
- Memory usage > 80%
- Slow performance

**Diagnosis:**
```bash
# Check memory usage
kubectl top pods -n paladin

# No /metrics endpoint exists (see Diagnostic Tools note) — check pod memory instead
```

**Solutions:**

**A. Garrison Too Large**

Defaults are `max_entries: 100`, `max_tokens: 4000` (`GarrisonSettings::default()`,
`crates/paladin-memory/src/config/garrison.rs:28-39`), not 1000/8000.

```yaml
# Fix: Reduce garrison limits below the defaults
garrison:
  max_entries: 50    # Reduce from default 100
  max_tokens: 2000    # Reduce from default 4000
```

**B. Memory Leak**
```bash
# Fix: Update to latest version
docker pull ghcr.io/your-org/paladin:latest
kubectl rollout restart deployment/paladin
```

**C. Insufficient Resources**
```yaml
# Fix: Increase resource limits
resources:
  limits:
    memory: 8Gi  # Increase from 4Gi
```

### 3. Connection Refused

**Symptoms:**
- Cannot connect to external services
- `ConnectionRefused` errors
- Network timeout

**Diagnosis:**
```bash
# Test connectivity from pod
kubectl exec -it <pod-name> -- curl http://redis:6379
kubectl exec -it <pod-name> -- nslookup redis

# Check network policies
kubectl get networkpolicy -n paladin
```

**Solutions:**

**A. Service Not Running**
```bash
# Fix: Start the service
kubectl get svc redis -n paladin
kubectl scale statefulset redis --replicas=1
```

**B. Wrong Hostname**

**Corrected 2026-08-24:** `QueueConfig` (`src/config/queue.rs:8-17`) has `redis_host`/
`redis_port` fields, not a single `url:` string.

```yaml
# Fix: Use correct service host/port
queue:
  redis_host: "redis.paladin.svc.cluster.local"
  redis_port: 6379
```

**C. Network Policy Blocking**
```yaml
# Fix: Allow egress to Redis
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-redis
spec:
  podSelector:
    matchLabels:
      app: paladin
  egress:
  - to:
    - podSelector:
        matchLabels:
          app: redis
    ports:
    - protocol: TCP
      port: 6379
```

### 4. Battalion Execution Hangs

**Symptoms:**
- Battalion never completes
- High CPU usage
- No error messages

**Diagnosis:**
```bash
# No /metrics endpoint exists (see Diagnostic Tools note) — check logs instead
# Look for deadlocks
kubectl logs deployment/paladin | grep -i "deadlock\|timeout"
```

**Solutions:**

**A. Circular Dependencies (Campaign)**
```rust,ignore
// Fix: Ensure DAG has no cycles
campaign.validate()?;  // Will error if cyclic
```

**B. Infinite Loop**
```rust,ignore
// Fix: Set reasonable max_loops
let paladin = PaladinBuilder::new(llm_port)
    .max_loops(10)  // Prevent infinite loops
    .build()?;
```

**C. Timeout Not Set**

**Corrected 2026-08-24:** there is no top-level `paladin:` config key. The real execution
timeout policy is `AgentTimeoutsConfig` (`src/config/agents.rs:18-25`), under `timeouts:`:

```yaml
# Fix: Add execution timeout
timeouts:
  default_seconds: 300  # 5 minutes
  max_seconds: 600
```

## Performance Issues

### Slow Response Times

**Symptoms:**
- P95 latency > 2s
- High request duration

**Diagnosis:**
```bash
# No /metrics endpoint exists (see Diagnostic Tools note) — profile directly instead
# Profile with flamegraph
cargo flamegraph --bin paladin-server
```

**Solutions:**

**A. Slow LLM Responses**

`default_model` and `timeout_seconds` (not `timeout`) are real, but nest under the named
provider block, matching the Rate Limiting fix above.

```yaml
# Fix: Use faster model or increase timeout
llm:
  openai:
    default_model: "gpt-3.5-turbo"  # Faster than gpt-4
    timeout_seconds: 30
```

**B. Garrison Query Slow**

**Corrected 2026-08-24:** `garrison_entries` has no `session_id` column — the real scoping
column is `paladin_id` (`migrations/001_create_garrison_tables.sql:7-16`), and an index on it
already ships (`idx_paladin_timestamp`, same migration file).

```sql
-- Fix: Add an index if a query pattern isn't covered by the shipped indexes
CREATE INDEX IF NOT EXISTS idx_garrison_paladin ON garrison_entries(paladin_id);
```

**C. Too Many Tool Calls**
```yaml
# Fix: Limit concurrent tool executions
arsenal:
  max_concurrent_tools: 5
```

### High CPU Usage

**Symptoms:**
- CPU throttling
- Slow processing
- Increased costs

**Diagnosis:**
```bash
# Check CPU usage
kubectl top pods -n paladin

# Profile CPU
cargo build --release
perf record -F 99 -g ./target/release/paladin-server
perf script | stackcollapse-perf.pl | flamegraph.pl > cpu.svg
```

**Solutions:**

**A. Too Many Replicas**
```yaml
# Fix: Reduce replica count
spec:
  replicas: 3  # Reduce from 10
```

**B. Inefficient Code**
```bash
# Fix: Update to optimized version
git pull origin main
cargo build --release
```

## Configuration Issues

### Invalid Configuration

**Symptoms:**
- Application won't start
- Configuration validation errors

**Diagnosis:**

**Corrected 2026-08-24:** there is no `config` subcommand on the shipped `paladin` CLI — the
top-level commands are `agent`, `battalion`, `arsenal`, `maneuver`, `onboarding`,
`setup-check`, `features`, `muster`, `council`, and others (`src/bin/paladin-cli.rs:32-`); none
of them validates a YAML file directly.

```bash
# Fix: check for syntax errors (config parsing errors surface at process startup)
yamllint config.yml
```

**Solutions:**

**Corrected 2026-08-24:** there is no top-level `paladin:` config key. `default_temperature`
and `max_loops` are real fields, but on per-agent `AgentDefinition` entries under `agents:`
(`src/config/agents.rs:209-229`), not a global `paladin:` block.

```yaml
# Fix: Correct YAML syntax (per-agent, under agents:)
agents:
  - id: "my-agent"
    model: "gpt-4"
    system_prompt: "..."
    temperature: 0.7  # Must be number
    max_loops: 3       # Must be integer
```

### Missing Environment Variables

**Symptoms:**
- `environment variable not set` errors
- API calls fail

**Diagnosis:**
```bash
# Check environment
kubectl exec deployment/paladin -- env | grep -i key
```

**Solutions:**
```bash
# Fix: Set missing variables
kubectl create secret generic paladin-secrets \
  --from-literal=openai-api-key="$OPENAI_API_KEY"
```

## Deployment Issues

### Pod CrashLoopBackOff

**Symptoms:**
- Pods constantly restarting
- `CrashLoopBackOff` status

**Diagnosis:**
```bash
# Check pod events
kubectl describe pod <pod-name> -n paladin

# View crash logs
kubectl logs <pod-name> -n paladin --previous
```

**Solutions:**

**A. Missing Dependencies**

**Corrected 2026-08-24:** the shipped `Dockerfile:48-50` runtime stage installs `libssl3`, not
the older `libssl1.1`.

```dockerfile
# Fix: Add runtime dependencies
RUN apt-get install -y libssl3 ca-certificates
```

**B. Health Check Failing**
```yaml
# Fix: Adjust health check timing
livenessProbe:
  initialDelaySeconds: 60  # Increase from 30
  periodSeconds: 30        # Increase from 10
```

### Image Pull Errors

**Symptoms:**
- `ImagePullBackOff` or `ErrImagePull`
- Pods stuck in pending

**Diagnosis:**
```bash
# Check image pull status
kubectl describe pod <pod-name> -n paladin | grep -A5 Events
```

**Solutions:**
```bash
# Fix: Authenticate with registry
kubectl create secret docker-registry ghcr-secret \
  --docker-server=ghcr.io \
  --docker-username=$GITHUB_USER \
  --docker-password=$GITHUB_TOKEN

# Update deployment to use secret
spec:
  imagePullSecrets:
  - name: ghcr-secret
```

## Integration Issues

### Redis Connection Failed

**Symptoms:**
- Queue operations fail
- `ConnectionRefused` errors

**Diagnosis:**
```bash
# Test Redis connectivity
kubectl exec deployment/paladin -- redis-cli -h redis ping
```

**Solutions:**
```bash
# Fix: Restart Redis
kubectl rollout restart statefulset redis

# Or check authentication
kubectl get secret redis-auth -o jsonpath='{.data.password}' | base64 -d
```

### MinIO/S3 Errors

**Symptoms:**
- File storage operations fail
- `AccessDenied` errors

**Diagnosis:**
```bash
# Test MinIO connectivity
kubectl exec deployment/paladin -- \
  curl -v http://minio:9000/minio/health/live
```

**Solutions:**
```bash
# Fix: Update credentials
kubectl create secret generic minio-credentials \
  --from-literal=access-key="minioadmin" \
  --from-literal=secret-key="minioadmin"
```

### LLM Provider Issues

**Symptoms:**
- API rate limiting
- Invalid credentials
- Model unavailable

**Solutions:**

**A. Rate Limit Exceeded**

**Corrected 2026-08-24:** there is no `rate_limit:` block anywhere in `LlmConfig` — no
requests-per-minute/tokens-per-minute config exists in this codebase. This is illustrative of a
feature not yet implemented, not shipped config.

**B. Switch Provider**

**Corrected 2026-08-24:** `LlmConfig` has no `providers:` list for fallback — it has
`default_provider: Option<String>` (singular) plus one optional block per named provider
(`openai`, `deepseek`, `anthropic`, etc.), each independently configured; there is no automatic
fallback-on-failure behavior.

```yaml
# Fix: configure the provider you want to switch to (no automatic fallback list)
llm:
  default_provider: "deepseek"
  deepseek:
    api_key: "${DEEPSEEK_API_KEY}"
```

## Getting Help

### Collect Debug Bundle

```bash
#!/bin/bash
# debug-bundle.sh

NAMESPACE="paladin"
OUTPUT="debug-bundle-$(date +%Y%m%d-%H%M%S).tar.gz"

mkdir -p debug-bundle
cd debug-bundle

# Logs
kubectl logs deployment/paladin -n $NAMESPACE > paladin.log

# Configuration
kubectl get all,cm,secrets -n $NAMESPACE -o yaml > resources.yaml

# Readiness snapshot (no /metrics endpoint exists — see Diagnostic Tools note)
curl http://localhost:8080/ready > readiness.txt

# Events
kubectl get events -n $NAMESPACE > events.txt

cd ..
tar czf $OUTPUT debug-bundle/
echo "Debug bundle created: $OUTPUT"
```

### Open an Issue

Include:
1. Paladin version
2. Deployment environment (Docker/K8s)
3. Error messages and logs
4. Steps to reproduce
5. Expected vs actual behavior

### Community Support

- **GitHub Issues**: Bug reports and feature requests
- **Discussions**: Questions and community help
- **Discord**: Real-time chat support

## Next Steps

- **[Monitoring](monitoring.md)** - Set up monitoring
- **[Performance Tuning](performance-tuning.md)** - Optimize performance
- **[Logging](logging.md)** - Configure logging
