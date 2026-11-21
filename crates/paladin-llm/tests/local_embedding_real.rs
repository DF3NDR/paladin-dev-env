//! Real-weights tests for the offline local embedder. Every test here is `#[ignore]`d: it needs the
//! pinned `bge-small-en-v1.5` files and the pinned ONNX Runtime library on disk, and
//! `ORT_DISABLE_TELEMETRY=1` in the environment.
//!
//! ```text
//! ORT_DISABLE_TELEMETRY=1 \
//! PALADIN_LOCAL_EMBEDDER_DIR=/path/to/model-dir \
//! PALADIN_LOCAL_ORT_PATH=/path/to/libonnxruntime.so.1.31.0 \
//!   cargo test -p paladin-llm --features local-embeddings --test local_embedding_real -- --ignored
//! ```

use paladin_llm::local::embedding::{LocalEmbeddingAdapter, LocalEmbeddingConfig};
use paladin_ports::output::embedding_port::EmbeddingPort;
use std::path::PathBuf;
use std::time::Duration;

async fn adapter() -> LocalEmbeddingAdapter {
    let dir = std::env::var("PALADIN_LOCAL_EMBEDDER_DIR")
        .map(PathBuf::from)
        .expect("PALADIN_LOCAL_EMBEDDER_DIR");
    let runtime = std::env::var("PALADIN_LOCAL_ORT_PATH")
        .map(PathBuf::from)
        .expect("PALADIN_LOCAL_ORT_PATH");
    LocalEmbeddingAdapter::try_new(LocalEmbeddingConfig {
        model_dir: dir,
        runtime_path: runtime,
        intra_threads: Some(2),
    })
    .await
    .expect("the local embedder must load")
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[tokio::test]
#[ignore = "needs bge-small weights: set PALADIN_LOCAL_EMBEDDER_DIR and PALADIN_LOCAL_ORT_PATH"]
async fn real_vectors_are_384_unit_and_stable() {
    let adapter = adapter().await;
    let texts = [
        "a rule that checks a ghost variable",
        "an invariant over totalSupply",
    ];
    let first = adapter.embed_batch(&texts).await.expect("first run");
    let second = adapter.embed_batch(&texts).await.expect("second run");
    for (a, b) in first.iter().zip(&second) {
        assert_eq!(a.vector.len(), 384);
        let norm = a.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "norm {norm}");
        assert_eq!(a.vector, b.vector, "two runs must be identical");
        assert!(a.model.starts_with("local:BAAI/bge-small-en-v1.5@"));
    }
}

#[tokio::test]
#[ignore = "needs bge-small weights: set PALADIN_LOCAL_EMBEDDER_DIR and PALADIN_LOCAL_ORT_PATH"]
async fn related_text_outranks_unrelated() {
    let adapter = adapter().await;
    let query = adapter
        .embed_text("How do I write a CVL rule about token transfers?")
        .await
        .expect("query");
    let related = adapter
        .embed_text("A CVL rule states what must hold after a transfer call on the token.")
        .await
        .expect("related");
    let unrelated = adapter
        .embed_text("Bake the banana bread for forty minutes at a low heat.")
        .await
        .expect("unrelated");
    assert!(dot(&query.vector, &related.vector) > dot(&query.vector, &unrelated.vector));
}

/// Embeds twelve texts, keeps the adapter (and so the ONNX Runtime environment) alive for 150
/// seconds, then embeds once more. Run under the network-logging shim to prove the runtime makes
/// no outbound call over a window long enough for its telemetry uploader to have fired.
#[tokio::test]
#[ignore = "needs bge-small weights: set PALADIN_LOCAL_EMBEDDER_DIR and PALADIN_LOCAL_ORT_PATH"]
async fn holds_the_runtime_for_150_seconds() {
    let adapter = adapter().await;
    let owned: Vec<String> = (0..12)
        .map(|i| format!("sample text number {i} about a prover rule"))
        .collect();
    let texts: Vec<&str> = owned.iter().map(String::as_str).collect();
    assert_eq!(adapter.embed_batch(&texts).await.expect("twelve").len(), 12);
    tokio::time::sleep(Duration::from_secs(150)).await;
    assert_eq!(
        adapter
            .embed_text("one more after the hold")
            .await
            .expect("last")
            .vector
            .len(),
        384
    );
}
