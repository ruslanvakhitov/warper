use crate::{scalars::Time, Id};

#[derive(Clone, Copy, Debug)]
pub enum RequestLimitRefreshDuration {
    Monthly,
    Weekly,
    EveryTwoWeeks,
}

#[derive(Debug)]
pub struct RequestLimitInfo {
    pub is_unlimited: bool,
    pub next_refresh_time: Time,
    pub request_limit: i32,
    pub requests_used_since_last_refresh: i32,
    pub request_limit_refresh_duration: RequestLimitRefreshDuration,
    pub is_unlimited_voice: bool,
    pub voice_request_limit: i32,
    pub voice_requests_used_since_last_refresh: i32,
    pub is_unlimited_codebase_indices: bool,
    pub max_codebase_indices: i32,
    pub max_files_per_repo: i32,
    pub embedding_generation_batch_size: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BonusGrantType {
    AmbientOnly,
    Any,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AgentTaskState {
    Blocked,
    Cancelled,
    Claimed,
    Error,
    InProgress,
    Succeeded,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlatformErrorCode {
    AuthenticationRequired,
    BudgetExceeded,
    ContentPolicyViolation,
    EnvironmentSetupFailed,
    ExternalAuthenticationRequired,
    FeatureNotAvailable,
    InsufficientCredits,
    IntegrationDisabled,
    IntegrationNotConfigured,
    InternalError,
    InvalidRequest,
    NotAuthorized,
    ResourceUnavailable,
    ResourceNotFound,
}

#[derive(Debug, Clone)]
pub struct PlanArtifact {
    pub document_uid: Id,
    pub notebook_uid: Option<Id>,
    pub title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PullRequestArtifact {
    pub url: String,
    pub branch: String,
}

#[derive(Debug, Clone)]
pub struct ScreenshotArtifact {
    pub artifact_uid: Id,
    pub mime_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileArtifact {
    pub artifact_uid: Id,
    pub filepath: String,
    pub mime_type: String,
    pub description: Option<String>,
    pub size_bytes: Option<i32>,
}

#[derive(Debug, Clone)]
pub enum AIConversationArtifact {
    PlanArtifact(PlanArtifact),
    PullRequestArtifact(PullRequestArtifact),
    ScreenshotArtifact(ScreenshotArtifact),
    FileArtifact(FileArtifact),
    Unknown,
}
