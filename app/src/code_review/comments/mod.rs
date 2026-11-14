mod batch;
mod comment;

pub(crate) use batch::{ReviewCommentBatch, ReviewCommentBatchEvent};
pub(crate) use comment::{
    AttachedReviewComment, AttachedReviewCommentTarget, CommentId, LineDiffContent,
};
