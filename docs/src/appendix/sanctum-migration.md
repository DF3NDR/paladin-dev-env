# Sanctum Migration Guide

Guide for migrating Sanctum memory storage between adapters, upgrading infrastructure, and managing data transitions.

## Table of Contents

- [Migration Scenarios](#migration-scenarios)
- [InMemory to Qdrant Migration](#inmemory-to-qdrant-migration)
- [Qdrant Version Upgrades](#qdrant-version-upgrades)
- [Changing Vector Dimensions](#changing-vector-dimensions)
- [Zero-Downtime Migration](#zero-downtime-migration)
- [Rollback Procedures](#rollback-procedures)
- [Data Validation](#data-validation)
- [Troubleshooting](#troubleshooting)

## Migration Scenarios

### Common Migration Paths

1. **Development to Production**: InMemory → Qdrant
2. **Scaling Up**: Local Qdrant → Qdrant Cluster
3. **Cloud Migration**: Self-hosted → Qdrant Cloud
4. **Dimension Change**: 384 → 1536 dimensions (model upgrade)
5. **Version Upgrade**: Qdrant v1.6 → v1.7

## InMemory to Qdrant Migration

### Overview

Migrate from ephemeral InMemory storage to persistent Qdrant for production use.

### Prerequisites

- Running Qdrant instance (local, cluster, or cloud)
- Sufficient storage capacity
- Matching embedding model dimensions
- Paladin application with both adapters available

### Migration Steps

#### Step 1: Export from InMemory

Create an export utility:

```rust,ignore
// src/bin/export_sanctum.rs
use paladin_ports::output::sanctum_port::{SanctumPort, SanctumFilter};
use paladin::core::platform::container::sanctum::SanctumEntry;
use std::fs::File;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize InMemory adapter with existing data
    let in_memory = InMemorySanctum::new();

    // Export all memories
    let filter = SanctumFilter::new(); // No filter = all memories
    let count = in_memory.count(Some(filter.clone())).await?;
    println!("Exporting {} memories...", count);

    // For InMemory, we need to implement an export method
    // This is a simplified example
    let memories = export_all_memories(&in_memory).await?;

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&memories)?;
    let mut file = File::create("sanctum_export.json")?;
    file.write_all(json.as_bytes())?;

    println!("Export complete: {} memories written to sanctum_export.json", memories.len());
    Ok(())
}

async fn export_all_memories(
    sanctum: &dyn SanctumPort
) -> Result<Vec<SanctumEntry>, Box<dyn std::error::Error>> {
    // Implementation depends on your specific setup
    // May need to add export methods to SanctumPort trait
    todo!("Implement export logic")
}
```

**Serialized Format**:

```json
{
  "version": "1.0",
  "exported_at": "2024-01-30T10:00:00Z",
  "total_entries": 10000,
  "entries": [
    {
      "memory": {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "paladin_id": "paladin-123",
        "content": "User asked about Rust programming",
        "memory_type": "Episodic",
        "importance": 0.8,
        "access_count": 5,
        "created_at": "2024-01-30T09:00:00Z",
        "last_accessed": "2024-01-30T09:30:00Z",
        "metadata": {}
      },
      "embedding": [0.1, -0.2, 0.3, ...]
    }
  ]
}
```

#### Step 2: Set Up Qdrant

**Option A: Docker**

```bash
docker run -d \
  --name paladin-qdrant \
  -p 6333:6333 -p 6334:6334 \
  -v $(pwd)/qdrant_storage:/qdrant/storage \
  qdrant/qdrant:v1.7.4
```

**Option B: Kubernetes**

```bash
kubectl apply -f k8s/qdrant-statefulset.yaml
```

**Option C: Qdrant Cloud**

Sign up at https://qdrant.to/cloud and create a cluster.

Verify connectivity:

```bash
curl http://localhost:6333/health
# Expected: {"title":"qdrant - vector search engine","version":"1.7.4"}
```

#### Step 3: Configure Paladin for Qdrant

Update configuration:

```yaml
# config.yml
sanctum:
  enabled: true
  adapter_type: "qdrant"
  qdrant:
    url: "http://localhost:6334"
    collection_name: "migrated_memories"
    vector_dimension: 1536  # Match your embeddings
```

Or via environment variables:

```bash
export APP_SANCTUM_ADAPTER_TYPE=qdrant
export APP_SANCTUM_QDRANT_URL=http://localhost:6334
export APP_SANCTUM_QDRANT_COLLECTION_NAME=migrated_memories
export APP_SANCTUM_QDRANT_VECTOR_DIMENSION=1536
```

#### Step 4: Import to Qdrant

Create an import utility:

```rust,ignore
// src/bin/import_sanctum.rs
use paladin::infrastructure::adapters::sanctum::QdrantSanctumAdapter;
use paladin::core::platform::container::sanctum::SanctumEntry;
use std::fs::File;
use std::io::Read;

#[derive(Deserialize)]
struct ExportData {
    version: String,
    total_entries: usize,
    entries: Vec<SanctumEntry>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read export file
    let mut file = File::open("sanctum_export.json")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let export: ExportData = serde_json::from_str(&contents)?;
    println!("Importing {} memories...", export.total_entries);

    // Initialize Qdrant adapter
    let qdrant = QdrantSanctumAdapter::new(
        "http://localhost:6334",
        "migrated_memories",
        1536,
    ).await?;

    // Import in batches for efficiency
    let batch_size = 100;
    for chunk in export.entries.chunks(batch_size) {
        qdrant.store_batch(chunk.to_vec()).await?;
        println!("Imported batch of {} memories", chunk.len());
    }

    // Verify count
    let count = qdrant.count(None).await?;
    println!("Import complete! Total memories in Qdrant: {}", count);

    if count != export.total_entries {
        eprintln!("WARNING: Count mismatch! Expected {}, got {}",
                  export.total_entries, count);
    }

    Ok(())
}
```

Run the import:

```bash
cargo run --bin import_sanctum
```

Expected output:

```
Importing 10000 memories...
Imported batch of 100 memories
Imported batch of 100 memories
...
Import complete! Total memories in Qdrant: 10000
```

#### Step 5: Validate Migration

Run validation checks:

```rust,ignore
// src/bin/validate_migration.rs
use paladin::infrastructure::adapters::sanctum::QdrantSanctumAdapter;
use paladin_ports::output::sanctum_port::{SanctumPort, SanctumQuery};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let qdrant = QdrantSanctumAdapter::new(
        "http://localhost:6334",
        "migrated_memories",
        1536,
    ).await?;

    // 1. Count check
    let total = qdrant.count(None).await?;
    println!("✓ Total memories: {}", total);

    // 2. Sample search test
    let test_embedding = vec![0.1; 1536]; // Dummy embedding
    let query = SanctumQuery::new(test_embedding, 5);
    let results = qdrant.search(query).await?;
    println!("✓ Search returned {} results", results.len());

    // 3. Specific memory retrieval
    // Test with a known memory ID from export
    println!("✓ Validation complete!");

    Ok(())
}
```

#### Step 6: Switch Production Traffic

**Graceful Cutover**:

1. Deploy new Paladin version with Qdrant configuration
2. Monitor for errors in logs
3. Compare search results between old and new
4. Gradually increase traffic to new adapter

**Configuration Update**:

```bash
# Update environment and restart
kubectl set env deployment/paladin \
  APP_SANCTUM_ADAPTER_TYPE=qdrant \
  APP_SANCTUM_QDRANT_URL=http://qdrant:6334

kubectl rollout status deployment/paladin
```

#### Step 7: Cleanup

After successful validation:

```bash
# Remove export file
rm sanctum_export.json

# Stop old InMemory instances
# Update documentation
# Remove InMemory-specific code if no longer needed
```

### Migration Checklist

- [ ] Export all memories from InMemory adapter
- [ ] Verify export file integrity and count
- [ ] Deploy Qdrant infrastructure
- [ ] Test Qdrant connectivity
- [ ] Configure Paladin for Qdrant
- [ ] Import memories in batches
- [ ] Validate total count matches
- [ ] Run sample searches
- [ ] Test specific memory retrieval
- [ ] Monitor application logs for errors
- [ ] Compare performance metrics
- [ ] Update production configuration
- [ ] Document new architecture
- [ ] Schedule backups
- [ ] Remove temporary export files

## Qdrant Version Upgrades

### Upgrade Path

Qdrant follows semantic versioning. Minor version upgrades (1.6 → 1.7) are generally safe.

### Upgrade Process

#### Step 1: Create Backup

```bash
# Create snapshot of all collections
curl -X POST http://localhost:6333/collections/paladin_memories/snapshots
```

#### Step 2: Test in Staging

Deploy new version to staging environment first:

```yaml
# docker-compose.staging.yml
services:
  qdrant-new:
    image: qdrant/qdrant:v1.7.4  # New version
    # ... rest of config
```

#### Step 3: Verify Compatibility

```bash
# Test with staging data
cargo test --test qdrant_integration
```

#### Step 4: Production Upgrade

**Blue-Green Deployment**:

1. Deploy new Qdrant instance (green)
2. Replicate data from old instance (blue)
3. Switch traffic to green
4. Monitor for issues
5. Decommission blue

**Rolling Update (Kubernetes)**:

```bash
kubectl set image statefulset/qdrant \
  qdrant=qdrant/qdrant:v1.7.4

kubectl rollout status statefulset/qdrant
```

## Changing Vector Dimensions

### Scenario

Upgrading embedding model (e.g., 384 → 1536 dimensions) requires re-embedding all content.

### Process

#### Step 1: Re-embed All Content

```rust,ignore
// src/bin/reembed_memories.rs
use paladin::infrastructure::adapters::sanctum::QdrantSanctumAdapter;
use paladin_ports::output::sanctum_port::SanctumPort;
use paladin_ports::output::embedding_port::EmbeddingPort;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Old adapter (384 dimensions)
    let old_qdrant = QdrantSanctumAdapter::new(
        "http://localhost:6334",
        "old_memories",
        384,
    ).await?;

    // New adapter (1536 dimensions)
    let new_qdrant = QdrantSanctumAdapter::new(
        "http://localhost:6334",
        "new_memories",
        1536,
    ).await?;

    // New embedding provider
    let embedding_service = OpenAIEmbeddingAdapter::new(...);

    // Re-embed and transfer
    let batch_size = 100;
    // ... implementation to fetch, re-embed, and store

    Ok(())
}
```

#### Step 2: Update Configuration

```yaml
sanctum:
  enabled: true
  adapter_type: "qdrant"
  qdrant:
    url: "http://localhost:6334"
    collection_name: "new_memories"  # New collection
    vector_dimension: 1536  # Updated dimension
```

#### Step 3: Cutover

Switch application to new collection and dimension.

## Zero-Downtime Migration

### Strategy: Dual-Write Pattern

Write to both old and new adapters simultaneously during migration.

```rust,ignore
pub struct DualWriteSanctum {
    primary: Arc<dyn SanctumPort>,
    secondary: Arc<dyn SanctumPort>,
}

#[async_trait]
impl SanctumPort for DualWriteSanctum {
    async fn store(&self, entry: SanctumEntry) -> Result<(), SanctumError> {
        // Write to both, but only require primary to succeed
        let primary_result = self.primary.store(entry.clone()).await;

        // Log secondary failures but don't fail the operation
        if let Err(e) = self.secondary.store(entry).await {
            warn!("Secondary write failed: {}", e);
        }

        primary_result
    }

    async fn search(&self, query: SanctumQuery) -> Result<Vec<SanctumSearchResult>, SanctumError> {
        // Always read from primary
        self.primary.search(query).await
    }

    // ... other methods
}
```

### Migration Steps with Dual-Write

1. **Phase 1: Dual-Write (Primary=Old, Secondary=New)**
   - Configure dual-write adapter
   - Deploy application
   - New writes go to both adapters
   - Reads come from old adapter

2. **Phase 2: Backfill Historical Data**
   - Run background job to copy old data to new adapter
   - Monitor progress

3. **Phase 3: Validation**
   - Compare counts
   - Spot-check search results
   - Validate data integrity

4. **Phase 4: Flip Primary**
   - Switch to Primary=New, Secondary=Old
   - Monitor for issues

5. **Phase 5: Remove Dual-Write**
   - Stop dual-write
   - Use only new adapter
   - Decommission old adapter

## Rollback Procedures

### Immediate Rollback

If critical issues occur during migration:

```bash
# Kubernetes
kubectl rollout undo deployment/paladin

# Docker Compose
docker-compose down
docker-compose -f docker-compose.old.yml up -d

# Environment variables
export APP_SANCTUM_ADAPTER_TYPE=in_memory  # Revert to old config
systemctl restart paladin
```

### Data Rollback

Restore from snapshot:

```bash
# List snapshots
curl http://localhost:6333/collections/paladin_memories/snapshots

# Recover from snapshot
curl -X PUT http://localhost:6333/collections/paladin_memories/snapshots/recover \
  -H "Content-Type: application/json" \
  -d '{"location": "snapshot-name"}'
```

### Validation After Rollback

```bash
# Verify service health
curl http://localhost:8080/health

# Check memory count
cargo run --bin count_memories

# Run smoke tests
cargo test --test smoke_test
```

## Data Validation

### Automated Validation Script

```rust,ignore
// src/bin/validate_sanctum.rs
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sanctum = initialize_adapter().await?;

    // 1. Count validation
    let count = sanctum.count(None).await?;
    assert!(count > 0, "No memories found");
    println!("✓ Count: {}", count);

    // 2. Search functionality
    let test_results = test_search(&sanctum).await?;
    assert!(!test_results.is_empty(), "Search returned no results");
    println!("✓ Search: {} results", test_results.len());

    // 3. Memory integrity
    for result in test_results.iter().take(10) {
        validate_memory(&result.entry.memory)?;
    }
    println!("✓ Memory integrity");

    // 4. Embedding dimensions
    let expected_dim = 1536;
    for result in test_results.iter().take(5) {
        assert_eq!(result.entry.embedding.len(), expected_dim,
                   "Embedding dimension mismatch");
    }
    println!("✓ Embedding dimensions");

    println!("\n✅ All validation checks passed!");
    Ok(())
}
```

### Manual Validation Checklist

- [ ] Total count matches expected
- [ ] Search returns relevant results
- [ ] All memory types present (Episodic, Semantic, Procedural)
- [ ] Importance scores in valid range (0.0-1.0)
- [ ] Timestamps are valid
- [ ] Metadata preserved
- [ ] Embedding dimensions correct
- [ ] No duplicate memories
- [ ] Performance within acceptable limits

## Troubleshooting

### Issue: Count Mismatch After Migration

**Problem**: Fewer memories in Qdrant than expected

**Solutions**:

1. Check import logs for errors:
   ```bash
   grep -i error import.log
   ```

2. Verify batch import completed:
   ```bash
   # Check Qdrant collection info
   curl http://localhost:6333/collections/paladin_memories
   ```

3. Re-run import for missing data:
   ```rust
   // Identify missing memories and re-import
   ```

### Issue: Search Returns Incorrect Results

**Problem**: Search results don't match expectations

**Solutions**:

1. Verify embedding dimensions match:
   ```yaml
   vector_dimension: 1536  # Must match embedding model
   ```

2. Check distance metric configuration:
   ```rust
   distance: Distance::Cosine  # Should match old setup
   ```

3. Rebuild HNSW index:
   ```bash
   curl -X POST http://localhost:6333/collections/paladin_memories/index
   ```

### Issue: Slow Import Performance

**Problem**: Import takes too long

**Solutions**:

1. Increase batch size:
   ```rust
   let batch_size = 500;  // Up from 100
   ```

2. Disable indexing during import:
   ```rust
   indexing_threshold: Some(0),  // Index after import complete
   ```

3. Use parallel imports:
   ```rust
   use futures::stream::StreamExt;

   futures::stream::iter(chunks)
       .for_each_concurrent(4, |chunk| async move {
           adapter.store_batch(chunk).await.unwrap();
       })
       .await;
   ```

### Issue: Out of Memory During Migration

**Problem**: Qdrant OOM killed during import

**Solutions**:

1. Reduce batch size:
   ```rust
   let batch_size = 50;  // Smaller batches
   ```

2. Enable quantization:
   ```rust
   quantization_config: Some(QuantizationConfig::Scalar(...))
   ```

3. Move vectors to disk temporarily:
   ```rust
   on_disk: true
   ```

4. Increase node resources:
   ```yaml
   resources:
     limits:
       memory: "16Gi"  # Increase from 8Gi
   ```

## Best Practices

1. **Always Backup First**: Create snapshots before any migration
2. **Test in Staging**: Never migrate production data untested
3. **Gradual Rollout**: Use blue-green or canary deployments
4. **Monitor Closely**: Watch metrics during and after migration
5. **Have Rollback Plan**: Know how to revert quickly
6. **Validate Thoroughly**: Don't assume migration succeeded
7. **Document Everything**: Record procedures and learnings
8. **Schedule Appropriately**: Migrate during low-traffic periods

## Support

For migration assistance:
- GitHub Issues: [paladin-dev-env/issues](https://github.com/DF3NDR/paladin-dev-env/issues)
- Qdrant Discord: https://qdrant.to/discord
- Qdrant Documentation: https://qdrant.tech/documentation/

---

**Next Steps**:
- [Deployment Guide](sanctum-deployment.md)
- [Main Documentation](../user-guides/sanctum-vector-memory.md)
- [Performance Tuning](../operations/performance-tuning.md)
