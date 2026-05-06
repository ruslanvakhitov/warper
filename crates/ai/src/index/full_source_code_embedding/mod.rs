mod changed_files;
mod chunker;
mod codebase_index;
mod fragment_metadata;
pub mod manager;
mod merkle_tree;
mod priority_queue;
mod snapshot;
pub mod store_client;
mod sync_client;

use std::{ops::Range, path::PathBuf, time::Duration};
pub use sync_client::SyncTask;

pub use codebase_index::{CodebaseIndex, RetrievalID, SyncProgress};
pub use merkle_tree::{ContentHash, NodeHash};

use fragment_metadata::FragmentMetadata;
use string_offset::ByteOffset;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("File I/O error {0:#}")]
    Io(#[from] std::io::Error),
    #[error("Not a git repository")]
    NotAGitRepository,
    #[error("Build tree error {0:#}")]
    BuildTreeError(#[from] crate::index::BuildTreeError),
    #[error("Unsupported platform")]
    UnsupportedPlatform,
    #[error("Invalid hash: {0:#}")]
    InvalidHash(base16ct::Error),
    #[error("Empty node content")]
    EmptyNodeContent,
    #[error("Failed to get metadata")]
    FailedToGetMetadata(PathBuf),
    #[error("File size exceeds maximum limit")]
    FileSizeExceeded,
    #[error(transparent)]
    InconsistentState(#[from] InconsistentStateError),
    #[error("Failed to generate embeddings for some hashes")]
    FailedToGenerateEmbeddings(Vec<FragmentMetadata>),
    #[error("Failed to sync some intermediate nodes")]
    FailedToSyncIntermediateNodes(Vec<NodeHash>),
    #[error("Diff merkle tree {0:#}")]
    DiffMerkleTreeError(#[from] crate::index::full_source_code_embedding::DiffMerkleTreeError),
    #[error("File system changed since merkle tree construction")]
    FileSystemStateChanged,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    #[error("Failed to parse snapshot")]
    SnapshotParsingFailed,
}

// Based off of BuildTreeError in entry.rs
#[derive(Debug, Error)]
pub enum DiffMerkleTreeError {
    #[error("Merkle tree node and file mismatch")]
    CurrentNodeMismatch(PathBuf),
    #[error("File is ignored")]
    Ignored,
    #[error("Symlink is not supported")]
    Symlink,
    #[error("Fragment node in diffing process")]
    Fragment(PathBuf),
    #[error("Max depth exceeded")]
    MaxDepthExceeded,
    #[error("Exceeded max file limit")]
    ExceededMaxFileLimit,
}

#[derive(Error, Debug)]
pub enum InconsistentStateError {
    #[error("Missing fragment metadata for {fragment_hash}")]
    MissingFragmentMetadata { fragment_hash: ContentHash },
    #[error("Can't find node index in merkle node")]
    NodeIndexNotFound,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingConfig {
    OpenAiTextSmall3_256,
    VoyageCode3_512,
    Voyage3_5_Lite_512,
    #[default]
    Voyage3_5_512,
}

#[derive(Debug, Clone)]
pub struct RepoMetadata {
    pub path: Option<String>,
}

#[derive(Clone, Copy)]
pub struct CodebaseContextConfig {
    pub embedding_config: EmbeddingConfig,
    pub embedding_cadence: Duration,
}

#[derive(Clone)]
pub struct FragmentLocation {
    absolute_path: PathBuf,
    byte_range: Range<ByteOffset>,
}

#[derive(Clone)]
pub struct Fragment {
    content: String,
    content_hash: ContentHash,
    location: FragmentLocation,
}
