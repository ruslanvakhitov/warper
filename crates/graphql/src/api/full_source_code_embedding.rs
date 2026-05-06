#[derive(Debug, Clone)]
pub struct ContentHash(pub String);

#[derive(Clone, Copy, Debug)]
pub enum EmbeddingConfig {
    OpenaiTextSmall3256,
    VoyageCode3512,
    Voyage35512,
    Voyage35Lite512,
}

#[derive(Debug, Clone)]
pub struct NodeHash(pub String);

#[derive(Debug)]
pub struct Fragment {
    pub content: String,
    pub content_hash: ContentHash,
}

#[derive(Debug)]
pub struct RepoMetadata {
    pub path: Option<String>,
}
