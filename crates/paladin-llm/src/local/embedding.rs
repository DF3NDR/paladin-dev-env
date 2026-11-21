//! Offline local embedder: [`LocalEmbeddingAdapter`] serves [`EmbeddingPort`] from model files and
//! an ONNX Runtime shared library that already sit on the operator's disk.
//!
//! # What it does, and what it refuses
//!
//! - **Downloads nothing.** `fastembed` is built with its default features off, so the model hub
//!   client is not in the build; only `TextEmbedding::try_new_from_user_defined` is used, which
//!   takes bytes the adapter already holds. A separate, explicit setup step (outside this crate)
//!   places the files; a missing file is refused with a message that says so.
//! - **Checks every file before use.** Each of the model files is read once, its size and sha256
//!   are compared with the pinned [`LocalModelSpec`], and the very bytes that were hashed are the
//!   bytes handed to `fastembed`. A missing, short, altered or symlinked file is refused, naming
//!   the file and (for an altered one) the pinned and the actual digest. The runtime library is
//!   hashed the same way (streaming) and loaded with `ort::init_from(<verified path>)`; the
//!   loader-path environment variable of the `ort` crate is never consulted.
//! - **Refuses to run with telemetry on.** The official ONNX Runtime build carries a telemetry
//!   uploader that is on by default; only the `ORT_DISABLE_TELEMETRY=1` environment variable,
//!   set before the library is initialised, stops it (the API switch alone does not). This crate
//!   forbids `unsafe`, and setting an environment variable is `unsafe`, so the adapter cannot set
//!   it: it refuses to construct unless the host already did.
//! - **Names its model with the revision.** [`LocalModelSpec::model_name`] is
//!   `local:<model id>@<40-hex revision>`, so a snapshot built by one embedder is never reused by
//!   another, and a pin change is a new identity.
//!
//! The library is loaded once per process: the first verified path wins, and a later adapter that
//! names a different path is refused rather than silently served by the first library.
//!
//! Inference runs on `spawn_blocking` behind a `Mutex`; no guard is held across an `.await`.

use async_trait::async_trait;
use paladin_ports::output::embedding_port::{Embedding, EmbeddingError, EmbeddingPort};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Name of the environment variable that must be `1` before the adapter will load the library.
const TELEMETRY_VARIABLE: &str = "ORT_DISABLE_TELEMETRY";

/// Texts per inference call (the length-sorted sub-batch size).
const SUB_BATCH: usize = 8;

/// The one runtime library path this process has loaded (first verified path wins).
static LOADED_RUNTIME: OnceLock<PathBuf> = OnceLock::new();

/// One pinned model file: its path under the model directory, exact size and sha256 (lowercase hex).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelFileSpec {
    /// Path relative to the model directory, for example `onnx/model.onnx`.
    pub path: &'static str,
    /// Exact size in bytes.
    pub size: u64,
    /// Lowercase hex sha256 of the file.
    pub sha256: &'static str,
}

/// The pinned ONNX Runtime shared library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeLibrarySpec {
    /// ONNX Runtime release, for example `1.31.0`.
    pub version: &'static str,
    /// File name of the shared library as shipped, for example `libonnxruntime.so.1.31.0`.
    pub file: &'static str,
    /// Exact size in bytes.
    pub size: u64,
    /// Lowercase hex sha256 of the file.
    pub sha256: &'static str,
}

/// How token vectors are pooled into one text vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LocalPooling {
    /// The first (classification) token's vector.
    Cls,
}

/// Everything that identifies one local model: its files, geometry and runtime library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalModelSpec {
    /// Model identifier, for example `BAAI/bge-small-en-v1.5`.
    pub model_id: &'static str,
    /// The 40-hex commit the files are pinned to.
    pub revision: &'static str,
    /// The pinned files: the ONNX graph and the four tokenizer files.
    pub files: &'static [ModelFileSpec],
    /// Vector dimension.
    pub dimension: usize,
    /// Token window (longer inputs are truncated by the tokenizer).
    pub max_length: usize,
    /// Pooling used to build one vector per text.
    pub pooling: LocalPooling,
    /// Whether vectors are L2-normalised.
    pub normalize: bool,
    /// Prefix the model expects on a query (not applied by the adapter), if any.
    pub query_prefix: Option<&'static str>,
    /// The pinned ONNX Runtime shared library.
    pub runtime: RuntimeLibrarySpec,
}

impl LocalModelSpec {
    /// The identity recorded on every [`Embedding`]: `local:<model id>@<revision>`.
    #[must_use]
    pub fn model_name(&self) -> String {
        format!("local:{}@{}", self.model_id, self.revision)
    }
}

/// `BAAI/bge-small-en-v1.5` (MIT) pinned to one commit, with ONNX Runtime 1.31.0.
///
/// 384 dimensions, 512-token window, CLS pooling, L2-normalised, no query prefix.
pub const LOCAL_BGE_SMALL_SPEC: LocalModelSpec = LocalModelSpec {
    model_id: "BAAI/bge-small-en-v1.5",
    revision: "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a",
    files: &[
        ModelFileSpec {
            path: "onnx/model.onnx",
            size: 133_093_490,
            sha256: "828e1496d7fabb79cfa4dcd84fa38625c0d3d21da474a00f08db0f559940cf35",
        },
        ModelFileSpec {
            path: "tokenizer.json",
            size: 711_396,
            sha256: "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66",
        },
        ModelFileSpec {
            path: "config.json",
            size: 743,
            sha256: "094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750",
        },
        ModelFileSpec {
            path: "special_tokens_map.json",
            size: 125,
            sha256: "b6d346be366a7d1d48332dbc9fdf3bf8960b5d879522b7799ddba59e76237ee3",
        },
        ModelFileSpec {
            path: "tokenizer_config.json",
            size: 366,
            sha256: "9261e7d79b44c8195c1cada2b453e55b00aeb81e907a6664974b4d7776172ab3",
        },
    ],
    dimension: 384,
    max_length: 512,
    pooling: LocalPooling::Cls,
    normalize: true,
    query_prefix: None,
    runtime: RuntimeLibrarySpec {
        version: "1.31.0",
        file: "libonnxruntime.so.1.31.0",
        size: 29_352_200,
        sha256: "21901b1b02233484b80888c16f4f5473742f796bf5528d5d0c4073dbe9802001",
    },
};

/// Where the adapter finds its files.
#[derive(Debug, Clone)]
pub struct LocalEmbeddingConfig {
    /// Directory holding the model files at the paths named by the spec.
    pub model_dir: PathBuf,
    /// Absolute path of the ONNX Runtime shared library.
    pub runtime_path: PathBuf,
    /// ONNX Runtime intra-op threads; `None` is `min(4, available cores)`.
    pub intra_threads: Option<usize>,
}

/// Why the local embedder refused to load or to run.
///
/// The port reports every one of these as [`EmbeddingError::ProviderError`] with this text.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LocalEmbeddingError {
    /// `ORT_DISABLE_TELEMETRY` is not `1` in the environment.
    #[error(
        "{TELEMETRY_VARIABLE} must be 1 before the local embedder loads ONNX Runtime (found {found}); \
         the official runtime reports to its vendor unless this variable is set before it starts"
    )]
    TelemetryNotDisabled {
        /// What the lookup returned, or `unset`.
        found: String,
    },
    /// A pinned file is not on disk.
    #[error(
        "local embedder file {path} is missing; run the explicit local-embedder setup step to place \
         it (nothing is downloaded at run time)"
    )]
    FileMissing {
        /// The file that is missing.
        path: String,
    },
    /// A pinned file is a symlink or not a regular file.
    #[error("local embedder file {path} is a symlink or not a regular file; refusing to load it")]
    FileNotRegular {
        /// The refused file.
        path: String,
    },
    /// A pinned file has the wrong size.
    #[error(
        "local embedder file {path} has size {actual} bytes, pinned size {pinned}; refusing to load it"
    )]
    FileSize {
        /// The refused file.
        path: String,
        /// Pinned size.
        pinned: u64,
        /// Size found.
        actual: u64,
    },
    /// A pinned file has the wrong sha256.
    #[error(
        "local embedder file {path} has sha256 {actual}, pinned sha256 {pinned}; refusing to load it"
    )]
    FileDigest {
        /// The refused file.
        path: String,
        /// Pinned digest.
        pinned: String,
        /// Digest found.
        actual: String,
    },
    /// A file could not be read.
    #[error("cannot read local embedder file {path}: {reason}")]
    FileUnreadable {
        /// The file.
        path: String,
        /// The operating-system reason.
        reason: String,
    },
    /// The runtime library path is not usable.
    #[error("ONNX Runtime library path {path}: {reason}")]
    RuntimePath {
        /// The path as given.
        path: String,
        /// Why it was refused.
        reason: String,
    },
    /// ONNX Runtime refused to load from the verified path.
    #[error("ONNX Runtime refused to load from {path}: {reason}")]
    RuntimeLoad {
        /// The verified path.
        path: String,
        /// The loader's reason.
        reason: String,
    },
    /// The model could not be built from the verified bytes.
    #[error("local embedding model failed to load: {0}")]
    ModelLoad(String),
    /// Inference failed or returned a malformed result.
    #[error("local embedding inference failed: {0}")]
    Inference(String),
    /// The worker thread failed or its lock was poisoned.
    #[error("local embedding worker failed: {0}")]
    Worker(String),
}

impl From<LocalEmbeddingError> for EmbeddingError {
    fn from(error: LocalEmbeddingError) -> Self {
        EmbeddingError::ProviderError(error.to_string())
    }
}

/// Lowercase hex sha256 of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Refuse unless the host already set `ORT_DISABLE_TELEMETRY=1`.
fn check_telemetry_disabled(
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<(), LocalEmbeddingError> {
    match lookup(TELEMETRY_VARIABLE) {
        Some(value) if value == "1" => Ok(()),
        Some(value) => Err(LocalEmbeddingError::TelemetryNotDisabled {
            found: format!("{value:?}"),
        }),
        None => Err(LocalEmbeddingError::TelemetryNotDisabled {
            found: "unset".to_string(),
        }),
    }
}

/// `symlink_metadata` of `path`, refusing a missing file, a symlink and anything not a regular file.
fn regular_file_size(path: &Path, shown: &str) -> Result<u64, LocalEmbeddingError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            LocalEmbeddingError::FileMissing {
                path: shown.to_string(),
            }
        } else {
            LocalEmbeddingError::FileUnreadable {
                path: shown.to_string(),
                reason: error.to_string(),
            }
        }
    })?;
    if !metadata.file_type().is_file() {
        return Err(LocalEmbeddingError::FileNotRegular {
            path: shown.to_string(),
        });
    }
    Ok(metadata.len())
}

fn unreadable(shown: &str, error: &std::io::Error) -> LocalEmbeddingError {
    LocalEmbeddingError::FileUnreadable {
        path: shown.to_string(),
        reason: error.to_string(),
    }
}

/// Read one pinned file once and check its size and sha256 against the pin.
fn read_verified(dir: &Path, pin: &ModelFileSpec) -> Result<Vec<u8>, LocalEmbeddingError> {
    let path = dir.join(pin.path);
    let shown = pin.path;
    let on_disk = regular_file_size(&path, shown)?;
    if on_disk != pin.size {
        return Err(LocalEmbeddingError::FileSize {
            path: shown.to_string(),
            pinned: pin.size,
            actual: on_disk,
        });
    }
    let file = std::fs::File::open(&path).map_err(|e| unreadable(shown, &e))?;
    // Bound the read at one byte past the pin so a file that grew after the metadata check cannot
    // exhaust memory.
    let mut bytes = Vec::with_capacity(usize::try_from(pin.size).unwrap_or(0));
    file.take(pin.size.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| unreadable(shown, &e))?;
    let actual_size = bytes.len() as u64;
    if actual_size != pin.size {
        return Err(LocalEmbeddingError::FileSize {
            path: shown.to_string(),
            pinned: pin.size,
            actual: actual_size,
        });
    }
    let actual = sha256_hex(&bytes);
    if actual != pin.sha256 {
        return Err(LocalEmbeddingError::FileDigest {
            path: shown.to_string(),
            pinned: pin.sha256.to_string(),
            actual,
        });
    }
    Ok(bytes)
}

/// Hash the runtime library (streaming) and check size and sha256 against the pin.
fn verify_runtime_library(
    path: &Path,
    pin: &RuntimeLibrarySpec,
) -> Result<(), LocalEmbeddingError> {
    let shown = pin.file;
    if !path.is_absolute() {
        return Err(LocalEmbeddingError::RuntimePath {
            path: path.display().to_string(),
            reason: "must be an absolute path".to_string(),
        });
    }
    let on_disk = regular_file_size(path, shown)?;
    if on_disk != pin.size {
        return Err(LocalEmbeddingError::FileSize {
            path: shown.to_string(),
            pinned: pin.size,
            actual: on_disk,
        });
    }
    let mut file = std::fs::File::open(path).map_err(|e| unreadable(shown, &e))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1 << 20];
    let mut total: u64 = 0;
    loop {
        let read = file.read(&mut buffer).map_err(|e| unreadable(shown, &e))?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        if let Some(chunk) = buffer.get(..read) {
            hasher.update(chunk);
        }
    }
    if total != pin.size {
        return Err(LocalEmbeddingError::FileSize {
            path: shown.to_string(),
            pinned: pin.size,
            actual: total,
        });
    }
    let actual = hex(&hasher.finalize());
    if actual != pin.sha256 {
        return Err(LocalEmbeddingError::FileDigest {
            path: shown.to_string(),
            pinned: pin.sha256.to_string(),
            actual,
        });
    }
    Ok(())
}

/// The inference seam: texts in, one raw vector per text out.
trait EmbedBackend {
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String>;
}

/// The real backend: a `fastembed` text model built from verified bytes.
struct FastembedBackend {
    model: fastembed::TextEmbedding,
}

impl EmbedBackend for FastembedBackend {
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        self.model
            .embed(texts, Some(SUB_BATCH))
            .map_err(|error| error.to_string())
    }
}

fn take_file(
    loaded: &mut Vec<(&'static str, Vec<u8>)>,
    path: &str,
) -> Result<Vec<u8>, LocalEmbeddingError> {
    let position = loaded.iter().position(|(name, _)| *name == path);
    match position {
        Some(index) => Ok(loaded.swap_remove(index).1),
        None => Err(LocalEmbeddingError::ModelLoad(format!(
            "the model spec does not pin {path}"
        ))),
    }
}

/// Verify every file, load the runtime, and build the model. Runs on a blocking thread.
fn load_backend(
    config: &LocalEmbeddingConfig,
    spec: &'static LocalModelSpec,
) -> Result<Box<dyn EmbedBackend + Send>, LocalEmbeddingError> {
    let mut loaded: Vec<(&'static str, Vec<u8>)> = Vec::new();
    for pin in spec.files {
        loaded.push((pin.path, read_verified(&config.model_dir, pin)?));
    }
    verify_runtime_library(&config.runtime_path, &spec.runtime)?;

    match LOADED_RUNTIME.get() {
        Some(already) if already != &config.runtime_path => {
            return Err(LocalEmbeddingError::RuntimePath {
                path: config.runtime_path.display().to_string(),
                reason: format!(
                    "ONNX Runtime was already loaded from {} in this process",
                    already.display()
                ),
            });
        }
        _ => {}
    }
    let builder =
        ort::init_from(&config.runtime_path).map_err(|error| LocalEmbeddingError::RuntimeLoad {
            path: config.runtime_path.display().to_string(),
            reason: error.to_string(),
        })?;
    // The first committed environment is the process-wide one; a repeat commit is a no-op.
    let _first_commit = builder.with_telemetry(false).commit();
    let _ = LOADED_RUNTIME.set(config.runtime_path.clone());

    let files = fastembed::TokenizerFiles {
        tokenizer_file: take_file(&mut loaded, "tokenizer.json")?,
        config_file: take_file(&mut loaded, "config.json")?,
        special_tokens_map_file: take_file(&mut loaded, "special_tokens_map.json")?,
        tokenizer_config_file: take_file(&mut loaded, "tokenizer_config.json")?,
    };
    let onnx = take_file(&mut loaded, "onnx/model.onnx")?;
    let pooling = match spec.pooling {
        LocalPooling::Cls => fastembed::Pooling::Cls,
    };
    let model = fastembed::UserDefinedEmbeddingModel::new(onnx, files).with_pooling(pooling);
    let threads = config.intra_threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1)
            .min(4)
    });
    let options = fastembed::InitOptionsUserDefined::new()
        .with_max_length(spec.max_length)
        .with_intra_threads(threads.max(1));
    let text_model = fastembed::TextEmbedding::try_new_from_user_defined(model, options)
        .map_err(|error| LocalEmbeddingError::ModelLoad(error.to_string()))?;
    // `loaded` (any bytes not consumed) and the moved buffers are dropped here.
    Ok(Box::new(FastembedBackend { model: text_model }))
}

/// Embed `texts` in length order, in sub-batches of [`SUB_BATCH`], and return the vectors in the
/// caller's order, each checked for size and (when pinned) L2-normalised.
fn run_sorted(
    backend: &Mutex<Box<dyn EmbedBackend + Send>>,
    texts: &[String],
    spec: &LocalModelSpec,
) -> Result<Vec<Vec<f32>>, LocalEmbeddingError> {
    let mut order: Vec<usize> = (0..texts.len()).collect();
    order.sort_by_key(|&index| texts.get(index).map_or(0, String::len));
    let mut slots: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
    let mut guard = backend
        .lock()
        .map_err(|error| LocalEmbeddingError::Worker(error.to_string()))?;
    for chunk in order.chunks(SUB_BATCH) {
        let refs: Vec<&str> = chunk
            .iter()
            .filter_map(|&index| texts.get(index).map(String::as_str))
            .collect();
        let vectors = guard.embed(&refs).map_err(LocalEmbeddingError::Inference)?;
        if vectors.len() != chunk.len() {
            return Err(LocalEmbeddingError::Inference(format!(
                "backend returned {} vectors for {} texts",
                vectors.len(),
                chunk.len()
            )));
        }
        for (&index, mut vector) in chunk.iter().zip(vectors) {
            if vector.len() != spec.dimension {
                return Err(LocalEmbeddingError::Inference(format!(
                    "backend returned a {}-dimension vector, pinned dimension {}",
                    vector.len(),
                    spec.dimension
                )));
            }
            if spec.normalize {
                normalise(&mut vector)?;
            }
            if let Some(slot) = slots.get_mut(index) {
                *slot = Some(vector);
            }
        }
    }
    slots
        .into_iter()
        .map(|slot| {
            slot.ok_or_else(|| LocalEmbeddingError::Inference("a text received no vector".into()))
        })
        .collect()
}

fn normalise(vector: &mut [f32]) -> Result<(), LocalEmbeddingError> {
    let norm = vector
        .iter()
        .map(|&x| f64::from(x) * f64::from(x))
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm < 1e-12 {
        return Err(LocalEmbeddingError::Inference(
            "backend returned a zero or non-finite vector".to_string(),
        ));
    }
    for x in vector.iter_mut() {
        *x = (f64::from(*x) / norm) as f32;
    }
    Ok(())
}

/// Offline [`EmbeddingPort`] adapter over `BAAI/bge-small-en-v1.5` (or another pinned
/// [`LocalModelSpec`]), served by ONNX Runtime from files already on disk.
///
/// Construct it with [`LocalEmbeddingAdapter::try_new`]; see the module documentation for what it
/// verifies and refuses.
pub struct LocalEmbeddingAdapter {
    spec: &'static LocalModelSpec,
    identity: String,
    backend: Arc<Mutex<Box<dyn EmbedBackend + Send>>>,
}

impl std::fmt::Debug for LocalEmbeddingAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalEmbeddingAdapter")
            .field("identity", &self.identity)
            .finish_non_exhaustive()
    }
}

impl LocalEmbeddingAdapter {
    /// Load the pinned [`LOCAL_BGE_SMALL_SPEC`] model from `config`, reading the process
    /// environment for `ORT_DISABLE_TELEMETRY`.
    ///
    /// # Errors
    ///
    /// [`EmbeddingError::ProviderError`] if telemetry is not disabled, a file is missing, short,
    /// altered or a symlink, the runtime library cannot be loaded, or the model cannot be built.
    pub async fn try_new(config: LocalEmbeddingConfig) -> Result<Self, EmbeddingError> {
        Self::try_new_with_env(config, &LOCAL_BGE_SMALL_SPEC, |name| {
            std::env::var(name).ok()
        })
        .await
    }

    /// Like [`try_new`](Self::try_new) with an explicit spec and environment lookup.
    ///
    /// The telemetry variable is checked first, then every model file, then the runtime library.
    ///
    /// # Errors
    ///
    /// As [`try_new`](Self::try_new).
    pub async fn try_new_with_env(
        config: LocalEmbeddingConfig,
        spec: &'static LocalModelSpec,
        lookup: impl Fn(&str) -> Option<String> + Send,
    ) -> Result<Self, EmbeddingError> {
        check_telemetry_disabled(&lookup)?;
        let backend = tokio::task::spawn_blocking(move || load_backend(&config, spec))
            .await
            .map_err(|error| LocalEmbeddingError::Worker(error.to_string()))??;
        Ok(Self {
            spec,
            identity: spec.model_name(),
            backend: Arc::new(Mutex::new(backend)),
        })
    }

    async fn embed_many(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        if texts.iter().any(|text| text.trim().is_empty()) {
            return Err(EmbeddingError::InvalidInput(
                "Text cannot be empty".to_string(),
            ));
        }
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let owned: Vec<String> = texts.iter().map(|text| (*text).to_string()).collect();
        let backend = Arc::clone(&self.backend);
        let spec = self.spec;
        let vectors = tokio::task::spawn_blocking(move || run_sorted(&backend, &owned, spec))
            .await
            .map_err(|error| LocalEmbeddingError::Worker(error.to_string()))??;
        Ok(vectors
            .into_iter()
            .map(|vector| Embedding {
                dimension: vector.len(),
                vector,
                model: self.identity.clone(),
                token_count: None,
            })
            .collect())
    }
}

#[async_trait]
impl EmbeddingPort for LocalEmbeddingAdapter {
    async fn embed_text(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let mut embeddings = self.embed_many(&[text]).await?;
        embeddings
            .pop()
            .ok_or_else(|| EmbeddingError::ProviderError("No embeddings returned".to_string()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        self.embed_many(texts).await
    }

    fn dimension(&self) -> usize {
        self.spec.dimension
    }

    fn model_name(&self) -> &str {
        &self.identity
    }
}

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
        fn with_backend(
            spec: &'static LocalModelSpec,
            backend: Box<dyn EmbedBackend + Send>,
        ) -> Self {
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
        let owned: Vec<String> = (0..20)
            .map(|i| "x".repeat(((i * 7) % 19) + 1 + i))
            .collect();
        let texts: Vec<&str> = owned.iter().map(String::as_str).collect();
        let out = adapter.embed_batch(&texts).await.expect("batch");
        assert_eq!(out.len(), texts.len());
        for (text, embedding) in texts.iter().zip(&out) {
            assert_eq!(
                embedding.vector[text.len() % 384].round() as i32,
                1,
                "hot slot"
            );
        }
        let recorded = calls.lock().expect("calls").clone();
        assert!(
            recorded.iter().all(|c| !c.is_empty() && c.len() <= 8),
            "{recorded:?}"
        );
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
        assert!(
            adapter
                .embed_batch(&[])
                .await
                .expect("empty batch")
                .is_empty()
        );
        assert!(
            calls.lock().expect("calls").is_empty(),
            "nothing reached the backend"
        );
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
            match LocalEmbeddingAdapter::try_new_with_env(config_for(dir.path()), spec, lookup)
                .await
            {
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
        let pinned = spec
            .files
            .iter()
            .find(|f| f.path == "config.json")
            .expect("pin")
            .sha256;
        assert!(msg.contains(pinned), "pinned digest named: {msg}");
        assert!(
            msg.contains(&sha256_hex(&bytes)),
            "actual digest named: {msg}"
        );
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
