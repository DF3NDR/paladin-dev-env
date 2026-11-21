//! Dependency-tree guard for the `local-embeddings` feature (D-11, T-07-42).
//!
//! The offline embedder must never be able to download a model: `fastembed` is built with its
//! default features off, so neither its hub client nor any HTTP client may appear in
//! `paladin-llm`'s normal dependency tree under `--no-default-features --features local-embeddings`.

use std::path::PathBuf;
use std::process::Command;

fn normal_tree_package_names() -> Vec<String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(cargo)
        .current_dir(&manifest_dir)
        .args([
            "tree",
            "-p",
            "paladin-llm",
            "--no-default-features",
            "--features",
            "local-embeddings",
            "-e",
            "normal",
            "--prefix",
            "none",
            "--offline",
        ])
        .output()
        .expect("cargo tree must start");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.split_whitespace().next().unwrap_or("").to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

#[test]
fn the_local_embedder_tree_has_no_hub_or_http_client() {
    let names = normal_tree_package_names();
    assert!(
        names.iter().any(|n| n == "fastembed"),
        "the tree must hold fastembed, got {} packages",
        names.len()
    );
    for banned in ["hf-hub", "ureq", "reqwest"] {
        assert!(
            !names.iter().any(|n| n == banned),
            "{banned} must not be in the local-embeddings dependency tree"
        );
    }
}
