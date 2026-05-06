use crate::full_source_code_embedding::ContentHash;

#[derive(Debug)]
pub struct RerankFragmentInput {
    pub content: String,
    pub content_hash: ContentHash,
    pub location: FragmentLocationInput,
}

#[derive(Debug)]
pub struct RerankFragment {
    pub content: String,
    pub content_hash: ContentHash,
    pub location: FragmentLocation,
}

#[derive(Debug)]
pub struct FragmentLocation {
    pub byte_end: i32,
    pub byte_start: i32,
    pub file_path: String,
}

#[derive(Debug)]
pub struct FragmentLocationInput {
    pub byte_end: i32,
    pub byte_start: i32,
    pub file_path: String,
}
