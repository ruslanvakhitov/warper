use crate::{
    cloud_object::{
        model::{
            generic_string_model::{GenericStringModel, GenericStringObjectId, StringModel},
            json_model::{JsonModel, JsonSerializer},
        },
        GenericCloudObject, GenericStringObjectFormat, GenericStringObjectUniqueKey,
        JsonObjectType, Revision, ServerCloudObject,
    },
    server::{ids::SyncId, sync_queue::QueueItem},
};
use serde::{Deserialize, Serialize};
use std::fmt;
use warpui::AppContext;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GithubRepo {
    /// Repository owner (e.g. "warpdotdev")
    pub owner: String,
    /// Repository name (e.g. "warp-internal")
    pub repo: String,
}

impl GithubRepo {
    pub fn new(owner: String, repo: String) -> Self {
        Self { owner, repo }
    }
}

impl fmt::Display for GithubRepo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.repo)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BaseImage {
    DockerImage(String),
}

impl fmt::Display for BaseImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BaseImage::DockerImage(s) => s.fmt(f),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GcpProviderConfig {
    pub project_number: String,
    pub workload_identity_federation_pool_id: String,
    pub workload_identity_federation_provider_id: String,
    /// Service account email for impersonation. When set, the federated token
    /// is exchanged for a service account access token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_account_email: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AwsProviderConfig {
    pub role_arn: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct ProvidersConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gcp: Option<GcpProviderConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aws: Option<AwsProviderConfig>,
}

impl ProvidersConfig {
    pub fn is_empty(&self) -> bool {
        self.gcp.is_none() && self.aws.is_none()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Environment settings used when preparing a local agent workspace.
pub struct AmbientAgentEnvironment {
    /// Environment name
    #[serde(default)]
    pub name: String,
    /// Optional description of the environment (max 240 characters)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// List of GitHub repositories
    #[serde(default)]
    pub github_repos: Vec<GithubRepo>,
    /// Base image specification
    #[serde(flatten)]
    pub base_image: BaseImage,
    /// List of setup commands to run after cloning
    #[serde(default)]
    pub setup_commands: Vec<String>,
    /// Optional provider configurations for automatic auth.
    #[serde(default, skip_serializing_if = "ProvidersConfig::is_empty")]
    pub providers: ProvidersConfig,
}

pub type AmbientAgentEnvironmentObject =
    GenericCloudObject<GenericStringObjectId, AmbientAgentEnvironmentObjectModel>;
pub type AmbientAgentEnvironmentObjectModel =
    GenericStringModel<AmbientAgentEnvironment, JsonSerializer>;

impl AmbientAgentEnvironmentObject {
    pub fn get_all(_app: &AppContext) -> Vec<AmbientAgentEnvironmentObject> {
        Vec::new()
    }

    pub fn get_by_id<'a>(
        _sync_id: &'a SyncId,
        _app: &'a AppContext,
    ) -> Option<&'a AmbientAgentEnvironmentObject> {
        None
    }
}

impl AmbientAgentEnvironment {
    pub fn new(
        name: String,
        description: Option<String>,
        github_repos: Vec<GithubRepo>,
        docker_image: String,
        setup_commands: Vec<String>,
    ) -> Self {
        Self {
            name,
            description,
            github_repos,
            base_image: BaseImage::DockerImage(docker_image),
            setup_commands,
            providers: ProvidersConfig::default(),
        }
    }
}

impl StringModel for AmbientAgentEnvironment {
    type CloudObjectType = AmbientAgentEnvironmentObject;

    fn model_type_name(&self) -> &'static str {
        "Agent environment"
    }

    fn should_enforce_revisions() -> bool {
        true
    }

    fn model_format() -> GenericStringObjectFormat {
        GenericStringObjectFormat::Json(JsonObjectType::AgentEnvironment)
    }

    fn display_name(&self) -> String {
        self.name.clone()
    }

    fn update_object_queue_item(
        &self,
        revision_ts: Option<Revision>,
        object: &AmbientAgentEnvironmentObject,
    ) -> QueueItem {
        QueueItem::UpdateAgentEnvironment {
            model: object.model().clone().into(),
            id: object.id,
            revision: revision_ts.or_else(|| object.metadata.revision.clone()),
        }
    }

    fn uniqueness_key(&self) -> Option<GenericStringObjectUniqueKey> {
        None
    }

    fn new_from_server_update(&self, server_cloud_object: &ServerCloudObject) -> Option<Self> {
        if let ServerCloudObject::AmbientAgentEnvironment(server_environment) = server_cloud_object
        {
            return Some(server_environment.model.clone().string_model);
        }
        None
    }

    fn should_show_activity_toasts() -> bool {
        false
    }

    fn warn_if_unsaved_at_quit() -> bool {
        true
    }
}

impl JsonModel for AmbientAgentEnvironment {
    fn json_object_type() -> JsonObjectType {
        JsonObjectType::AgentEnvironment
    }
}

/// Resolves the current owner for creating new environments.
///
/// If the user is on a team, returns `Owner::Team`. Otherwise, returns
/// `Owner::User` with the current user's ID. Returns `None` if the user
/// is not logged in.
pub fn owner_for_new_environment(
    _ctx: &AppContext,
) -> Option<warp_server_client::cloud_object::Owner> {
    None
}

/// Resolves the current owner for creating new personal environments.
///
/// Returns `Owner::User` with the current user's ID. Returns `None` if the user
/// is not logged in.
pub fn owner_for_new_personal_environment(
    _ctx: &AppContext,
) -> Option<warp_server_client::cloud_object::Owner> {
    None
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
