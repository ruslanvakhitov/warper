use crate::{
    code::editor::{line::EditorLineLocation, EditorReviewComment},
    code_review::context::{CurrentHead, DiffBase},
};
use chrono::{DateTime, Local};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use warp_editor::render::model::LineCount;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct LineDiffContent {
    pub content: String,
    pub lines_added: LineCount,
    pub lines_removed: LineCount,
}

impl LineDiffContent {
    /// The text in the diff line, without the `+` or `-` diff prefix or trailing newlines.
    ///
    /// Uses `strip_prefix` (removes exactly one occurrence) rather than `trim_start_matches`
    /// (which removes all leading occurrences) so that content characters are preserved.
    pub(crate) fn original_text(&self) -> String {
        let s = self.content.trim_end_matches('\n');
        s.strip_prefix('+')
            .or_else(|| s.strip_prefix('-'))
            .unwrap_or(s)
            .to_string()
    }

    pub(crate) fn from_content(diff_line: &str) -> Self {
        let lines_added = LineCount::from(if diff_line.starts_with('+') { 1 } else { 0 });
        let lines_removed = LineCount::from(if diff_line.starts_with('-') { 1 } else { 0 });
        Self {
            content: diff_line.to_owned(),
            lines_added,
            lines_removed,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommentId(uuid::Uuid);

impl CommentId {
    pub(crate) fn new() -> Self {
        CommentId(uuid::Uuid::new_v4())
    }

    pub(crate) fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }
}

impl Display for CommentId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for CommentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Code review comment attached to a local file editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachedReviewComment {
    /// Locally-generated ID.
    pub id: CommentId,
    pub content: String,
    pub target: AttachedReviewCommentTarget,
    pub last_update_time: DateTime<Local>,
    pub base: Option<DiffBase>,
    pub head: Option<CurrentHead>,
    pub outdated: bool,
}

/// Target for an attached review comment. File paths are always absolute when present.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttachedReviewCommentTarget {
    Line {
        absolute_file_path: PathBuf,
        line: EditorLineLocation,
        content: LineDiffContent,
    },
    File {
        absolute_file_path: PathBuf,
    },
    General,
}

impl AttachedReviewCommentTarget {
    pub(crate) fn absolute_file_path(&self) -> Option<&PathBuf> {
        match self {
            AttachedReviewCommentTarget::Line {
                absolute_file_path, ..
            } => Some(absolute_file_path),
            AttachedReviewCommentTarget::File { absolute_file_path } => Some(absolute_file_path),
            AttachedReviewCommentTarget::General => None,
        }
    }

    pub(crate) fn line_number(&self) -> Option<LineCount> {
        match self {
            AttachedReviewCommentTarget::Line { line, .. } => line.line_number(),
            _ => None,
        }
    }
}

impl AttachedReviewComment {
    pub(crate) fn from_editor_review_comment(
        comment: EditorReviewComment,
        absolute_file_path: PathBuf,
        base: Option<DiffBase>,
        head: Option<CurrentHead>,
    ) -> AttachedReviewComment {
        AttachedReviewComment {
            id: comment.id,
            content: comment.comment_content,
            base,
            head,
            target: AttachedReviewCommentTarget::Line {
                absolute_file_path,
                line: comment.line,
                content: comment.diff_content,
            },
            last_update_time: comment.last_update_time,
            outdated: false,
        }
    }

    pub fn head(&self) -> Option<&CurrentHead> {
        self.head.as_ref()
    }
}

#[cfg(test)]
#[path = "comment_tests.rs"]
mod tests;
