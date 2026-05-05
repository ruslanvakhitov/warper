use anyhow::anyhow;
use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Utc};
use cynic::{MutationBuilder, QueryBuilder};
use itertools::Itertools;
#[cfg(test)]
use mockall::automock;
use prost::Message;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use warp_core::report_error;
use warp_multi_agent_api::ConversationData;

use super::ServerApi;
use crate::ai::agent::api::ServerConversationToken;
use crate::ai::agent::conversation::AmbientAgentTaskId;
use crate::ai::agent::conversation::{
    AIAgentConversationFormat, AIAgentHarness, AIAgentSerializedBlockFormat,
    ServerAIConversationMetadata, ServerConversationObjectMetadata, ServerConversationPermissions,
};
use crate::ai::artifacts::Artifact;
use crate::ai::generate_code_review_content::api::{
    GenerateCodeReviewContentRequest, GenerateCodeReviewContentResponse,
};
#[cfg(feature = "agent_mode_evals")]
use crate::ai::request_limits::RequestLimitInfo;
#[cfg(not(feature = "agent_mode_evals"))]
use crate::ai::BonusGrant;
use crate::persistence::model::ConversationUsageMetadata;
use crate::terminal::model::block::SerializedBlock;
#[cfg(not(feature = "agent_mode_evals"))]
use crate::{
    ai::request_limits::BonusGrantScope, server::ids::ServerId, workspaces::workspace::WorkspaceUid,
};
use crate::{
    ai::{
        llms::{
            AvailableLLMs, DisableReason, LLMInfo, LLMModelHost, LLMProvider, LLMSpec,
            LLMUsageMetadata, ModelsByFeature, RoutingHostConfig,
        },
        RequestUsageInfo,
    },
    ai_assistant::{
        execution_context::WarpAiExecutionContext, requests::GenerateDialogueResult,
        utils::TranscriptPart, AIGeneratedCommand, GenerateCommandsFromNaturalLanguageError,
    },
    server::graphql::{get_request_context, get_user_facing_error_message},
};
use ai::index::full_source_code_embedding::{
    self,
    store_client::{IntermediateNode, StoreClient},
    CodebaseContextConfig, ContentHash, EmbeddingConfig, NodeHash, RepoMetadata,
};
#[cfg(not(feature = "agent_mode_evals"))]
use warp_graphql::queries::get_request_limit_info::{
    GetRequestLimitInfo, GetRequestLimitInfoVariables,
};
use warp_graphql::{
    ai::PlatformErrorCode,
    mutations::{
        delete_ai_conversation::{
            DeleteAIConversation, DeleteAIConversationVariables, DeleteConversationInput,
            DeleteConversationResult,
        },
        generate_code_embeddings::{
            GenerateCodeEmbeddings, GenerateCodeEmbeddingsInput, GenerateCodeEmbeddingsResult,
            GenerateCodeEmbeddingsVariables,
        },
        generate_commands::{
            GenerateCommands, GenerateCommandsInput, GenerateCommandsResult,
            GenerateCommandsStatus, GenerateCommandsVariables,
        },
        generate_dialogue::{
            GenerateDialogue, GenerateDialogueInput,
            GenerateDialogueResult as GenerateDialogueResultGraphql, GenerateDialogueStatus,
            GenerateDialogueVariables, TranscriptPart as TranscriptPartGraphql,
        },
        generate_metadata_for_command::{
            GenerateMetadataForCommand, GenerateMetadataForCommandInput,
            GenerateMetadataForCommandResult, GenerateMetadataForCommandStatus,
            GenerateMetadataForCommandVariables,
        },
        populate_merkle_tree_cache::{
            PopulateMerkleTreeCache, PopulateMerkleTreeCacheResult,
            PopulateMerkleTreeCacheVariables,
        },
        request_bonus::{
            ProvideNegativeFeedbackResponseForAiConversation,
            ProvideNegativeFeedbackResponseForAiConversationInput,
            ProvideNegativeFeedbackResponseForAiConversationVariables, RequestsRefundedResult,
        },
        update_merkle_tree::{
            MerkleTreeNode, UpdateMerkleTree, UpdateMerkleTreeInput, UpdateMerkleTreeResult,
            UpdateMerkleTreeVariables,
        },
    },
    queries::{
        codebase_context_config::{
            CodebaseContextConfigQuery, CodebaseContextConfigResult, CodebaseContextConfigVariables,
        },
        get_feature_model_choices::{GetFeatureModelChoices, GetFeatureModelChoicesVariables},
        get_relevant_fragments::{
            GetRelevantFragmentsQuery, GetRelevantFragmentsResult, GetRelevantFragmentsVariables,
        },
        rerank_fragments::{RerankFragments, RerankFragmentsResult, RerankFragmentsVariables},
        sync_merkle_tree::{
            SyncMerkleTree, SyncMerkleTreeInput, SyncMerkleTreeResult, SyncMerkleTreeVariables,
        },
    },
};

#[cfg(not(feature = "agent_mode_evals"))]
const PLACEHOLDER_WORKSPACE_UID: &str = "NOT_A_REAL_WORKSPACE_UID";

#[cfg(not(feature = "agent_mode_evals"))]
impl BonusGrant {
    fn from_gql_bonus_grant(
        bonus_grant: warp_graphql::billing::BonusGrant,
        scope: BonusGrantScope,
    ) -> Self {
        Self {
            created_at: bonus_grant.created_at.utc(),
            cost_cents: bonus_grant.cost_cents,
            expiration: bonus_grant.expiration.map(|exp| exp.utc()),
            grant_type: bonus_grant.grant_type,
            reason: bonus_grant.reason,
            user_facing_message: bonus_grant.user_facing_message,
            request_credits_granted: bonus_grant.request_credits_granted,
            request_credits_remaining: bonus_grant.request_credits_remaining,
            scope,
        }
    }
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GeneratedCommandMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GeneratedCommandMetadataError {
    RateLimited,
    Other,
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

const AI_ASSISTANT_REQUEST_TIMEOUT_SECONDS: u64 = 30;

/// A status update for a task, optionally including a platform error code.
pub struct TaskStatusUpdate {
    pub message: String,
    pub error_code: Option<PlatformErrorCode>,
}

impl TaskStatusUpdate {
    /// Create a status update with just a message (no error code).
    pub fn message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            error_code: None,
        }
    }

    /// Create a status update with a message and error code.
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

/// Screenshot-specific data from the artifact endpoint.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ScreenshotArtifactResponseData {
    pub download_url: String,
    pub expires_at: DateTime<Utc>,
    pub content_type: String,
    pub description: Option<String>,
}

/// File-specific data from the artifact endpoint.
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

#[cfg_attr(test, automock)]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait AIClient: 'static + Send + Sync {
    async fn generate_commands_from_natural_language(
        &self,
        prompt: String,
        ai_execution_context: Option<WarpAiExecutionContext>,
    ) -> Result<Vec<AIGeneratedCommand>, GenerateCommandsFromNaturalLanguageError>;

    async fn generate_dialogue_answer(
        &self,
        transcript: Vec<TranscriptPart>,
        prompt: String,
        ai_execution_context: Option<WarpAiExecutionContext>,
    ) -> anyhow::Result<GenerateDialogueResult>;

    async fn generate_metadata_for_command(
        &self,
        command: String,
    ) -> Result<GeneratedCommandMetadata, GeneratedCommandMetadataError>;

    async fn get_request_limit_info(&self) -> Result<RequestUsageInfo, anyhow::Error>;

    async fn get_feature_model_choices(&self) -> Result<ModelsByFeature, anyhow::Error>;

    /// Fetches the free-tier available models without requiring authentication.
    /// Used during pre-login onboarding so logged-out users see an accurate model list
    /// instead of the hard-coded `ModelsByFeature::default()` fallback.
    async fn get_free_available_models(
        &self,
        referrer: Option<String>,
    ) -> Result<ModelsByFeature, anyhow::Error>;

    async fn update_merkle_tree(
        &self,
        embedding_config: EmbeddingConfig,
        nodes: Vec<IntermediateNode>,
    ) -> anyhow::Result<HashMap<NodeHash, bool>>;

    async fn generate_code_embeddings(
        &self,
        embedding_config: EmbeddingConfig,
        fragments: Vec<full_source_code_embedding::Fragment>,
        root_hash: NodeHash,
        repo_metadata: RepoMetadata,
    ) -> anyhow::Result<HashMap<ContentHash, bool>>;

    async fn provide_negative_feedback_response_for_ai_conversation(
        &self,
        conversation_id: String,
        request_ids: Vec<String>,
    ) -> anyhow::Result<i32, anyhow::Error>;

    async fn get_ai_conversation(
        &self,
        server_conversation_token: ServerConversationToken,
    ) -> anyhow::Result<(ConversationData, ServerAIConversationMetadata), anyhow::Error>;

    async fn list_ai_conversation_metadata(
        &self,
        conversation_ids: Option<Vec<String>>,
    ) -> anyhow::Result<Vec<ServerAIConversationMetadata>>;

    async fn get_ai_conversation_format(
        &self,
        server_conversation_token: ServerConversationToken,
    ) -> anyhow::Result<AIAgentConversationFormat, anyhow::Error>;

    async fn get_block_snapshot(
        &self,
        server_conversation_token: ServerConversationToken,
    ) -> anyhow::Result<SerializedBlock, anyhow::Error>;

    async fn delete_ai_conversation(
        &self,
        server_conversation_token: String,
    ) -> anyhow::Result<(), anyhow::Error>;

    /// Generates AI copy for code-review flows: commit messages at dialog-open
    /// time and PR titles / bodies at confirm time. `output_type` in the
    /// request picks which of the three the server returns.
    async fn generate_code_review_content(
        &self,
        request: GenerateCodeReviewContentRequest,
    ) -> Result<GenerateCodeReviewContentResponse, anyhow::Error>;
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl AIClient for ServerApi {
    async fn generate_commands_from_natural_language(
        &self,
        prompt: String,
        // TODO: use relevant context from RequestContext and deprecate usage of ai_execution_context
        _ai_execution_context: Option<WarpAiExecutionContext>,
    ) -> Result<Vec<AIGeneratedCommand>, GenerateCommandsFromNaturalLanguageError> {
        let default_err = GenerateCommandsFromNaturalLanguageError::Other;

        let variables = GenerateCommandsVariables {
            input: GenerateCommandsInput { prompt },
            request_context: get_request_context(),
        };

        let operation = GenerateCommands::build(variables);
        let response = self
            .send_graphql_request(
                operation,
                Some(Duration::from_secs(AI_ASSISTANT_REQUEST_TIMEOUT_SECONDS)),
            )
            .await
            .map_err(|_| default_err)?;

        match response.generate_commands {
            GenerateCommandsResult::GenerateCommandsOutput(output) => match output.status {
                GenerateCommandsStatus::GenerateCommandsSuccess(success) => {
                    Ok(success.commands.into_iter().map(Into::into).collect_vec())
                }
                GenerateCommandsStatus::GenerateCommandsFailure(failure) => {
                    Err(failure.type_.into())
                }
                GenerateCommandsStatus::Unknown => {
                    Err(GenerateCommandsFromNaturalLanguageError::Other)
                }
            },
            _ => Err(GenerateCommandsFromNaturalLanguageError::Other),
        }
    }

    async fn generate_dialogue_answer(
        &self,
        transcript: Vec<TranscriptPart>,
        prompt: String,
        // TODO: use relevant context from RequestContext and deprecate usage of ai_execution_context
        _ai_execution_context: Option<WarpAiExecutionContext>,
    ) -> anyhow::Result<GenerateDialogueResult> {
        let graphql_transcript: Vec<TranscriptPartGraphql> = transcript
            .into_iter()
            .map(|part| TranscriptPartGraphql {
                user: part.raw_user_prompt().to_string(),
                assistant: part.raw_assistant_answer().to_string(),
            })
            .collect();
        let variables = GenerateDialogueVariables {
            input: GenerateDialogueInput {
                transcript: graphql_transcript,
                prompt,
            },
            request_context: get_request_context(),
        };

        let operation = GenerateDialogue::build(variables);
        let response = self
            .send_graphql_request(
                operation,
                Some(Duration::from_secs(AI_ASSISTANT_REQUEST_TIMEOUT_SECONDS)),
            )
            .await?;
        match response.generate_dialogue {
            GenerateDialogueResultGraphql::GenerateDialogueOutput(output) => match output.status {
                GenerateDialogueStatus::GenerateDialogueSuccess(success) => {
                    Ok(GenerateDialogueResult::Success {
                        answer: success.answer,
                        truncated: success.truncated,
                        request_limit_info: success.request_limit_info.into(),
                        transcript_summarized: success.transcript_summarized,
                    })
                }
                GenerateDialogueStatus::GenerateDialogueFailure(failure) => {
                    Ok(GenerateDialogueResult::Failure {
                        request_limit_info: failure.request_limit_info.into(),
                    })
                }
                GenerateDialogueStatus::Unknown => Err(anyhow!("failed to generate AI dialogue")),
            },
            GenerateDialogueResultGraphql::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)))
            }
            GenerateDialogueResultGraphql::Unknown => {
                Err(anyhow!("failed to generate AI dialogue"))
            }
        }
    }

    async fn generate_metadata_for_command(
        &self,
        command: String,
    ) -> Result<GeneratedCommandMetadata, GeneratedCommandMetadataError> {
        let default_err = GeneratedCommandMetadataError::Other;
        let variables = GenerateMetadataForCommandVariables {
            input: GenerateMetadataForCommandInput { command },
            request_context: get_request_context(),
        };

        let operation = GenerateMetadataForCommand::build(variables);
        let response = self
            .send_graphql_request(
                operation,
                Some(Duration::from_secs(AI_ASSISTANT_REQUEST_TIMEOUT_SECONDS)),
            )
            .await
            .map_err(|_| default_err)?;

        match response.generate_metadata_for_command {
            GenerateMetadataForCommandResult::GenerateMetadataForCommandOutput(output) => {
                match output.status {
                    GenerateMetadataForCommandStatus::GenerateMetadataForCommandSuccess(
                        success,
                    ) => Ok(GeneratedCommandMetadata {
                        title: Some(success.title),
                        description: Some(success.description),
                    }),
                    GenerateMetadataForCommandStatus::GenerateMetadataForCommandFailure(
                        failure,
                    ) => Err(match failure.type_ {
                        warp_graphql::mutations::generate_metadata_for_command::GenerateMetadataForCommandFailureType::RateLimited => {
                            GeneratedCommandMetadataError::RateLimited
                        }
                        _ => GeneratedCommandMetadataError::Other,
                    }),
                    GenerateMetadataForCommandStatus::Unknown => {
                        Err(GeneratedCommandMetadataError::Other)
                    }
                }
            }
            _ => Err(GeneratedCommandMetadataError::Other),
        }
    }

    #[cfg(feature = "agent_mode_evals")]
    async fn get_request_limit_info(&self) -> Result<RequestUsageInfo, anyhow::Error> {
        Ok(RequestUsageInfo {
            request_limit_info: RequestLimitInfo::new_for_evals(),
            bonus_grants: vec![],
        })
    }

    #[cfg(not(feature = "agent_mode_evals"))]
    async fn get_request_limit_info(&self) -> Result<RequestUsageInfo, anyhow::Error> {
        let variables = GetRequestLimitInfoVariables {
            request_context: get_request_context(),
        };
        let operation = GetRequestLimitInfo::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.user {
            warp_graphql::queries::get_request_limit_info::UserResult::UserOutput(user_output) => {
                let request_limit_info = user_output.user.request_limit_info.into();

                let workspace_bonus_grants = user_output
                    .user
                    .workspaces
                    .into_iter()
                    .filter(|workspace| workspace.uid != PLACEHOLDER_WORKSPACE_UID.into())
                    .flat_map(|workspace| {
                        let workspace_uid =
                            WorkspaceUid::from(ServerId::from_string_lossy(workspace.uid.inner()));
                        workspace
                            .bonus_grants_info
                            .grants
                            .into_iter()
                            .map(move |grant| {
                                BonusGrant::from_gql_bonus_grant(
                                    grant,
                                    BonusGrantScope::Workspace(workspace_uid),
                                )
                            })
                    });

                let bonus_grants: Vec<BonusGrant> = user_output
                    .user
                    .bonus_grants
                    .into_iter()
                    .map(|grant| BonusGrant::from_gql_bonus_grant(grant, BonusGrantScope::User))
                    .chain(workspace_bonus_grants)
                    .collect();

                Ok(RequestUsageInfo {
                    request_limit_info,
                    bonus_grants,
                })
            }
            warp_graphql::queries::get_request_limit_info::UserResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)))
            }
            warp_graphql::queries::get_request_limit_info::UserResult::Unknown => {
                Err(anyhow!("failed to get request limit info"))
            }
        }
    }

    async fn get_feature_model_choices(&self) -> Result<ModelsByFeature, anyhow::Error> {
        let variables = GetFeatureModelChoicesVariables {
            request_context: get_request_context(),
        };
        let operation = GetFeatureModelChoices::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.user {
            warp_graphql::queries::get_feature_model_choices::UserResult::UserOutput(
                warp_graphql::queries::get_feature_model_choices::UserOutput {
                    user: warp_graphql::queries::get_feature_model_choices::User { mut workspaces },
                },
            ) if !workspaces.is_empty() => {
                // This is safe (`remove()` can panic) because we ensure workspaces is non-empty
                // above.
                workspaces.remove(0).feature_model_choice.try_into()
            }
            _ => Err(anyhow!("Failed to get available feature model choices")),
        }
    }

    async fn get_free_available_models(
        &self,
        referrer: Option<String>,
    ) -> Result<ModelsByFeature, anyhow::Error> {
        let _ = referrer;
        Err(anyhow!(
            "Hosted model-choice discovery is unavailable in Warper"
        ))
    }

    async fn update_merkle_tree(
        &self,
        embedding_config: EmbeddingConfig,
        nodes: Vec<IntermediateNode>,
    ) -> anyhow::Result<HashMap<NodeHash, bool>> {
        let nodes = nodes
            .into_iter()
            .map(|node| MerkleTreeNode {
                hash: node.hash.into(),
                children: node.children.into_iter().map(Into::into).collect(),
            })
            .collect_vec();
        let variables = UpdateMerkleTreeVariables {
            input: UpdateMerkleTreeInput {
                embedding_config: embedding_config.into(),
                nodes,
            },
            request_context: get_request_context(),
        };
        let operation = UpdateMerkleTree::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.update_merkle_tree {
            UpdateMerkleTreeResult::UpdateMerkleTreeOutput(output) => {
                let mut node_results = HashMap::with_capacity(output.results.len());
                for result in output.results {
                    node_results.insert(result.hash.try_into()?, result.success);
                }
                Ok(node_results)
            }
            UpdateMerkleTreeResult::UpdateMerkleTreeError(e) => Err(anyhow!(e.error)),
            UpdateMerkleTreeResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)))
            }
            UpdateMerkleTreeResult::Unknown => Err(anyhow!("failed to update merkle tree")),
        }
    }

    async fn generate_code_embeddings(
        &self,
        embedding_config: EmbeddingConfig,
        fragments: Vec<full_source_code_embedding::Fragment>,
        root_hash: NodeHash,
        repo_metadata: RepoMetadata,
    ) -> anyhow::Result<HashMap<ContentHash, bool>> {
        let variables = GenerateCodeEmbeddingsVariables {
            input: GenerateCodeEmbeddingsInput {
                embedding_config: embedding_config.into(),
                fragments: fragments.into_iter().map(Into::into).collect(),
                repo_metadata: repo_metadata.into(),
                root_hash: root_hash.into(),
            },
            request_context: get_request_context(),
        };

        let operation = GenerateCodeEmbeddings::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.generate_code_embeddings {
            GenerateCodeEmbeddingsResult::GenerateCodeEmbeddingsOutput(output) => {
                let mut results = HashMap::with_capacity(output.embedding_results.len());
                for result in output.embedding_results {
                    results.insert(result.hash.try_into()?, result.success);
                }
                Ok(results)
            }
            GenerateCodeEmbeddingsResult::GenerateCodeEmbeddingsError(e) => Err(anyhow!(e.error)),
            GenerateCodeEmbeddingsResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)))
            }
            GenerateCodeEmbeddingsResult::Unknown => {
                Err(anyhow!("failed to generate code embeddings"))
            }
        }
    }

    async fn provide_negative_feedback_response_for_ai_conversation(
        &self,
        conversation_id: String,
        request_ids: Vec<String>,
    ) -> anyhow::Result<i32, anyhow::Error> {
        let variables = ProvideNegativeFeedbackResponseForAiConversationVariables {
            input: ProvideNegativeFeedbackResponseForAiConversationInput {
                conversation_id: conversation_id.into(),
                request_ids: request_ids.into_iter().map(Into::into).collect(),
            },
            request_context: get_request_context(),
        };

        let operation = ProvideNegativeFeedbackResponseForAiConversation::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.provide_negative_feedback_response_for_ai_conversation {
            RequestsRefundedResult::RequestsRefundedOutput(output) => Ok(output.requests_refunded),
            RequestsRefundedResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)))
            }
            RequestsRefundedResult::Unknown => Err(anyhow!(
                "failed to provide negative feedback response for ai conversation"
            )),
        }
    }

    async fn get_ai_conversation(
        &self,
        server_conversation_token: ServerConversationToken,
    ) -> anyhow::Result<(ConversationData, ServerAIConversationMetadata), anyhow::Error> {
        use warp_graphql::queries::list_ai_conversations::{
            ListAIConversations, ListAIConversationsInput, ListAIConversationsResult,
            ListAIConversationsVariables,
        };

        let conversation_id = server_conversation_token.as_str().to_string();
        let operation = ListAIConversations::build(ListAIConversationsVariables {
            input: ListAIConversationsInput {
                conversation_ids: Some(vec![cynic::Id::new(conversation_id)]),
            },
            request_context: get_request_context(),
        });
        let response = self.send_graphql_request(operation, None).await?;

        let gql_conversation = match response.list_ai_conversations {
            ListAIConversationsResult::ListAIConversationsOutput(output) => output
                .conversations
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("Conversation not found"))?,
            ListAIConversationsResult::UserFacingError(e) => {
                return Err(anyhow!(get_user_facing_error_message(e)));
            }
            ListAIConversationsResult::Unknown => {
                return Err(anyhow!("Failed to get AI conversation"));
            }
        };

        let conversation_data_bytes = base64::engine::general_purpose::STANDARD
            .decode(&gql_conversation.final_task_list)
            .map_err(|e| anyhow!("Failed to decode base64 conversation data: {e}"))?;

        let conversation_data = ConversationData::decode(conversation_data_bytes.as_slice())
            .map_err(|e| anyhow!("Failed to decode proto ConversationData: {e}"))?;

        // Build AIConversationMetadata from GraphQL response
        let metadata = gql_conversation.try_into()?;

        Ok((conversation_data, metadata))
    }

    async fn list_ai_conversation_metadata(
        &self,
        _conversation_ids: Option<Vec<String>>,
    ) -> anyhow::Result<Vec<ServerAIConversationMetadata>> {
        Ok(vec![])
    }

    async fn get_ai_conversation_format(
        &self,
        server_conversation_token: ServerConversationToken,
    ) -> anyhow::Result<AIAgentConversationFormat, anyhow::Error> {
        use warp_graphql::queries::get_ai_conversation_format::{
            GetAIConversationFormat, GetAIConversationFormatResult,
            GetAIConversationFormatVariables,
        };
        use warp_graphql::queries::list_ai_conversations::ListAIConversationsInput;

        let conversation_id = server_conversation_token.as_str().to_string();
        let operation = GetAIConversationFormat::build(GetAIConversationFormatVariables {
            input: ListAIConversationsInput {
                conversation_ids: Some(vec![cynic::Id::new(conversation_id)]),
            },
            request_context: get_request_context(),
        });
        let response = self.send_graphql_request(operation, None).await?;

        match response.list_ai_conversations {
            GetAIConversationFormatResult::ListAIConversationsOutput(output) => {
                let conversation = output
                    .conversations
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow!("Conversation not found"))?;
                Ok(convert_conversation_format(conversation.format))
            }
            GetAIConversationFormatResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)))
            }
            GetAIConversationFormatResult::Unknown => {
                Err(anyhow!("Failed to get AI conversation format"))
            }
        }
    }

    async fn get_block_snapshot(
        &self,
        server_conversation_token: ServerConversationToken,
    ) -> anyhow::Result<SerializedBlock, anyhow::Error> {
        let conversation_id = server_conversation_token.as_str();
        // Make sure to use `SerializedBlock::from_json` to correctly handle the serialized
        // command and output grid contents.
        let response = self
            .get_public_api_response(&format!(
                "agent/conversations/{conversation_id}/block-snapshot"
            ))
            .await?;
        let json_bytes = response
            .bytes()
            .await
            .map_err(|e| anyhow!("Failed to read block snapshot for {conversation_id}: {e}"))?;
        SerializedBlock::from_json(&json_bytes)
    }

    async fn delete_ai_conversation(
        &self,
        server_conversation_token: String,
    ) -> anyhow::Result<(), anyhow::Error> {
        let variables = DeleteAIConversationVariables {
            input: DeleteConversationInput {
                conversation_id: server_conversation_token.into(),
            },
            request_context: get_request_context(),
        };

        let operation = DeleteAIConversation::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.delete_conversation {
            DeleteConversationResult::DeleteConversationOutput(_) => Ok(()),
            DeleteConversationResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)))
            }
            DeleteConversationResult::Unknown => Err(anyhow!("Failed to delete AI conversation")),
        }
    }

    async fn generate_code_review_content(
        &self,
        request: GenerateCodeReviewContentRequest,
    ) -> Result<GenerateCodeReviewContentResponse, anyhow::Error> {
        let _ = request;
        Err(anyhow::anyhow!(
            "Hosted code-review generation is unavailable in Warper"
        ))
    }
}

impl TryFrom<warp_graphql::queries::get_feature_model_choices::FeatureModelChoice>
    for ModelsByFeature
{
    type Error = anyhow::Error;

    fn try_from(
        value: warp_graphql::queries::get_feature_model_choices::FeatureModelChoice,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            agent_mode: value.agent_mode.try_into()?,
            coding: value.coding.try_into()?,
            cli_agent: Some(value.cli_agent.try_into()?),
            computer_use: Some(value.computer_use_agent.try_into()?),
        })
    }
}

impl TryFrom<warp_graphql::workspace::FeatureModelChoice> for ModelsByFeature {
    type Error = anyhow::Error;

    fn try_from(value: warp_graphql::workspace::FeatureModelChoice) -> Result<Self, Self::Error> {
        Ok(Self {
            agent_mode: value.agent_mode.try_into()?,
            coding: value.coding.try_into()?,
            cli_agent: Some(value.cli_agent.try_into()?),
            computer_use: Some(value.computer_use_agent.try_into()?),
        })
    }
}

impl TryFrom<warp_graphql::queries::get_feature_model_choices::AvailableLlms> for AvailableLLMs {
    type Error = anyhow::Error;

    fn try_from(
        value: warp_graphql::queries::get_feature_model_choices::AvailableLlms,
    ) -> Result<Self, Self::Error> {
        Self::new(
            value.default_id.into(),
            value.choices.into_iter().map(LLMInfo::from),
            value.preferred_codex_model_id.map(Into::into),
        )
    }
}

impl TryFrom<warp_graphql::workspace::AvailableLlms> for AvailableLLMs {
    type Error = anyhow::Error;

    fn try_from(value: warp_graphql::workspace::AvailableLlms) -> Result<Self, Self::Error> {
        Self::new(
            value.default_id.into(),
            value.choices.into_iter().map(LLMInfo::from),
            value.preferred_codex_model_id.map(Into::into),
        )
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::LlmInfo> for LLMInfo {
    fn from(value: warp_graphql::queries::get_feature_model_choices::LlmInfo) -> Self {
        let host_configs = {
            let mut map = std::collections::HashMap::new();
            for config in value.host_configs {
                let config: RoutingHostConfig = config.into();
                let host = config.model_routing_host.clone();
                if map.insert(host.clone(), config).is_some() {
                    log::warn!(
                        "Duplicate LlmModelHost entry for {:?}, using latest value",
                        host
                    );
                }
            }
            map
        };
        Self {
            id: value.id.into(),
            display_name: value.display_name,
            base_model_name: value.base_model_name,
            reasoning_level: value.reasoning_level,
            usage_metadata: value.usage_metadata.into(),
            description: value.description,
            disable_reason: value.disable_reason.map(DisableReason::from),
            vision_supported: value.vision_supported,
            spec: value.spec.map(Into::into),
            provider: value.provider.into(),
            host_configs,
            discount_percentage: value.pricing.discount_percentage.map(|v| v as f32),
        }
    }
}

impl From<warp_graphql::workspace::LlmInfo> for LLMInfo {
    fn from(value: warp_graphql::workspace::LlmInfo) -> Self {
        let host_configs = {
            let mut map = std::collections::HashMap::new();
            for config in value.host_configs {
                let config: RoutingHostConfig = config.into();
                let host = config.model_routing_host.clone();
                if map.insert(host.clone(), config).is_some() {
                    log::warn!(
                        "Duplicate LlmModelHost entry for {:?}, using latest value",
                        host
                    );
                }
            }
            map
        };
        Self {
            id: value.id.into(),
            display_name: value.display_name,
            base_model_name: value.base_model_name,
            reasoning_level: value.reasoning_level,
            usage_metadata: value.usage_metadata.into(),
            description: value.description,
            disable_reason: value.disable_reason.map(DisableReason::from),
            vision_supported: value.vision_supported,
            spec: value.spec.map(Into::into),
            provider: value.provider.into(),
            host_configs,
            discount_percentage: value.pricing.discount_percentage.map(|v| v as f32),
        }
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::RoutingHostConfig>
    for RoutingHostConfig
{
    fn from(value: warp_graphql::queries::get_feature_model_choices::RoutingHostConfig) -> Self {
        Self {
            enabled: value.enabled,
            model_routing_host: value.model_routing_host.into(),
        }
    }
}

impl From<warp_graphql::workspace::RoutingHostConfig> for RoutingHostConfig {
    fn from(value: warp_graphql::workspace::RoutingHostConfig) -> Self {
        Self {
            enabled: value.enabled,
            model_routing_host: value.model_routing_host.into(),
        }
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::LlmModelHost> for LLMModelHost {
    fn from(value: warp_graphql::queries::get_feature_model_choices::LlmModelHost) -> Self {
        match value {
            warp_graphql::queries::get_feature_model_choices::LlmModelHost::DirectApi => {
                LLMModelHost::DirectApi
            }
            warp_graphql::queries::get_feature_model_choices::LlmModelHost::AwsBedrock => {
                LLMModelHost::AwsBedrock
            }
            warp_graphql::queries::get_feature_model_choices::LlmModelHost::Other(value) => {
                report_error!(anyhow!(
                    "Unknown LlmModelHost '{value}'. Make sure to update client GraphQL types!"
                ));
                LLMModelHost::Unknown
            }
        }
    }
}

impl From<warp_graphql::workspace::LlmModelHost> for LLMModelHost {
    fn from(value: warp_graphql::workspace::LlmModelHost) -> Self {
        match value {
            warp_graphql::workspace::LlmModelHost::DirectApi => LLMModelHost::DirectApi,
            warp_graphql::workspace::LlmModelHost::AwsBedrock => LLMModelHost::AwsBedrock,
            warp_graphql::workspace::LlmModelHost::Other(value) => {
                report_error!(anyhow!(
                    "Unknown LlmModelHost '{value}'. Make sure to update client GraphQL types!"
                ));
                LLMModelHost::Unknown
            }
        }
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::LlmProvider> for LLMProvider {
    fn from(value: warp_graphql::queries::get_feature_model_choices::LlmProvider) -> Self {
        match value {
            warp_graphql::queries::get_feature_model_choices::LlmProvider::Openai => {
                LLMProvider::OpenAI
            }
            warp_graphql::queries::get_feature_model_choices::LlmProvider::Anthropic => {
                LLMProvider::Anthropic
            }
            warp_graphql::queries::get_feature_model_choices::LlmProvider::Google => {
                LLMProvider::Google
            }
            warp_graphql::queries::get_feature_model_choices::LlmProvider::Xai => LLMProvider::Xai,
            warp_graphql::queries::get_feature_model_choices::LlmProvider::Unknown => {
                LLMProvider::Unknown
            }
            warp_graphql::queries::get_feature_model_choices::LlmProvider::Other(value)
                if value == "OPENROUTER" || value == "OPEN_ROUTER" =>
            {
                LLMProvider::OpenRouter
            }
            warp_graphql::queries::get_feature_model_choices::LlmProvider::Other(value) => {
                report_error!(anyhow!(
                    "Invalid LlmProvider '{value}'. Make sure to update client GraphQL types!"
                ));
                LLMProvider::Unknown
            }
        }
    }
}

impl From<warp_graphql::workspace::LlmProvider> for LLMProvider {
    fn from(value: warp_graphql::workspace::LlmProvider) -> Self {
        match value {
            warp_graphql::workspace::LlmProvider::Openai => LLMProvider::OpenAI,
            warp_graphql::workspace::LlmProvider::Anthropic => LLMProvider::Anthropic,
            warp_graphql::workspace::LlmProvider::Google => LLMProvider::Google,
            warp_graphql::workspace::LlmProvider::Xai => LLMProvider::Xai,
            warp_graphql::workspace::LlmProvider::Unknown => LLMProvider::Unknown,
            warp_graphql::workspace::LlmProvider::Other(value)
                if value == "OPENROUTER" || value == "OPEN_ROUTER" =>
            {
                LLMProvider::OpenRouter
            }
            warp_graphql::workspace::LlmProvider::Other(value) => {
                report_error!(anyhow!(
                    "Invalid LlmProvider '{value}'. Make sure to update client GraphQL types!"
                ));
                LLMProvider::Unknown
            }
        }
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::LlmSpec> for LLMSpec {
    fn from(value: warp_graphql::queries::get_feature_model_choices::LlmSpec) -> Self {
        Self {
            cost: value.cost as f32,
            quality: value.quality as f32,
            speed: value.speed as f32,
        }
    }
}

impl From<warp_graphql::workspace::LlmSpec> for LLMSpec {
    fn from(value: warp_graphql::workspace::LlmSpec) -> Self {
        Self {
            cost: value.cost as f32,
            quality: value.quality as f32,
            speed: value.speed as f32,
        }
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::LlmUsageMetadata> for LLMUsageMetadata {
    fn from(value: warp_graphql::queries::get_feature_model_choices::LlmUsageMetadata) -> Self {
        Self {
            request_multiplier: value.request_multiplier.max(1) as usize,
            credit_multiplier: value.credit_multiplier.map(|v| v as f32),
        }
    }
}

impl From<warp_graphql::workspace::LlmUsageMetadata> for LLMUsageMetadata {
    fn from(value: warp_graphql::workspace::LlmUsageMetadata) -> Self {
        Self {
            request_multiplier: value.request_multiplier.max(1) as usize,
            credit_multiplier: value.credit_multiplier.map(|v| v as f32),
        }
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::DisableReason> for DisableReason {
    fn from(value: warp_graphql::queries::get_feature_model_choices::DisableReason) -> Self {
        match value {
            warp_graphql::queries::get_feature_model_choices::DisableReason::AdminDisabled => {
                DisableReason::AdminDisabled
            }
            warp_graphql::queries::get_feature_model_choices::DisableReason::OutOfRequests => {
                DisableReason::OutOfRequests
            }
            warp_graphql::queries::get_feature_model_choices::DisableReason::ProviderOutage => {
                DisableReason::ProviderOutage
            }
            warp_graphql::queries::get_feature_model_choices::DisableReason::RequiresUpgrade => {
                DisableReason::RequiresUpgrade
            }
            warp_graphql::queries::get_feature_model_choices::DisableReason::Other(_) => {
                DisableReason::Unavailable
            }
        }
    }
}

impl From<warp_graphql::workspace::DisableReason> for DisableReason {
    fn from(value: warp_graphql::workspace::DisableReason) -> Self {
        match value {
            warp_graphql::workspace::DisableReason::AdminDisabled => DisableReason::AdminDisabled,
            warp_graphql::workspace::DisableReason::OutOfRequests => DisableReason::OutOfRequests,
            warp_graphql::workspace::DisableReason::ProviderOutage => DisableReason::ProviderOutage,
            warp_graphql::workspace::DisableReason::RequiresUpgrade => {
                DisableReason::RequiresUpgrade
            }
            warp_graphql::workspace::DisableReason::Other(_) => DisableReason::Unavailable,
        }
    }
}

// Conversions for AIConversationMetadata from GraphQL types

fn convert_harness(harness: warp_graphql::ai::AgentHarness) -> AIAgentHarness {
    match harness {
        warp_graphql::ai::AgentHarness::Oz => AIAgentHarness::Oz,
        warp_graphql::ai::AgentHarness::ClaudeCode => AIAgentHarness::ClaudeCode,
        warp_graphql::ai::AgentHarness::Gemini => AIAgentHarness::Gemini,
        warp_graphql::ai::AgentHarness::Other(value) => {
            report_error!(anyhow!(
                "Invalid AgentHarness '{value}'. Make sure to update client GraphQL types!"
            ));
            AIAgentHarness::Unknown
        }
    }
}

fn convert_block_snapshot_format(
    format: warp_graphql::ai::SerializedBlockFormat,
) -> AIAgentSerializedBlockFormat {
    match format {
        warp_graphql::ai::SerializedBlockFormat::JsonV1 => AIAgentSerializedBlockFormat::JsonV1,
    }
}

fn convert_conversation_format(
    format: warp_graphql::ai::AIConversationFormat,
) -> AIAgentConversationFormat {
    AIAgentConversationFormat {
        has_task_list: format.has_task_list,
        block_snapshot: format.block_snapshot.map(convert_block_snapshot_format),
    }
}

// Helper function
fn convert_usage_metadata(
    summarized: bool,
    context_window_usage: f64,
    credits_spent: f64,
) -> ConversationUsageMetadata {
    ConversationUsageMetadata {
        was_summarized: summarized,
        context_window_usage: context_window_usage as f32,
        credits_spent: credits_spent as f32,
        credits_spent_for_last_block: None,
        token_usage: vec![],
        tool_usage_metadata: Default::default(),
    }
}

impl TryFrom<warp_graphql::ai::AIConversation> for ServerAIConversationMetadata {
    type Error = anyhow::Error;

    fn try_from(value: warp_graphql::ai::AIConversation) -> Result<Self, Self::Error> {
        let usage = convert_usage_metadata(
            value.usage.usage_metadata.summarized,
            value.usage.usage_metadata.context_window_usage,
            value.usage.usage_metadata.credits_spent,
        );
        let metadata = ServerConversationObjectMetadata {
            uid: ServerId::from_string_lossy(value.metadata.uid.into_inner()),
            creator_uid: value.metadata.creator_uid.map(|id| id.into_inner()),
            metadata_last_updated_ts: value.metadata.metadata_last_updated_ts,
        };
        let permissions = ServerConversationPermissions;
        let ambient_agent_task_id = value
            .ambient_agent_task_id
            .map(|id| id.into_inner().parse())
            .transpose()?;
        let server_conversation_token =
            ServerConversationToken::new(value.conversation_id.into_inner());

        // If we fail to parse any artifacts, don't fail the entire conversion -- just don't include them in the list
        let artifacts = value
            .artifacts
            .unwrap_or_default()
            .into_iter()
            .filter_map(|a| Artifact::try_from(a).ok())
            .collect();

        Ok(Self {
            title: value.title,
            working_directory: value.working_directory,
            harness: convert_harness(value.harness),
            usage,
            metadata,
            permissions,
            ambient_agent_task_id,
            server_conversation_token,
            artifacts,
        })
    }
}

impl TryFrom<warp_graphql::queries::list_ai_conversations::AIConversationMetadata>
    for ServerAIConversationMetadata
{
    type Error = anyhow::Error;

    fn try_from(
        value: warp_graphql::queries::list_ai_conversations::AIConversationMetadata,
    ) -> Result<Self, Self::Error> {
        let usage = convert_usage_metadata(
            value.usage.usage_metadata.summarized,
            value.usage.usage_metadata.context_window_usage,
            value.usage.usage_metadata.credits_spent,
        );
        let metadata = ServerConversationObjectMetadata {
            uid: ServerId::from_string_lossy(value.metadata.uid.into_inner()),
            creator_uid: value.metadata.creator_uid.map(|id| id.into_inner()),
            metadata_last_updated_ts: value.metadata.metadata_last_updated_ts,
        };
        let permissions = ServerConversationPermissions;
        let ambient_agent_task_id = value
            .ambient_agent_task_id
            .map(|id| id.into_inner().parse())
            .transpose()?;
        let server_conversation_token =
            ServerConversationToken::new(value.conversation_id.into_inner());

        let artifacts = value
            .artifacts
            .unwrap_or_default()
            .into_iter()
            .filter_map(|a| Artifact::try_from(a).ok())
            .collect();

        Ok(Self {
            title: value.title,
            working_directory: value.working_directory,
            harness: convert_harness(value.harness),
            usage,
            metadata,
            permissions,
            ambient_agent_task_id,
            server_conversation_token,
            artifacts,
        })
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl StoreClient for ServerApi {
    async fn update_intermediate_nodes(
        &self,
        embedding_config: EmbeddingConfig,
        nodes: Vec<IntermediateNode>,
    ) -> Result<HashMap<NodeHash, bool>, full_source_code_embedding::Error> {
        let results = self.update_merkle_tree(embedding_config, nodes).await?;
        Ok(results)
    }

    async fn generate_embeddings(
        &self,
        embedding_config: EmbeddingConfig,
        fragments: Vec<full_source_code_embedding::Fragment>,
        root_hash: NodeHash,
        repo_metadata: RepoMetadata,
    ) -> Result<HashMap<ContentHash, bool>, full_source_code_embedding::Error> {
        let results = self
            .generate_code_embeddings(embedding_config, fragments, root_hash, repo_metadata)
            .await?;
        Ok(results)
    }

    async fn populate_merkle_tree_cache(
        &self,
        embedding_config: EmbeddingConfig,
        root_hash: NodeHash,
        repo_metadata: RepoMetadata,
    ) -> Result<bool, full_source_code_embedding::Error> {
        let variables = PopulateMerkleTreeCacheVariables {
            embedding_config: embedding_config.into(),
            root_hash: root_hash.into(),
            repo_metadata: repo_metadata.into(),
            request_context: get_request_context(),
        };
        let operation = PopulateMerkleTreeCache::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.populate_merkle_tree_cache {
            PopulateMerkleTreeCacheResult::PopulateMerkleTreeCacheOutput(output) => {
                Ok(output.success)
            }
            PopulateMerkleTreeCacheResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)).into())
            }
            PopulateMerkleTreeCacheResult::Unknown => {
                Err(anyhow!("failed to populate merkle tree cache").into())
            }
        }
    }

    async fn sync_merkle_tree(
        &self,
        nodes: Vec<NodeHash>,
        embedding_config: EmbeddingConfig,
    ) -> Result<HashSet<NodeHash>, full_source_code_embedding::Error> {
        let input = SyncMerkleTreeInput {
            hashed_nodes: nodes.into_iter().map(Into::into).collect(),
            embedding_config: embedding_config.into(),
        };

        let variables = SyncMerkleTreeVariables {
            input,
            request_context: get_request_context(),
        };

        let operation = SyncMerkleTree::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.sync_merkle_tree {
            SyncMerkleTreeResult::SyncMerkleTreeOutput(output) => {
                let mut node_results = HashSet::with_capacity(output.changed_nodes.len());
                for hash in output.changed_nodes {
                    node_results.insert(hash.try_into()?);
                }
                Ok(node_results)
            }
            SyncMerkleTreeResult::SyncMerkleTreeError(e) => Err(anyhow!(e.error).into()),
            SyncMerkleTreeResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)).into())
            }
            SyncMerkleTreeResult::Unknown => Err(anyhow!("failed to sync merkle tree").into()),
        }
    }

    async fn rerank_fragments(
        &self,
        query: String,
        fragments: Vec<full_source_code_embedding::Fragment>,
    ) -> Result<Vec<full_source_code_embedding::Fragment>, full_source_code_embedding::Error> {
        let variables = RerankFragmentsVariables {
            query,
            fragments: fragments.into_iter().map(Into::into).collect(),
            request_context: get_request_context(),
        };
        let operation = RerankFragments::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.rerank_fragments {
            RerankFragmentsResult::RerankFragmentsOutput(output) => Ok(output
                .ranked_fragments
                .into_iter()
                .map(|fragment| fragment.try_into())
                .collect::<Result<Vec<_>, _>>()?),
            RerankFragmentsResult::RerankFragmentsError(e) => Err(anyhow!(e.error).into()),
            RerankFragmentsResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)).into())
            }
            RerankFragmentsResult::Unknown => Err(anyhow!("failed to rerank fragments").into()),
        }
    }

    async fn get_relevant_fragments(
        &self,
        embedding_config: EmbeddingConfig,
        query: String,
        root_hash: NodeHash,
        repo_metadata: RepoMetadata,
    ) -> Result<Vec<ContentHash>, full_source_code_embedding::Error> {
        let variables = GetRelevantFragmentsVariables {
            query,
            root_hash: root_hash.into(),
            embedding_config: embedding_config.into(),
            request_context: get_request_context(),
            repo_metadata: repo_metadata.into(),
        };
        let operation = GetRelevantFragmentsQuery::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.get_relevant_fragments {
            GetRelevantFragmentsResult::GetRelevantFragmentsOutput(output) => Ok(output
                .candidate_hashes
                .into_iter()
                .map(|hash| hash.try_into())
                .collect::<Result<Vec<_>, _>>()?),
            GetRelevantFragmentsResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)).into())
            }
            GetRelevantFragmentsResult::GetRelevantFragmentsError(e) => {
                Err(anyhow!(e.error).into())
            }
            GetRelevantFragmentsResult::Unknown => {
                Err(anyhow!("failed to get relevant fragments").into())
            }
        }
    }

    async fn codebase_context_config(
        &self,
    ) -> Result<CodebaseContextConfig, full_source_code_embedding::Error> {
        let variables = CodebaseContextConfigVariables {
            request_context: get_request_context(),
        };
        let operation = CodebaseContextConfigQuery::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.codebase_context_config {
            CodebaseContextConfigResult::CodebaseContextConfigOutput(output) => {
                Ok(CodebaseContextConfig {
                    embedding_config: output.embedding_config.try_into()?,
                    embedding_cadence: Duration::from_secs(output.embedding_cadence as u64),
                })
            }
            CodebaseContextConfigResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)).into())
            }
            CodebaseContextConfigResult::Unknown => {
                Err(anyhow!("failed to retrieve codebase context config").into())
            }
        }
    }
}

#[cfg(test)]
#[path = "ai_test.rs"]
mod tests;
