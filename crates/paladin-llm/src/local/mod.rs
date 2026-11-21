//! Local (offline) adapters: ports served from files on the operator's own disk, with no network.
//!
//! Everything here sits behind an off-by-default feature (`local-embeddings`) and downloads
//! nothing: the bytes an adapter loads are given to it as paths and are checked against pinned
//! sha256 digests before use.

pub mod embedding;
