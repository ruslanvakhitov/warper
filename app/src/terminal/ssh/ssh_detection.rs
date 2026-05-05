use serde::{Deserialize, Serialize};
use warp_util::path::ShellFamily;

use crate::terminal::warpify::settings::WarpifySettings;

/// The different possible outcomes of detecting an interactive SSH session.
/// Also the payload for the [`crate::server::telemetry::TelemetryEvent::SshInteractiveSessionDetected`] event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SshInteractiveSessionDetected {
    #[serde(rename = "feature_disabled")]
    FeatureDisabled,
    #[serde(rename = "host_denylisted")]
    HostDenylisted,
    #[serde(rename = "warpify_prompt")]
    ShouldPromptWarpification {
        #[serde(skip)]
        command: String,
        #[serde(skip)]
        host: Option<String>,
    },
}

/// Determines whether a host could be warpified.
pub fn evaluate_warpify_ssh_host(
    command: &str,
    ssh_host: Option<&str>,
    shell_family: ShellFamily,
    warpify_settings: &WarpifySettings,
) -> SshInteractiveSessionDetected {
    let _ = (command, ssh_host, shell_family, warpify_settings);
    SshInteractiveSessionDetected::FeatureDisabled
}
