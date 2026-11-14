use crate::code_review::diff_state::{DiffLineType, FileDiff};
use std::collections::HashMap;
use warp_editor::render::model::LineCount;

/// Converts file diffs into a HashMap of file paths to DiffSetHunks
/// If repo_path is provided, file paths will be relative to the repo root
pub fn convert_file_diffs_to_diffset_hunks<'a, I>(files: I) -> HashMap<String, Vec<DiffSetHunk>>
where
    I: Iterator<Item = &'a FileDiff>,
{
    let mut file_diffs: HashMap<String, Vec<DiffSetHunk>> = HashMap::new();

    for file_diff in files {
        let file_path = file_diff.file_path.display().to_string();

        let mut file_hunks = Vec::new();
        for hunk in file_diff.hunks.iter() {
            // Format the diff content for this hunk
            let mut diff_lines = Vec::new();
            let mut lines_added = 0;
            let mut lines_removed = 0;
            for line in &hunk.lines {
                let prefix = match line.line_type {
                    DiffLineType::Add => {
                        lines_added += 1;
                        "+"
                    }
                    DiffLineType::Delete => {
                        lines_removed += 1;
                        "-"
                    }
                    DiffLineType::Context => "",
                    DiffLineType::HunkHeader => continue,
                };
                diff_lines.push(format!("{}{}", prefix, line.text));
            }
            let diff_content = diff_lines.join("\n");

            // Create line range using LineCount: Note that git lines are 1-based and LineCount is 0-based
            let line_range = LineCount::from(hunk.new_start_line.saturating_sub(1))
                ..LineCount::from(hunk.new_start_line.saturating_sub(1) + hunk.new_line_count);

            file_hunks.push(DiffSetHunk {
                line_range,
                diff_content,
                lines_added,
                lines_removed,
            });
        }

        if !file_hunks.is_empty() {
            file_diffs.insert(file_path, file_hunks);
        }
    }

    file_diffs
}

/// Git reference displayed by manual code review.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CurrentHead {
    BranchName(String),
    HeadlessCommitSha(String),
}

impl CurrentHead {
    pub fn title(&self) -> String {
        match self {
            Self::BranchName(name) => name.clone(),
            Self::HeadlessCommitSha(sha) => {
                format!("Commit {}", sha.chars().take(7).collect::<String>())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DiffBase {
    BranchName(String),
    HeadlessCommitSha(String),
    UncommittedChanges,
}

/// Diff context supplied to a third-party CLI agent.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiffSetHunk {
    pub line_range: std::ops::Range<LineCount>,
    pub diff_content: String,
    pub lines_added: u32,
    pub lines_removed: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReviewCommentPacket {
    pub comments: Vec<super::comments::AttachedReviewComment>,
    pub diff_set: HashMap<String, Vec<DiffSetHunk>>,
}

impl ReviewCommentPacket {
    pub fn review_comments(&self) -> super::comments::ReviewCommentBatch {
        super::comments::ReviewCommentBatch::from_comments(self.comments.clone())
    }
}
