use crate::server::ids::ServerId;

use super::workspace::WorkspaceSettings;

#[derive(Clone, Debug)]
pub struct Team {
    pub uid: ServerId,
    pub name: String,
    pub organization_settings: WorkspaceSettings,
}

impl Team {
    pub fn from_local_cache(
        uid: ServerId,
        name: String,
        workspace_settings: Option<WorkspaceSettings>,
    ) -> Self {
        Self {
            uid,
            name,
            organization_settings: workspace_settings.unwrap_or_default(),
        }
    }

    pub fn is_custom_llm_enabled(&self) -> bool {
        self.organization_settings.llm_settings.enabled
    }
}
