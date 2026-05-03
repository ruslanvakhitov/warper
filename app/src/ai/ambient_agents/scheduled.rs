use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::future::Future;

use super::AgentConfigSnapshot;

use crate::{
    cloud_object::{
        model::{
            generic_string_model::{GenericStringModel, GenericStringObjectId, StringModel},
            json_model::{JsonModel, JsonSerializer},
        },
        GenericCloudObject, GenericStringObjectFormat, GenericStringObjectUniqueKey,
        JsonObjectType, Owner, Revision, ServerCloudObject,
    },
    server::{ids::SyncId, sync_queue::QueueItem},
};
use warp_graphql::queries::get_scheduled_agent_history::ScheduledAgentHistory;
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// A ScheduledAmbientAgent represents configuration for ambient agents that run on a cron schedule.
pub struct ScheduledAmbientAgent {
    /// Agent name
    #[serde(default)]
    pub name: String,
    /// Cron schedule expression
    #[serde(default)]
    pub cron_schedule: String,
    /// Whether the scheduled agent is enabled
    #[serde(default)]
    pub enabled: bool,
    /// The prompt to use for the scheduled agent
    #[serde(default)]
    pub prompt: String,
    /// The latest failure to execute this scheduled agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_spawn_error: Option<String>,
    /// Configuration for how the ambient agent should run.
    #[serde(default, skip_serializing_if = "AgentConfigSnapshot::is_empty")]
    pub agent_config: AgentConfigSnapshot,
}

pub type CloudScheduledAmbientAgent =
    GenericCloudObject<GenericStringObjectId, CloudScheduledAmbientAgentModel>;
pub type CloudScheduledAmbientAgentModel =
    GenericStringModel<ScheduledAmbientAgent, JsonSerializer>;

impl CloudScheduledAmbientAgent {
    pub fn get_all(_app: &AppContext) -> Vec<CloudScheduledAmbientAgent> {
        Vec::new()
    }

    pub fn get_by_id<'a>(
        _sync_id: &'a SyncId,
        _app: &'a AppContext,
    ) -> Option<&'a CloudScheduledAmbientAgent> {
        None
    }
}

impl ScheduledAmbientAgent {
    pub fn new(name: String, cron_schedule: String, enabled: bool, prompt: String) -> Self {
        Self {
            name,
            cron_schedule,
            enabled,
            prompt,
            last_spawn_error: None,
            agent_config: Default::default(),
        }
    }
}

impl StringModel for ScheduledAmbientAgent {
    type CloudObjectType = CloudScheduledAmbientAgent;

    fn model_type_name(&self) -> &'static str {
        "Scheduled ambient agent"
    }

    fn should_enforce_revisions() -> bool {
        true
    }

    fn model_format() -> GenericStringObjectFormat {
        GenericStringObjectFormat::Json(JsonObjectType::ScheduledAmbientAgent)
    }

    fn display_name(&self) -> String {
        self.name.clone()
    }

    fn update_object_queue_item(
        &self,
        revision_ts: Option<Revision>,
        object: &CloudScheduledAmbientAgent,
    ) -> QueueItem {
        QueueItem::UpdateScheduledAmbientAgent {
            model: object.model().clone().into(),
            id: object.id,
            revision: revision_ts.or_else(|| object.metadata.revision.clone()),
        }
    }

    fn uniqueness_key(&self) -> Option<GenericStringObjectUniqueKey> {
        None
    }

    fn new_from_server_update(&self, server_cloud_object: &ServerCloudObject) -> Option<Self> {
        if let ServerCloudObject::ScheduledAmbientAgent(server_scheduled_agent) =
            server_cloud_object
        {
            return Some(server_scheduled_agent.model.clone().string_model);
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

impl JsonModel for ScheduledAmbientAgent {
    fn json_object_type() -> JsonObjectType {
        JsonObjectType::ScheduledAmbientAgent
    }
}

/// Parameters for updating a scheduled ambient agent.
pub struct UpdateScheduleParams {
    /// The new name of the scheduled agent. If not provided, the name will not be updated.
    pub name: Option<String>,
    /// The new cron schedule of the scheduled agent. If not provided, the cron schedule will not be updated.
    pub cron: Option<String>,
    /// The new model ID of the scheduled agent. If not provided, the model ID will not be updated.
    pub model_id: Option<String>,
    /// The new environment ID of the scheduled agent.
    /// If this is:
    /// * `Some(Some(id))`, the environment ID will be updated to the given ID.
    /// * `Some(None)`, the environment will be removed.
    /// * `None`, the environment will not be updated.
    pub environment_id: Option<Option<String>>,
    /// The new base prompt to use for the scheduled agent's configuration.
    /// If not provided, the base prompt will not be updated.
    pub base_prompt: Option<String>,
    /// The new prompt of the scheduled agent. If not provided, the prompt will not be updated.
    pub prompt: Option<String>,
    /// MCP servers to upsert into this schedule's agent config.
    ///
    /// Entries are merged by key, overwriting existing keys.
    pub mcp_servers_upsert: Option<Map<String, Value>>,
    /// MCP server names (keys) to remove from this schedule's agent config.
    pub remove_mcp_server_names: Vec<String>,
    /// The new skill spec for the scheduled agent.
    /// If this is:
    /// * `Some(Some(spec))`, the skill spec will be updated to the given value.
    /// * `Some(None)`, the skill will be removed.
    /// * `None`, the skill spec will not be updated.
    pub skill_spec: Option<Option<String>>,
    /// The new worker host for the scheduled agent.
    /// If not provided, the worker host will not be updated.
    /// Setting to "warp" or empty string reverts to Warp-hosted.
    pub worker_host: Option<String>,
}

pub struct ScheduledAgentManager;

#[cfg_attr(target_family = "wasm", allow(dead_code))]
impl ScheduledAgentManager {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }

    /// List all scheduled ambient agents currently present in the local cloud object store.
    pub fn list_schedules(&self, app: &AppContext) -> Vec<CloudScheduledAmbientAgent> {
        CloudScheduledAmbientAgent::get_all(app)
    }

    /// Get the execution history for a scheduled ambient agent.
    pub fn fetch_schedule_history(
        &self,
        schedule_id: SyncId,
        app: &AppContext,
    ) -> impl warpui::r#async::Spawnable<Output = anyhow::Result<Option<ScheduledAgentHistory>>>
    {
        let _ = app;
        async move {
            let _ = schedule_id;
            Err(anyhow::anyhow!(
                "Scheduled ambient agents are unavailable in local-only Warper"
            ))
        }
    }

    /// Create a new scheduled ambient agent.
    pub fn create_schedule(
        &mut self,
        config: ScheduledAmbientAgent,
        owner: Owner,
        _ctx: &mut ModelContext<Self>,
    ) -> impl Future<Output = anyhow::Result<SyncId>> + Send + 'static {
        async move {
            let _ = (config, owner);
            Err(anyhow::anyhow!(
                "Scheduled ambient agents are unavailable in local-only Warper"
            ))
        }
    }

    /// Pause a scheduled ambient agent.
    pub fn pause_schedule(
        &mut self,
        schedule_id: SyncId,
        _ctx: &mut ModelContext<Self>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send + 'static {
        async move {
            let _ = schedule_id;
            Err(anyhow::anyhow!(
                "Scheduled ambient agents are unavailable in local-only Warper"
            ))
        }
    }

    /// Unpause a scheduled ambient agent.
    pub fn unpause_schedule(
        &mut self,
        schedule_id: SyncId,
        _ctx: &mut ModelContext<Self>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send + 'static {
        async move {
            let _ = schedule_id;
            Err(anyhow::anyhow!(
                "Scheduled ambient agents are unavailable in local-only Warper"
            ))
        }
    }

    /// Update a scheduled ambient agent.
    pub fn update_schedule(
        &mut self,
        schedule_id: SyncId,
        params: UpdateScheduleParams,
        _ctx: &mut ModelContext<Self>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send + 'static {
        async move {
            let _ = (schedule_id, params);
            Err(anyhow::anyhow!(
                "Scheduled ambient agents are unavailable in local-only Warper"
            ))
        }
    }

    /// Delete a scheduled ambient agent.
    pub fn delete_schedule(
        &mut self,
        schedule_id: SyncId,
        _ctx: &mut ModelContext<Self>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send + 'static {
        async move {
            let _ = schedule_id;
            Err(anyhow::anyhow!(
                "Scheduled ambient agents are unavailable in local-only Warper"
            ))
        }
    }
}

impl Entity for ScheduledAgentManager {
    type Event = ();
}

impl SingletonEntity for ScheduledAgentManager {}
