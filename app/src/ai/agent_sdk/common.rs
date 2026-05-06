//! Common utilities for agent SDK commands.

use std::fmt;
use std::future::Future;
use std::time::Duration;

use warpui::{AppContext, SingletonEntity as _};

use crate::ai::llms::{LLMId, LLMPreferences};

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

/// Refresh workspace metadata before executing an operation.
///
/// This ensures that team state is up-to-date before creating cloud objects or performing
/// other operations that depend on team membership.
pub fn refresh_workspace_metadata<C>(
    _ctx: &mut C,
) -> impl Future<Output = anyhow::Result<()>> + Send + 'static {
    async { Ok(()) }
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
