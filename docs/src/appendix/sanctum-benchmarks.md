# Sanctum Benchmarks

## Overview

Performance benchmarks for the Sanctum long-term memory system measuring vector storage operations, semantic search, and filtering capabilities.

## Test Environment

- **Adapter**: InMemorySanctum (brute-force cosine similarity)
- **Vector Dimensions**: 384, 768, 1536 (common embedding sizes)
- **Test Data Scales**: 100 to 10,000 vectors
- **Hardware**: Results will show actual hardware

## Performance Targets

- **InMemory Adapter**: < 100ms search latency at 10,000 vectors
- **Qdrant Adapter** (shipped, behind the `qdrant` feature; benchmark numbers not yet captured):
  target < 500ms search latency at 100,000 vectors

## Benchmark Categories

### 1. Store Operations

#### Single Store
Measures latency for storing a single memory entry with embedding.

**Test Dimensions**: 384, 768, 1536

**Expected Results**:
- Low latency (< 1ms) for all dimensions
- Minimal variation across dimension sizes

#### Batch Store
Measures throughput for batch storage operations.

**Batch Sizes**: 10, 50, 100, 500 entries

**Expected Results**:
- Efficient batch processing
- Linear scaling with batch size
- Better throughput than individual stores

### 2. Vector Search

#### Search at Scale
Tests semantic search performance across different vector counts.

**Vector Counts**: 100, 1,000, 5,000, 10,000

**Search Parameters**:
- top_k: 10 results
- No filters

**Expected Results**:
- Linear O(n) complexity (brute-force)
- < 10ms @ 100 vectors
- < 50ms @ 1,000 vectors
- < 100ms @ 10,000 vectors ✅ **Target**

#### Top-K Variation
Tests impact of different result set sizes.

**Top-K Values**: 1, 5, 10, 50, 100
**Vector Count**: 5,000

**Expected Results**:
- Minor impact from result set size
- Dominant cost is similarity computation

#### Search with Filters
Tests filter overhead on search performance.

**Filters Tested**:
- No filter (baseline)
- Filter by `paladin_id`
- Filter by `memory_type`
- Filter by `min_importance`
- Combined filters (all three)

**Vector Count**: 5,000

**Expected Results**:
- Filters applied during similarity computation
- Minimal overhead for simple filters
- Slight overhead for combined filters

### 3. Update Operations

Measures latency for updating existing memory entries.

**Vector Count**: 1,000 pre-populated

**Expected Results**:
- Fast update (< 1ms)
- Replace operation in HashMap

### 4. Delete Operations

Measures latency for deleting memory entries.

**Vector Count**: 100 pre-populated

**Expected Results**:
- Fast delete (< 1ms)
- HashMap removal operation

### 5. Count Operations

Measures performance of counting entries with and without filters.

**Tests**:
- Count all (no filter)
- Count with combined filter

**Vector Count**: 5,000

**Expected Results**:
- Fast count without filter (HashMap len)
- Filter count requires iteration

## Benchmark Results

### Execution

```bash
cargo bench --bench sanctum_benchmarks
```

Results are saved to:
- `sanctum_benchmark_results.txt` - Full criterion output
- `target/criterion/` - HTML reports and historical data

### Performance Summary

Results will be populated after benchmark run

#### Store Operations

| Operation | Dimension | Time (avg) | Throughput |
|-----------|-----------|------------|------------|
| Single Store | 384 | - | - |
| Single Store | 768 | - | - |
| Single Store | 1536 | - | - |
| Batch (10) | 384 | - | - entries/sec |
| Batch (50) | 384 | - | - entries/sec |
| Batch (100) | 384 | - | - entries/sec |
| Batch (500) | 384 | - | - entries/sec |

#### Search Performance

| Vector Count | Time (avg) | Time (p95) | Status |
|--------------|------------|------------|--------|
| 100 | - | - | - |
| 1,000 | - | - | - |
| 5,000 | - | - | - |
| 10,000 | - | - | ✅ / ❌ Target < 100ms |

#### Search with Filters

| Filter Type | Time (avg) | Overhead |
|-------------|------------|----------|
| No filter | - | Baseline |
| paladin_id | - | - |
| memory_type | - | - |
| min_importance | - | - |
| Combined | - | - |

#### Other Operations

| Operation | Time (avg) |
|-----------|------------|
| Update | - |
| Delete | - |
| Count (all) | - |
| Count (filtered) | - |

## Analysis

### InMemory Adapter Characteristics

**Strengths**:
- Zero external dependencies
- Predictable latency
- Simple deployment
- Excellent for development and testing

**Limitations**:
- O(n) search complexity (brute-force)
- Memory bounded (recommended < 10K vectors)
- No persistence (lost on restart)
- Single-process only

**Recommended Use Cases**:
- Development and testing
- Small-scale deployments
- Short-lived sessions
- Embedded scenarios

### Performance Optimization Notes

1. **Vector Dimensions**: Higher dimensions increase computation but have minimal storage overhead
2. **Batch Operations**: Significant throughput gains with batching
3. **Filters**: Applied during search, minimal overhead for selective filters
4. **Capacity**: Performance degrades linearly beyond 10K vectors

### Future Optimizations

- SIMD for cosine similarity (potential 4-8x speedup)
- Approximate Nearest Neighbor (ANN) algorithms for > 10K vectors
- Memory mapping for larger-than-RAM datasets
- Multi-threaded search for high concurrency

## Qdrant Adapter (Benchmarks Not Yet Captured)

The Qdrant adapter ships today (`crates/paladin-memory/src/sanctum/qdrant_adapter.rs`, behind the
`qdrant` Cargo feature) — it is implemented, not future work. Its benchmark numbers have not been
captured yet, which is a different claim from the adapter being unimplemented. Additional
benchmarks will measure:

- **Large Scale**: 10K, 50K, 100K, 1M vectors
- **HNSW Performance**: Sub-100ms at 100K vectors
- **Concurrent Searches**: Multi-threaded throughput
- **Batch Upserts**: High-volume ingestion rates
- **Persistent Storage**: Disk I/O impact

## Viewing Results

### Terminal Output

```bash
cat sanctum_benchmark_results.txt
```

### HTML Reports

```bash
open target/criterion/sanctum_store_single/report/index.html
open target/criterion/sanctum_search_scale/report/index.html
```

### Comparison Across Runs

Criterion automatically tracks historical data and shows performance regressions/improvements.

```bash
# View all benchmark groups
ls target/criterion/
```

## Reproducing Benchmarks

```bash
# Clean build
cargo clean

# Run all Sanctum benchmarks
cargo bench --bench sanctum_benchmarks

# Run specific benchmark group
cargo bench --bench sanctum_benchmarks -- sanctum_search_scale

# Save baseline for comparison
cargo bench --bench sanctum_benchmarks -- --save-baseline my-baseline

# Compare against baseline
cargo bench --bench sanctum_benchmarks -- --baseline my-baseline
```

## Continuous Performance Monitoring

Integrate benchmarks into CI/CD:

```yaml
- name: Run Benchmarks
  run: cargo bench --bench sanctum_benchmarks -- --save-baseline ci-baseline

- name: Check for Regressions
  run: cargo bench --bench sanctum_benchmarks -- --baseline ci-baseline
```

Criterion will fail if performance regresses significantly.

---

**Last Updated**: TBD
**Benchmark Version**: Initial implementation
**Contact**: Paladin Development Team
