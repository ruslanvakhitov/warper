use serde_with::SerializeDisplay;
use std::fmt::Display;

/// Entry points for opening the code review pane.
#[derive(Clone, Copy, Debug, SerializeDisplay, Default)]
pub enum CodeReviewPaneEntrypoint {
    /// Opened via the git diff chip.
    GitDiffChip,
    // Opened via the pane header
    PaneHeader,
    // Opened via the code mode v2 right panel button
    RightPanel,
    /// Opened via other means (unknown entry point).
    #[default]
    Other,
}

impl Display for CodeReviewPaneEntrypoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitDiffChip => write!(f, "git_diff_chip"),
            Self::PaneHeader => write!(f, "pane_header"),
            Self::RightPanel => write!(f, "right_panel"),
            Self::Other => write!(f, "other"),
        }
    }
}

/// Where code review content was sent after the user action.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub enum CodeReviewContextDestination {
    /// Written directly to the terminal PTY for an active CLI agent.
    #[serde(rename = "pty")]
    Pty,
    /// Inserted into the active command buffer while a command is running.
    #[serde(rename = "active_command_buffer")]
    ActiveCommandBuffer,
    /// Inserted into CLI agent rich input.
    #[serde(rename = "rich_input")]
    RichInput,
}
