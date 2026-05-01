use std::sync::Arc;

use anyhow::Result;
use channel_versions::Changelog;

use crate::{channel::Channel, server::server_api::ServerApi};

pub async fn get_current_changelog(server_api: Arc<ServerApi>) -> Result<Option<Changelog>> {
    let _ = server_api;
    log::debug!("Hosted changelog fetch is disabled.");
    Ok(None)
}

/// Returns whether the app should fetch changelog.json for the current
/// build (true), or use the changelog information embedded in
/// channel_versions.json (false).
pub fn should_fetch_changelog_json(channel: Channel) -> bool {
    channel == Channel::Dev
}
