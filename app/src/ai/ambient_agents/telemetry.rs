use crate::server::ids::ServerId;
use serde_json::{json, Value};
use strum_macros::{EnumDiscriminants, EnumIter};
use warp_core::telemetry::{EnablementState, TelemetryEvent, TelemetryEventDesc};

/// Telemetry events for retained local environment setup interactions.
#[derive(Debug, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
pub enum AmbientAgentTelemetryEvent {
    /// User created a new environment.
    EnvironmentCreated,
    /// User updated an existing environment.
    EnvironmentUpdated {
        /// The server ID of the updated environment, if available.
        environment_id: Option<ServerId>,
    },
    /// User deleted an environment.
    EnvironmentDeleted {
        /// The server ID of the deleted environment, if available.
        environment_id: Option<ServerId>,
    },
    /// Docker image was successfully suggested for an environment.
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    ImageSuggested {
        /// The suggested Docker image string.
        image: String,
        /// Whether the user needs to create a custom image.
        needs_custom_image: bool,
    },
    /// Docker image suggestion failed.
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    ImageSuggestionFailed {
        /// Error message describing why the suggestion failed.
        error: String,
    },
    /// User launched an environment setup agent from the environment form.
    LaunchedAgentFromEnvironmentForm,
    /// User started GitHub authentication from the environment form.
    GitHubAuthFromEnvironmentForm,
    /// Ambient agent failed to dispatch or encountered an error during subscription.
    DispatchFailed {
        /// Error message describing the failure.
        error: String,
    },
}

impl TelemetryEvent for AmbientAgentTelemetryEvent {
    fn name(&self) -> &'static str {
        AmbientAgentTelemetryEventDiscriminants::from(self).name()
    }

    fn payload(&self) -> Option<Value> {
        match self {
            AmbientAgentTelemetryEvent::EnvironmentCreated => None,
            AmbientAgentTelemetryEvent::EnvironmentUpdated { environment_id } => Some(json!({
                "environment_id": environment_id.map(|id| id.to_string()),
            })),
            AmbientAgentTelemetryEvent::EnvironmentDeleted { environment_id } => Some(json!({
                "environment_id": environment_id.map(|id| id.to_string()),
            })),
            AmbientAgentTelemetryEvent::ImageSuggested {
                image,
                needs_custom_image,
            } => Some(json!({
                "image": image,
                "needs_custom_image": needs_custom_image,
            })),
            AmbientAgentTelemetryEvent::ImageSuggestionFailed { error } => Some(json!({
                "error": error,
            })),
            AmbientAgentTelemetryEvent::LaunchedAgentFromEnvironmentForm => None,
            AmbientAgentTelemetryEvent::GitHubAuthFromEnvironmentForm => None,
            AmbientAgentTelemetryEvent::DispatchFailed { error } => Some(json!({
                "error": error,
            })),
        }
    }

    fn description(&self) -> &'static str {
        AmbientAgentTelemetryEventDiscriminants::from(self).description()
    }

    fn enablement_state(&self) -> EnablementState {
        AmbientAgentTelemetryEventDiscriminants::from(self).enablement_state()
    }

    fn contains_ugc(&self) -> bool {
        false
    }

    fn event_descs() -> impl Iterator<Item = Box<dyn TelemetryEventDesc>> {
        warp_core::telemetry::enum_events::<Self>()
    }
}

impl TelemetryEventDesc for AmbientAgentTelemetryEventDiscriminants {
    fn name(&self) -> &'static str {
        match self {
            Self::EnvironmentCreated => "AmbientAgent.EnvironmentSettings.CreatedEnvironment",
            Self::EnvironmentUpdated => "AmbientAgent.EnvironmentSettings.UpdatedEnvironment",
            Self::EnvironmentDeleted => "AmbientAgent.EnvironmentSettings.DeletedEnvironment",
            Self::ImageSuggested => "AmbientAgent.EnvironmentSettings.Image.Suggested",
            Self::ImageSuggestionFailed => {
                "AmbientAgent.EnvironmentSettings.Image.SuggestionFailed"
            }
            Self::LaunchedAgentFromEnvironmentForm => {
                "AmbientAgent.EnvironmentSettings.LaunchedAgent"
            }
            Self::GitHubAuthFromEnvironmentForm => "AmbientAgent.EnvironmentSettings.GitHubAuth",
            Self::DispatchFailed => "AmbientAgent.DispatchFailed",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::EnvironmentCreated => "User created a new environment",
            Self::EnvironmentUpdated => "User updated an existing environment",
            Self::EnvironmentDeleted => "User deleted an environment",
            Self::ImageSuggested => "Docker image was suggested for an environment",
            Self::ImageSuggestionFailed => "Docker image suggestion failed",
            Self::LaunchedAgentFromEnvironmentForm => {
                "User launched an environment setup agent from the environment form"
            }
            Self::GitHubAuthFromEnvironmentForm => {
                "User started GitHub authentication from the environment form"
            }
            Self::DispatchFailed => "Ambient agent failed to dispatch or encountered an error",
        }
    }

    fn enablement_state(&self) -> EnablementState {
        EnablementState::ChannelSpecific {
            channels: Vec::new(),
        }
    }
}

warp_core::register_telemetry_event!(AmbientAgentTelemetryEvent);
