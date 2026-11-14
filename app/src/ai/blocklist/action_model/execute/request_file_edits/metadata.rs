use serde::Serialize;

/// Coarse format classification for the edit payload that produced a code diff.
#[derive(Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum RequestFileEditsFormatKind {
    /// Legacy search/replace diff format (`edit_files` style).
    StrReplace,
    /// Structured V4A patch format (`apply_patch` style with Begin/End Patch hunks).
    V4A,
    /// Both formats were present in the same requested edit payload.
    Mixed,
    /// The format could not be determined from the payload.
    Unknown,
}
