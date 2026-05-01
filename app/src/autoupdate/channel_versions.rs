use std::{env, fs::read_to_string, sync::Arc};

use anyhow::{Context as _, Result};
use channel_versions::ChannelVersions;

use crate::server::server_api::ServerApi;

// Fetches channel versions from a local test fixture only. Hosted version endpoints are disabled
// in Warper.
pub async fn fetch_channel_versions(
    _nonce: &str,
    server_api: Arc<ServerApi>,
    _include_changelogs: bool,
    _is_daily: bool,
) -> Result<ChannelVersions> {
    let _ = server_api;
    if let Ok(path) = env::var("WARP_CHANNEL_VERSIONS_PATH") {
        // Load channel versions from local filesystem. Used for testing both
        // autoupdate and changelog behavior.
        let path = shellexpand::tilde(&path);
        let channel_versions_string = read_to_string::<&str>(&path)?;
        return serde_json::from_str(channel_versions_string.as_str())
            .context("Failed to parse channel versions JSON");
    }

    anyhow::bail!("Hosted channel version fetch is disabled")
}
