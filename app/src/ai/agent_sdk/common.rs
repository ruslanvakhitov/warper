//! Common utilities for agent SDK commands.

use std::fmt;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use warp_cli::agent::Harness;
use warpui::{AppContext, SingletonEntity as _};

use crate::ai::agent::conversation::AmbientAgentTaskId;
use crate::ai::agent::conversation::ServerAIConversationMetadata;
use crate::ai::agent_sdk::driver::AgentDriverError;
use crate::ai::llms::{LLMId, LLMPreferences};
use crate::server::server_api::ai::AIClient;
use crate::workspaces::user_workspaces::Owner;

/// How long to wait for workspace metadata to refresh.
pub const WORKSPACE_METADATA_REFRESH_TIMEOUT: Duration = Duration::from_secs(10);

pub fn validate_agent_mode_base_model_id(
    model_id: &str,
    ctx: &AppContext,
) -> anyhow::Result<LLMId> {
    let llm_prefs = LLMPreferences::as_ref(ctx);

    let llm_id: LLMId = model_id.into();
    let valid_ids = llm_prefs
        .get_base_llm_choices_for_agent_mode()
        .map(|info| info.id.clone())
        .collect::<Vec<_>>();

    if valid_ids.contains(&llm_id) {
        Ok(llm_id)
    } else {
        let suggestions = valid_ids
            .into_iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Err(anyhow::anyhow!(
            "Unknown model id '{model_id}'. Try one of: {suggestions}"
        ))
    }
}

pub(super) fn parse_ambient_task_id(
    run_id: &str,
    error_prefix: &str,
) -> anyhow::Result<AmbientAgentTaskId> {
    run_id
        .parse()
        .map_err(|err| anyhow::anyhow!("{error_prefix} '{run_id}': {err}"))
}

pub(super) fn set_ambient_task_context_from_run_id(
    _ctx: &AppContext,
    run_id: &str,
) -> anyhow::Result<AmbientAgentTaskId> {
    let task_id = parse_ambient_task_id(run_id, "Invalid run ID")?;
    let _ = task_id;
    anyhow::bail!("Hosted ambient-agent task context is unavailable in local-only Warper")
}

/// Resolve the owner of a new cloud object. This resolution is based on the CLI `--team` and `--personal` flags.
///
/// If `team_flag` is true, attempts to get the current team UID (errors if not on a team).
/// If `user_flag` is true, gets the current user's UID.
/// Otherwise, defaults to team if available, falling back to user.
pub fn resolve_owner(
    _team_flag: bool,
    _user_flag: bool,
    _ctx: &AppContext,
) -> anyhow::Result<Owner> {
    Err(anyhow::anyhow!(
        "Hosted cloud objects are unavailable in local-only Warper"
    ))
}

/// Refresh workspace metadata before executing an operation.
///
/// This ensures that team state is up-to-date before creating cloud objects or performing
/// other operations that depend on team membership.
pub fn refresh_workspace_metadata<C>(
    _ctx: &mut C,
) -> impl Future<Output = anyhow::Result<()>> + Send + 'static {
    async { Ok(()) }
}

/// Retained compatibility hook for callers that previously waited on hosted sync.
pub fn refresh_warp_drive(
    _ctx: &AppContext,
) -> impl Future<Output = anyhow::Result<()>> + Send + 'static {
    async { Ok(()) }
}

/// Fetch the conversation's server metadata and validate that its harness matches the caller's
/// local runner choice. Returns the metadata on success so the caller can reuse it (e.g. for the
/// server conversation token).
///
/// Called up-front before any task/config-build logic consumes `args.harness`, so a mismatch
/// error surfaces before side effects like task creation. We deliberately do NOT auto-upgrade
/// the harness: `Harness::Oz` default with a Claude conversation id is treated as a mismatch
/// and errors out.
pub(super) async fn fetch_and_validate_conversation_harness(
    _ai_client: Arc<dyn AIClient>,
    conversation_id: &str,
    _args_harness: Harness,
) -> Result<ServerAIConversationMetadata, AgentDriverError> {
    Err(AgentDriverError::ConversationLoadFailed(format!(
        "conversation {conversation_id} is unavailable because hosted agent metadata is amputated in local-only Warper"
    )))
}

/// Format an object owner for display in the CLI.
pub fn format_owner(owner: &Owner) -> &'static str {
    // TODO: For potentially-shared objects, consider looking up the particular user/team name.
    match owner {
        Owner::User { .. } => "Personal",
        Owner::Team { .. } => "Team",
    }
}

/// An error resolving an agent option, which we may have prompted the user for.
#[derive(Debug, thiserror::Error)]
pub enum ResolveConfigurationError {
    /// The user canceled the operation, and we should exit.
    #[error("Operation canceled")]
    Canceled,
    #[error("{id} is not a valid {kind} identifier")]
    InvalidId { id: String, kind: &'static str },
    #[error("{kind} {id} not found")]
    ObjectNotFound { id: String, kind: &'static str },
    #[error(transparent)]
    Other(anyhow::Error),
}

#[derive(Clone, Debug, PartialEq)]
pub enum EnvironmentChoice {
    /// The user explicitly chose not to use an environment.
    None,
    /// The user chose a specific environment.
    Environment { id: String, name: String },
}

impl EnvironmentChoice {
    /// Resolve the environment to use when creating an agent integration.
    /// Hosted environments are unavailable in local-only Warper.
    pub fn resolve_for_create<T>(
        _args: T,
        _ctx: &AppContext,
    ) -> Result<Self, ResolveConfigurationError> {
        Ok(EnvironmentChoice::None)
    }

    /// Resolve the environment to use when updating an agent integration. If the user did not
    /// request any changes to the environment, this returns `Ok(None)`.
    /// Hosted environments are unavailable in local-only Warper.
    pub fn resolve_for_update<T>(
        _args: T,
        _ctx: &AppContext,
    ) -> Result<Option<Self>, ResolveConfigurationError> {
        Ok(None)
    }

    fn get_by_id(_id: String, _ctx: &AppContext) -> Result<Self, ResolveConfigurationError> {
        Err(ResolveConfigurationError::Other(anyhow::anyhow!(
            "Hosted cloud environments are unavailable in local-only Warper"
        )))
    }
}

impl fmt::Display for EnvironmentChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvironmentChoice::None => write!(
                f,
                "No environment (agent will not be able to access private repositories or create pull requests)",
            ),
            EnvironmentChoice::Environment { id, name } => write!(f, "{name} ({id})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_ambient_task_id;

    #[test]
    fn parse_ambient_task_id_accepts_valid_ids() {
        let task_id =
            parse_ambient_task_id("550e8400-e29b-41d4-a716-446655440000", "Invalid run ID")
                .unwrap();

        assert_eq!(task_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn parse_ambient_task_id_preserves_error_prefix() {
        let err = parse_ambient_task_id("not-a-run-id", "Invalid run ID").unwrap_err();

        assert!(err.to_string().contains("Invalid run ID 'not-a-run-id'"));
    }
}
