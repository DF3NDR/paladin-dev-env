//! Offline local embedder: the contract tests (RED). The adapter itself lands in the next commit.

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_ports::output::embedding_port::{EmbeddingError, EmbeddingPort};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    /// A backend that records each call's batch lengths and encodes every text's byte length as
    /// the hot position of an (unnormalised) 384-dimension vector.
    struct LengthBackend {
        calls: Arc<Mutex<Vec<Vec<usize>>>>,
        scale: f32,
    }

    impl EmbedBackend for LengthBackend {
        fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
            let lengths: Vec<usize> = texts.iter().map(|t| t.len()).collect();
            self.calls
                .lock()
                .map_err(|e| e.to_string())?
                .push(lengths.clone());
            Ok(lengths
                .iter()
                .map(|len| {
                    let mut v = vec![0.0_f32; 384];
                    v[*len % 384] = self.scale;
                    v
                })
                .collect())
        }
    }

    impl LocalEmbeddingAdapter {
        /// Test constructor: serve the port from a fake backend, no model files.
        fn with_backend(spec: &'static LocalModelSpec, backend: Box<dyn EmbedBackend + Send>) -> Self {
            Self {
                spec,
                identity: spec.model_name(),
                backend: Arc::new(Mutex::new(backend)),
            }
        }
    }

    fn fake(scale: f32) -> (LocalEmbeddingAdapter, Arc<Mutex<Vec<Vec<usize>>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let backend = LengthBackend {
            calls: Arc::clone(&calls),
            scale,
        };
        (
            LocalEmbeddingAdapter::with_backend(&LOCAL_BGE_SMALL_SPEC, Box::new(backend)),
            calls,
        )
    }

    /// Five model files in `dir`, and a spec whose pins are computed from them.
    fn fixture(dir: &Path) -> &'static LocalModelSpec {
        let names = [
            "onnx/model.onnx",
            "tokenizer.json",
            "config.json",
            "special_tokens_map.json",
            "tokenizer_config.json",
        ];
        let mut files = Vec::new();
        for (i, name) in names.iter().enumerate() {
            let bytes = format!("fixture bytes for {name} number {i}").into_bytes();
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(&path, &bytes).expect("write");
            files.push(ModelFileSpec {
                path: name,
                size: bytes.len() as u64,
                sha256: Box::leak(sha256_hex(&bytes).into_boxed_str()),
            });
        }
        Box::leak(Box::new(LocalModelSpec {
            model_id: "test/model",
            revision: "0123456789abcdef0123456789abcdef01234567",
            files: Box::leak(files.into_boxed_slice()),
            dimension: 384,
            max_length: 512,
            pooling: LocalPooling::Cls,
            normalize: true,
            query_prefix: None,
            runtime: RuntimeLibrarySpec {
                version: "0.0.0",
                file: "libonnxruntime.so.0.0.0",
                size: 1,
                sha256: "00",
            },
        }))
    }

    fn config_for(dir: &Path) -> LocalEmbeddingConfig {
        LocalEmbeddingConfig {
            model_dir: dir.to_path_buf(),
            runtime_path: PathBuf::from("/nonexistent/libonnxruntime.so"),
            intra_threads: Some(1),
        }
    }

    fn telemetry_off(name: &str) -> Option<String> {
        (name == "ORT_DISABLE_TELEMETRY").then(|| "1".to_string())
    }

    async fn load_err(spec: &'static LocalModelSpec, dir: &Path) -> String {
        match LocalEmbeddingAdapter::try_new_with_env(config_for(dir), spec, telemetry_off).await {
            Err(EmbeddingError::ProviderError(msg)) => msg,
            Err(other) => panic!("expected ProviderError, got {other:?}"),
            Ok(_) => panic!("loading must have been refused"),
        }
    }

    #[test]
    fn identity_carries_the_revision() {
        let name = LOCAL_BGE_SMALL_SPEC.model_name();
        let revision = name
            .strip_prefix("local:BAAI/bge-small-en-v1.5@")
            .expect("identity prefix");
        assert_eq!(revision.len(), 40, "{name}");
        assert!(revision.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
        assert!(name.len() <= 256);
        assert!(!name.chars().any(char::is_control));
        assert_eq!(LOCAL_BGE_SMALL_SPEC.dimension, 384);
        assert_eq!(LOCAL_BGE_SMALL_SPEC.max_length, 512);
        assert!(LOCAL_BGE_SMALL_SPEC.query_prefix.is_none());
        assert_eq!(LOCAL_BGE_SMALL_SPEC.files.len(), 5);
    }

    #[test]
    fn sha256_hex_matches_the_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[tokio::test]
    async fn batch_keeps_caller_order() {
        let (adapter, calls) = fake(3.0);
        let owned: Vec<String> = (0..20).map(|i| "x".repeat(((i * 7) % 19) + 1 + i)).collect();
        let texts: Vec<&str> = owned.iter().map(String::as_str).collect();
        let out = adapter.embed_batch(&texts).await.expect("batch");
        assert_eq!(out.len(), texts.len());
        for (text, embedding) in texts.iter().zip(&out) {
            assert_eq!(embedding.vector[text.len() % 384].round() as i32, 1, "hot slot");
        }
        let recorded = calls.lock().expect("calls").clone();
        assert!(recorded.iter().all(|c| !c.is_empty() && c.len() <= 8), "{recorded:?}");
        assert_eq!(recorded.iter().map(Vec::len).sum::<usize>(), 20);
        let flat: Vec<usize> = recorded.concat();
        let mut sorted = flat.clone();
        sorted.sort_unstable();
        assert_eq!(flat, sorted, "sub-batches run in length order");
    }

    #[tokio::test]
    async fn empty_text_is_invalid_input() {
        let (adapter, calls) = fake(1.0);
        assert!(matches!(
            adapter.embed_text("  ").await,
            Err(EmbeddingError::InvalidInput(_))
        ));
        assert!(matches!(
            adapter.embed_batch(&["fine", ""]).await,
            Err(EmbeddingError::InvalidInput(_))
        ));
        assert!(adapter.embed_batch(&[]).await.expect("empty batch").is_empty());
        assert!(calls.lock().expect("calls").is_empty(), "nothing reached the backend");
    }

    #[tokio::test]
    async fn vectors_are_unit_norm_and_sized() {
        let (adapter, _calls) = fake(7.5);
        let one = adapter.embed_text("hello").await.expect("embed");
        let identity = LOCAL_BGE_SMALL_SPEC.model_name();
        assert_eq!(one.model, identity);
        assert_eq!(adapter.model_name(), identity);
        assert_eq!(one.dimension, 384);
        assert_eq!(adapter.dimension(), 384);
        assert_eq!(one.vector.len(), 384);
        assert!(one.token_count.is_none());
        let norm = one.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "norm {norm}");
    }

    #[tokio::test]
    async fn refuses_without_telemetry_disabled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = fixture(dir.path());
        for lookup in [
            (|_: &str| None) as fn(&str) -> Option<String>,
            |name: &str| (name == "ORT_DISABLE_TELEMETRY").then(|| "0".to_string()),
            |name: &str| (name == "ORT_DISABLE_TELEMETRY").then(String::new),
        ] {
            match LocalEmbeddingAdapter::try_new_with_env(config_for(dir.path()), spec, lookup).await {
                Err(EmbeddingError::ProviderError(msg)) => {
                    assert!(msg.contains("ORT_DISABLE_TELEMETRY"), "{msg}")
                }
                other => panic!("expected a telemetry refusal, got {:?}", other.err()),
            }
        }
        // With the variable at 1 the telemetry check passes: the refusal that follows is about the
        // runtime library (missing), not about telemetry.
        let msg = load_err(spec, dir.path()).await;
        assert!(!msg.contains("ORT_DISABLE_TELEMETRY"), "{msg}");
    }

    #[tokio::test]
    async fn refuses_a_missing_weight_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = fixture(dir.path());
        std::fs::remove_file(dir.path().join("tokenizer.json")).expect("remove");
        let msg = load_err(spec, dir.path()).await;
        assert!(msg.contains("tokenizer.json"), "{msg}");
        assert!(msg.contains("setup"), "{msg}");
        assert!(msg.contains("downloaded"), "{msg}");
    }

    #[tokio::test]
    async fn refuses_a_flipped_byte() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = fixture(dir.path());
        let path = dir.path().join("config.json");
        let mut bytes = std::fs::read(&path).expect("read");
        bytes[3] ^= 0x01;
        std::fs::write(&path, &bytes).expect("write");
        let msg = load_err(spec, dir.path()).await;
        assert!(msg.contains("config.json"), "{msg}");
        let pinned = spec.files.iter().find(|f| f.path == "config.json").expect("pin").sha256;
        assert!(msg.contains(pinned), "pinned digest named: {msg}");
        assert!(msg.contains(&sha256_hex(&bytes)), "actual digest named: {msg}");
    }

    #[tokio::test]
    async fn refuses_a_wrong_size() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = fixture(dir.path());
        let path = dir.path().join("special_tokens_map.json");
        let mut bytes = std::fs::read(&path).expect("read");
        bytes.pop();
        std::fs::write(&path, &bytes).expect("write");
        let msg = load_err(spec, dir.path()).await;
        assert!(msg.contains("special_tokens_map.json"), "{msg}");
        assert!(msg.contains("size"), "{msg}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn refuses_a_symlinked_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = fixture(dir.path());
        let real = dir.path().join("tokenizer_config.json.real");
        let link = dir.path().join("tokenizer_config.json");
        std::fs::rename(&link, &real).expect("rename");
        std::os::unix::fs::symlink(&real, &link).expect("symlink");
        let msg = load_err(spec, dir.path()).await;
        assert!(msg.contains("tokenizer_config.json"), "{msg}");
        assert!(msg.contains("symlink"), "{msg}");
    }

    #[test]
    fn never_reads_ort_dylib_path() {
        let whole = include_str!("embedding.rs");
        let production = whole
            .split("\n#[cfg(test)]\nmod tests")
            .next()
            .expect("production text");
        let variable = concat!("ORT_DYLIB", "_PATH");
        assert!(!production.contains(variable));
        assert!(production.contains("init_from("));
    }
}
