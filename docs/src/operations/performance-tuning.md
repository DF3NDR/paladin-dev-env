# Performance Tuning Guide

Comprehensive guide for optimizing Paladin performance across different workloads and deployment scenarios.

## Table of Contents

- [Performance Baselines](#performance-baselines)
- [Benchmarking](#benchmarking)
- [LLM Optimization](#llm-optimization)
- [Memory Optimization](#memory-optimization)
- [Concurrency Tuning](#concurrency-tuning)
- [Database Optimization](#database-optimization)
- [Network Optimization](#network-optimization)
- [Resource Allocation](#resource-allocation)

## Performance Baselines

### Expected Performance

| Metric | Target | Acceptable | Action Required |
|--------|--------|------------|-----------------|
| **Throughput** | ≥10 req/s | ≥5 req/s | <5 req/s |
| **P95 Latency** | <2s | <5s | >5s |
| **Memory per Paladin** | <50MB | <100MB | >100MB |
| **CPU per Paladin** | <100m | <200m | >200m |
| **Error Rate** | <0.1% | <1% | >1% |

### Benchmark Results

**Unverified against the current tree.** `benches/` today contains two real benchmark files:
`config_benchmarks.rs` (benchmarking `Settings::new()` and domain config accessors) and
`engine_benchmarks.rs` (the `WarEngine` superstep-cost and `SqliteWaypointStore::save` overhead
benchmarks — ENG-NFR-01/02, see the [Superstep Engine guide](../user-guides/superstep-engine.md)
for what it measures). No Garrison, Battalion, or Herald benchmark file exists to reproduce the
figures below (`benches/BENCHMARK_FIXES.md` records that `garrison_benchmarks.rs`,
`battalion_benchmarks.rs`, `paladin_benchmarks.rs`, and `arsenal_benchmarks.rs` were drafted for
a past task but never fixed to compile, and are absent from the tree now — that part of the
record still holds). The figures below are left in place as a historical measurement claim, not
re-derived or invented; see `config_benchmarks.rs` and `engine_benchmarks.rs` for benchmarks
reproducible today.

**Garrison Memory Operations (Measured - January 2026):**

Single Entry Operations:
- Add entry (10 chars): ~170 ns
- Add entry (100 chars): ~210 ns
- Add entry (1000 chars): ~225 ns
- Add entry (10000 chars): ~380 ns

Batch Operations:
- Add 10 entries: ~1.05 µs (105 ns/entry)
- Add 50 entries: ~4.2 µs (84 ns/entry)
- Add 100 entries: ~8.0 µs (80 ns/entry)
- Add 500 entries: ~37.5 µs (75 ns/entry)

Retrieval Operations:
- Get last 10 entries: ~33 ns
- Get last 50 entries: ~46 ns
- Get all (100 entries): ~55 ns

Eviction Strategies:
- FIFO eviction: ~280 ns/eviction
- SlidingWindow eviction: ~295 ns/eviction

Realistic Conversation (10 turns, 20 messages): ~3.35 µs

**Battalion Orchestration (Measured - January 2026):**

Formation (Sequential):
- 3 Paladins (10ms latency): ~30 ms total
- 5 Paladins (10ms latency): ~50 ms total
- 10 Paladins (10ms latency): ~100 ms total

Phalanx (Concurrent):
- 3-20 Paladins (10ms latency): ~10 ms total (parallel)

Orchestration Overhead (Zero Latency):
- Formation (5 Paladins): ~1.8 µs pure overhead
- Phalanx (5 Paladins): ~25 µs pure overhead

Aggregation Strategies:
- CollectAll: ~25 µs
- FirstSuccess: ~2.6 µs
- Majority: ~25 µs

**Herald Output Formatting (Measured - January 2026):**

- JSON (1KB): ~2.3 µs
- Markdown (1KB): ~570 ns (fastest)
- Table (1KB): ~5.5 µs
- JSON (10KB): ~10 µs
- Markdown (10KB): ~2.3 µs
- Table (10KB): ~23 µs

**Key Insights:**
- Garrison operations are sub-microsecond (extremely fast)
- Batch operations show ~25% performance improvement
- Battalion orchestration overhead is negligible vs LLM latency
- Markdown formatting is 2-4x faster than JSON
- All orchestration overhead < 100µs (LLM calls dominate at 1-5s)

## Benchmarking

### Running Benchmarks

```bash
# All benchmarks
cargo bench

# Specific benchmark
cargo bench config_benchmarks

# With baseline comparison
cargo bench --bench config_benchmarks -- --save-baseline v0.8.0
cargo bench --bench config_benchmarks -- --baseline v0.8.0

# Generate HTML report
cargo bench --bench config_benchmarks -- --plotting-backend gnuplot
```

### Custom Benchmarks

```rust,ignore
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn paladin_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let paladin = create_test_paladin();

    c.bench_function("paladin execution", |b| {
        b.to_async(&rt).iter(|| async {
            let result = paladin.execute(black_box("test input")).await;
            black_box(result)
        })
    });
}

criterion_group!(benches, paladin_benchmark);
criterion_main!(benches);
```

### Load Testing

```bash
# Using Apache Bench
ab -n 1000 -c 10 -T 'application/json' \
  -p request.json \
  http://localhost:8080/api/paladin/execute

# Using k6
k6 run --vus 10 --duration 30s load-test.js
```

## LLM Optimization

### Model Selection

```yaml
# Use appropriate model for task complexity
llm:
  model_routing:
    simple_tasks:
      model: "gpt-3.5-turbo"  # 5-10x faster than GPT-4
      max_tokens: 500

    complex_tasks:
      model: "gpt-4"
      max_tokens: 2000

    classification:
      model: "gpt-3.5-turbo"  # Sufficient for most classification
      temperature: 0.1
```

### Request Batching

```rust,ignore
// Batch similar requests
pub struct LlmBatcher {
    pending: Vec<LlmRequest>,
    max_batch_size: usize,
    max_wait_time: Duration,
}

impl LlmBatcher {
    pub async fn add_request(&mut self, request: LlmRequest) -> Result<LlmResponse> {
        self.pending.push(request);

        if self.pending.len() >= self.max_batch_size {
            return self.flush().await;
        }

        // Wait for more requests or timeout
        tokio::select! {
            _ = tokio::time::sleep(self.max_wait_time) => {
                self.flush().await
            }
        }
    }

    async fn flush(&mut self) -> Result<Vec<LlmResponse>> {
        let batch = std::mem::take(&mut self.pending);
        self.llm_port.generate_batch(batch).await
    }
}
```

### Caching Responses

`moka` is not a dependency anywhere in this workspace (`grep -n moka Cargo.toml` → 0 hits) —
illustrative only.

```rust,ignore
use moka::future::Cache;

pub struct CachedLlmPort {
    inner: Arc<dyn LlmPort>,
    cache: Cache<String, LlmResponse>,
}

impl CachedLlmPort {
    pub fn new(port: Arc<dyn LlmPort>, max_capacity: u64) -> Self {
        Self {
            inner: port,
            cache: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(Duration::from_secs(3600))
                .build(),
        }
    }

    async fn generate_cached(&self, messages: &[Message]) -> Result<LlmResponse> {
        let key = compute_cache_key(messages);

        if let Some(cached) = self.cache.get(&key).await {
            return Ok(cached);
        }

        let response = self.inner.generate(messages).await?;
        self.cache.insert(key, response.clone()).await;
        Ok(response)
    }
}
```

### Streaming for Long Responses

```rust,ignore
// Use streaming to reduce perceived latency
pub async fn execute_with_streaming(
    paladin: &Paladin,
    input: &str,
) -> Result<impl Stream<Item = String>> {
    let stream = paladin.execute_stream(input).await?;

    Ok(stream.map(|chunk| {
        // Process chunk immediately
        format!("Received: {}\n", chunk.content)
    }))
}
```

## Memory Optimization

### Garrison Configuration

**Corrected 2026-08-24.** The real `GarrisonSettings` fields
(`crates/paladin-memory/src/config/garrison.rs:11-26`) are `garrison_type` (not `type`;
defaults `"in_memory"`, max_entries 100, max_tokens 4000), `path`, `max_entries`, `max_tokens`,
`tokenizer`, `eviction_strategy` (`"importance_based"` (default) | `"fifo"` | `"sliding_window"`
— not the string `"sliding"`), and `preserve_recent_count`. There is no `windowing:` or
`cleanup:` block anywhere in the config surface.

```yaml
# Optimize memory usage
garrison:
  garrison_type: "sqlite"
  max_entries: 500              # Reduce from default 100
  max_tokens: 4000              # Same as default
  eviction_strategy: "sliding_window"
  preserve_recent_count: 10     # Keep last 10 entries regardless of eviction
```

### Memory Pooling

```rust,ignore
use tokio::sync::RwLock;

pub struct MemoryPool<T> {
    pool: RwLock<Vec<T>>,
    factory: Box<dyn Fn() -> T + Send + Sync>,
}

impl<T> MemoryPool<T> {
    pub async fn acquire(&self) -> T {
        let mut pool = self.pool.write().await;
        pool.pop().unwrap_or_else(|| (self.factory)())
    }

    pub async fn release(&self, item: T) {
        let mut pool = self.pool.write().await;
        if pool.len() < 100 {  // Max pool size
            pool.push(item);
        }
    }
}
```

### Lazy Loading

```rust,ignore
// Load garrison entries on-demand
pub struct LazyGarrison {
    session_id: Uuid,
    cache: RwLock<Option<Vec<GarrisonEntry>>>,
    repository: Arc<dyn GarrisonRepository>,
}

impl LazyGarrison {
    pub async fn get_entries(&self) -> Result<Vec<GarrisonEntry>> {
        let cache = self.cache.read().await;
        if let Some(entries) = cache.as_ref() {
            return Ok(entries.clone());
        }

        drop(cache);
        let entries = self.repository.load(self.session_id).await?;
        *self.cache.write().await = Some(entries.clone());
        Ok(entries)
    }
}
```

## Concurrency Tuning

### Thread Pool Configuration

```rust,ignore
use tokio::runtime::Builder;

pub fn create_runtime() -> Runtime {
    Builder::new_multi_thread()
        .worker_threads(8)              // Match CPU cores
        .max_blocking_threads(16)       // For blocking operations
        .thread_name("paladin-worker")
        .thread_stack_size(3 * 1024 * 1024)  // 3MB stack
        .build()
        .unwrap()
}
```

### Concurrency Limits

**Corrected 2026-08-24.** There is no top-level `paladin:` or `battalion:` config key anywhere
in `Settings` (consistent with 16-04's `docker.md` finding), so `max_concurrent_executions` and
`battalion.phalanx.max_concurrent_paladins` do not correspond to any real config surface. The
`arsenal:` key is real (`ArsenalConfig`, `src/config/arsenal.rs:31-38`), but the timeout field is
named `default_timeout_seconds`, not `tool_timeout`:

```yaml
# Control concurrent tool execution (the only one of these three that is real config)
arsenal:
  max_concurrent_tools: 10
  default_timeout_seconds: 30
```

### Backpressure Handling

```rust,ignore
use tokio::sync::Semaphore;

pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
}

impl RateLimiter {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub async fn acquire(&self) -> Result<()> {
        match self.semaphore.acquire().await {
            Ok(permit) => {
                permit.forget();  // Release on drop
                Ok(())
            }
            Err(_) => Err(Error::RateLimitExceeded),
        }
    }
}
```

## Database Optimization

### SQLite Configuration

```sql
-- Optimize SQLite for performance
PRAGMA journal_mode = WAL;           -- Write-Ahead Logging
PRAGMA synchronous = NORMAL;         -- Balance safety/speed
PRAGMA cache_size = -64000;          -- 64MB cache
PRAGMA temp_store = MEMORY;          -- In-memory temp tables
PRAGMA mmap_size = 268435456;        -- 256MB memory-mapped I/O
PRAGMA page_size = 4096;             -- Optimal page size

-- Corrected 2026-08-24: garrison_entries has no session_id column (the real scoping column
-- is paladin_id); an index on it already ships as idx_paladin_timestamp
-- (migrations/001_create_garrison_tables.sql:18-19). GIN/to_tsvector is PostgreSQL syntax —
-- this project's Garrison store is SQLite (garrison_type: "sqlite"), which has no GIN index
-- type. Full-text search already ships via a real SQLite FTS5 virtual table instead:
-- CREATE VIRTUAL TABLE garrison_search USING fts5(content, content='garrison_entries', ...)
-- (same migration file, lines 31-36), kept in sync by triggers, not by a manual index.
```

### Connection Pooling

```rust,ignore
use sqlx::sqlite::SqlitePoolOptions;

pub async fn create_pool(database_url: &str) -> Result<SqlitePool> {
    SqlitePoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await?
}
```

### Query Optimization

**Corrected 2026-08-24:** `garrison_entries` has no `session_id` column; the real scoping
column is `paladin_id` (`migrations/001_create_garrison_tables.sql:7-16`).

```rust,ignore
// Use prepared statements
let stmt = sqlx::query!(
    "SELECT * FROM garrison_entries
     WHERE paladin_id = ? AND timestamp > ?
     ORDER BY timestamp DESC
     LIMIT ?",
    paladin_id,
    cutoff_time,
    limit
);

// Batch inserts
let mut tx = pool.begin().await?;
for entry in entries {
    sqlx::query!(
        "INSERT INTO garrison_entries (paladin_id, content, timestamp)
         VALUES (?, ?, ?)",
        entry.paladin_id, entry.content, entry.timestamp
    )
    .execute(&mut *tx)
    .await?;
}
tx.commit().await?;
```

## Network Optimization

### Connection Reuse

```rust,ignore
use reqwest::Client;

// Reuse HTTP client
lazy_static! {
    static ref HTTP_CLIENT: Client = Client::builder()
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
}
```

### Compression

**Corrected 2026-08-24.** `ServerConfig` (`src/config/web_server.rs:17-20`) has exactly two
fields, `host` and `port` — no `compression:` block exists anywhere in the config surface. This
snippet describes a feature that is not implemented; treat it as illustrative only, not shipped
config.

```yaml
# Illustrative only — no compression config exists in ServerConfig today
server:
  compression:
    enabled: true
    level: 6              # Balance between size and CPU
    min_size: 1024        # Only compress responses > 1KB
```

### HTTP/2 and Keep-Alive

```rust,ignore
let client = reqwest::Client::builder()
    .http2_prior_knowledge()      // Use HTTP/2
    .tcp_keepalive(Duration::from_secs(60))
    .pool_max_idle_per_host(10)
    .build()?;
```

## Resource Allocation

### Kubernetes Resource Tuning

```yaml
resources:
  requests:
    cpu: "1000m"        # Guaranteed
    memory: "2Gi"
  limits:
    cpu: "4000m"        # Allow bursting
    memory: "4Gi"       # Hard limit

# Horizontal Pod Autoscaler
autoscaling:
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

### JVM-Style Tuning (for context)

```bash
# Rust doesn't need JVM tuning, but consider:

# 1. Release build optimizations
cargo build --release

# 2. Profile-guided optimization (PGO)
cargo build --profile production

# 3. Link-time optimization
[profile.release]
lto = "fat"
codegen-units = 1
```

### Monitoring Resource Usage

`sysinfo` is not a dependency anywhere in this workspace (`grep -n sysinfo Cargo.toml` → 0
hits), and this snippet's `info!(...)` call is `tracing`-shaped, not the real `log`-crate
facade (see `logging.md`'s scope note) — illustrative only.

```rust,ignore
use sysinfo::{System, SystemExt};

pub fn log_resource_usage() {
    let mut system = System::new_all();
    system.refresh_all();

    info!(
        cpu_usage = system.global_cpu_info().cpu_usage(),
        memory_used = system.used_memory(),
        memory_total = system.total_memory(),
        "Resource usage"
    );
}
```

## Performance Checklist

Before production deployment:

- [ ] Run benchmarks and verify targets met
- [ ] Profile CPU and memory usage under load
- [ ] Test with expected concurrency levels
- [ ] Verify database indexes exist
- [ ] Enable connection pooling
- [ ] Configure resource limits
- [ ] Set up monitoring and alerts
- [ ] Test auto-scaling behavior
- [ ] Optimize LLM model selection
- [ ] Enable response caching where appropriate

## Next Steps

- **[Monitoring](monitoring.md)** - Set up performance monitoring
- **[Troubleshooting](troubleshooting.md)** - Debug performance issues
- **[Production Best Practices](../deployment/production.md)** - Production readiness
