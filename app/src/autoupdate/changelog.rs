use std::sync::Arc;

use anyhow::Result;
use channel_versions::Changelog;

use crate::{channel::Channel, server::server_api::ServerApi};

pub async fn get_current_changelog(server_api: Arc<ServerApi>) -> Result<Option<Changelog>> {
    let _ = server_api;
    log::debug!("Hosted changelog fetch is disabled.");
    Ok(None)
}

/// Returns whether the app should fetch changelog JSON for the current
/// build (true), or use the embedded release metadata (false).
pub fn should_fetch_changelog_json(channel: Channel) -> bool {
    channel == Channel::Dev
}
