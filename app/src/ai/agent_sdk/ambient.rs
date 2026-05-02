//! Hosted ambient-agent CLI support is removed from Warper.
//!
//! This module is intentionally kept as a small fail-closed boundary so stale
//! task/message/conversation dispatch cannot reach Warp/Oz hosted APIs.

/// Error returned for removed hosted ambient-agent commands.
pub(super) fn hosted_ambient_removed_error() -> anyhow::Error {
    anyhow::anyhow!(
        "hosted ambient agent task, message, and conversation commands are unavailable in this build"
    )
}
