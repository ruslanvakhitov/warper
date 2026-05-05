use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use warp_graphql::ai::PlatformErrorCode;

pub use crate::ai::agent::conversation::AmbientAgentTaskId;
use crate::ai::artifacts::Artifact;

pub mod task {
    pub use super::{AgentConfigSnapshot, TaskCreatorInfo, TaskStatusMessage};
    pub use crate::ai::agent_sdk::config_file::{HarnessAuthSecretsConfig, HarnessConfig};
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct AgentConfigSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_spec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub computer_use_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness: Option<crate::ai::agent_sdk::config_file::HarnessConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_auth_secrets: Option<crate::ai::agent_sdk::config_file::HarnessAuthSecretsConfig>,
}

impl AgentConfigSnapshot {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AgentSource {
    Linear,
    AgentWebhook,
    Slack,
    Cli,
    ScheduledAgent,
    Interactive,
    RemovedHosted,
    GitHubAction,
}

impl AgentSource {
    pub fn as_str(&self) -> &str {
        match self {
            AgentSource::Linear => "LINEAR",
            AgentSource::AgentWebhook => "API",
            AgentSource::Slack => "SLACK",
            AgentSource::Cli => "CLI",
            AgentSource::ScheduledAgent => "SCHEDULED_AGENT",
            AgentSource::Interactive => "LOCAL",
            AgentSource::RemovedHosted => "REMOVED_HOSTED",
            AgentSource::GitHubAction => "GITHUB_ACTION",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            AgentSource::Linear => "Linear",
            AgentSource::AgentWebhook => "API",
            AgentSource::Slack => "Slack",
            AgentSource::Cli => "CLI",
            AgentSource::ScheduledAgent => "Scheduled",
            AgentSource::Interactive => "Local agent",
            AgentSource::RemovedHosted => "Removed hosted agent",
            AgentSource::GitHubAction => "GitHub Action",
        }
    }

    pub fn is_user_initiated(&self) -> bool {
        matches!(
            self,
            AgentSource::Linear | AgentSource::Slack | AgentSource::Interactive
        )
    }
}

impl AmbientAgentTaskState {
    pub fn is_failure_like(&self) -> bool {
        matches!(
            self,
            AmbientAgentTaskState::Failed
                | AmbientAgentTaskState::Error
                | AmbientAgentTaskState::Blocked
                | AmbientAgentTaskState::Cancelled
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AmbientAgentTaskState {
    Queued,
    Pending,
    Claimed,
    InProgress,
    Succeeded,
    Failed,
    Error,
    Blocked,
    Cancelled,
    Unknown,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TaskStatusMessage {
    pub message: String,
    #[serde(default)]
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TaskCreatorInfo {
    pub uid: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RequestUsage {
    pub requests_used: i32,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AmbientAgentTask {
    pub task_id: AmbientAgentTaskId,
    #[serde(default)]
    pub parent_run_id: Option<String>,
    pub title: String,
    pub state: AmbientAgentTaskState,
    pub prompt: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub status_message: Option<TaskStatusMessage>,
    #[serde(default)]
    pub source: Option<AgentSource>,
    pub session_id: Option<String>,
    pub session_link: Option<String>,
    pub creator: Option<TaskCreatorInfo>,
    pub conversation_id: Option<String>,
    pub request_usage: Option<RequestUsage>,
    pub is_sandbox_running: bool,
    #[serde(default, alias = "agent_config")]
    pub agent_config_snapshot: Option<AgentConfigSnapshot>,
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
    #[serde(default)]
    pub last_event_sequence: Option<i64>,
    #[serde(default)]
    pub children: Vec<String>,
}

impl AmbientAgentTask {
    pub fn creator_display_name(&self) -> Option<String> {
        self.creator
            .as_ref()
            .and_then(|creator| creator.name.clone().or_else(|| creator.email.clone()))
    }

    pub fn credits_used(&self) -> Option<f32> {
        self.request_usage
            .as_ref()
            .map(|usage| usage.requests_used as f32)
    }

    pub fn run_time(&self) -> Option<chrono::Duration> {
        let started_at = self.started_at?;
        Some(self.updated_at - started_at)
    }
}

/// A status update for a task, optionally including a platform error code.
pub struct TaskStatusUpdate {
    pub message: String,
    pub error_code: Option<PlatformErrorCode>,
}

impl TaskStatusUpdate {
    pub fn message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            error_code: None,
        }
    }

    pub fn with_error_code(message: impl Into<String>, error_code: PlatformErrorCode) -> Self {
        Self {
            message: message.into(),
            error_code: Some(error_code),
        }
    }
}

/// Response from the artifact endpoint.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "artifact_type")]
pub enum ArtifactDownloadResponse {
    #[serde(rename = "SCREENSHOT")]
    Screenshot {
        #[serde(flatten)]
        common: ArtifactDownloadCommonFields,
        data: ScreenshotArtifactResponseData,
    },
    #[serde(rename = "FILE")]
    File {
        #[serde(flatten)]
        common: ArtifactDownloadCommonFields,
        data: FileArtifactResponseData,
    },
}

impl ArtifactDownloadResponse {
    fn common(&self) -> &ArtifactDownloadCommonFields {
        match self {
            ArtifactDownloadResponse::Screenshot { common, .. }
            | ArtifactDownloadResponse::File { common, .. } => common,
        }
    }

    pub fn artifact_uid(&self) -> &str {
        &self.common().artifact_uid
    }

    pub fn artifact_type(&self) -> &'static str {
        match self {
            ArtifactDownloadResponse::Screenshot { .. } => "SCREENSHOT",
            ArtifactDownloadResponse::File { .. } => "FILE",
        }
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.common().created_at
    }

    pub fn download_url(&self) -> &str {
        match self {
            ArtifactDownloadResponse::Screenshot { data, .. } => &data.download_url,
            ArtifactDownloadResponse::File { data, .. } => &data.download_url,
        }
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        match self {
            ArtifactDownloadResponse::Screenshot { data, .. } => data.expires_at,
            ArtifactDownloadResponse::File { data, .. } => data.expires_at,
        }
    }

    pub fn content_type(&self) -> &str {
        match self {
            ArtifactDownloadResponse::Screenshot { data, .. } => &data.content_type,
            ArtifactDownloadResponse::File { data, .. } => &data.content_type,
        }
    }

    pub fn filepath(&self) -> Option<&str> {
        match self {
            ArtifactDownloadResponse::Screenshot { .. } => None,
            ArtifactDownloadResponse::File { data, .. } => Some(&data.filepath),
        }
    }

    pub fn filename(&self) -> Option<&str> {
        match self {
            ArtifactDownloadResponse::Screenshot { .. } => None,
            ArtifactDownloadResponse::File { data, .. } => Some(&data.filename),
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            ArtifactDownloadResponse::Screenshot { data, .. } => data.description.as_deref(),
            ArtifactDownloadResponse::File { data, .. } => data.description.as_deref(),
        }
    }

    pub fn size_bytes(&self) -> Option<i64> {
        match self {
            ArtifactDownloadResponse::Screenshot { .. } => None,
            ArtifactDownloadResponse::File { data, .. } => data.size_bytes,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ArtifactDownloadCommonFields {
    pub artifact_uid: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ScreenshotArtifactResponseData {
    pub download_url: String,
    pub expires_at: DateTime<Utc>,
    pub content_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FileArtifactResponseData {
    pub download_url: String,
    pub expires_at: DateTime<Utc>,
    pub content_type: String,
    pub filepath: String,
    pub filename: String,
    pub description: Option<String>,
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct FileArtifactRecord {
    pub artifact_uid: String,
    pub filepath: String,
    pub description: Option<String>,
    pub mime_type: String,
    pub size_bytes: Option<i32>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct SpawnAgentRequest {
    pub prompt: String,
    pub config: Option<AgentConfigSnapshot>,
    pub title: Option<String>,
    pub team: Option<String>,
    pub skill: Option<String>,
    pub attachments: Vec<serde_json::Value>,
    pub interactive: Option<bool>,
    pub parent_run_id: Option<String>,
    pub runtime_skills: Vec<String>,
    pub referenced_attachments: Vec<serde_json::Value>,
}
