use std::time::Duration;

pub mod error;
pub mod install_tmux;
pub mod root_access;
mod script_controls;
mod script_preview;
pub mod ssh_detection;
pub mod util;
pub mod warpify;

pub const SSH_WARPIFY_TIMEOUT_DURATION: Duration = Duration::from_secs(8);
