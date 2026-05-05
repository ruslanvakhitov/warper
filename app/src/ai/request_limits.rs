use chrono::Utc;
use serde::{Deserialize, Serialize};
use warp_graphql::ai::{
    RequestLimitInfo as RequestLimitInfoGraphql,
    RequestLimitRefreshDuration as RequestLimitRefreshDurationGraphql,
};
use warp_graphql::billing::BonusGrantType;
use warp_graphql::scalars::time::ServerTimestamp;
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

use crate::workspaces::workspace::WorkspaceUid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BonusGrantScope {
    User,
    Workspace(WorkspaceUid),
}

#[derive(Clone, Debug)]
pub struct BonusGrant {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub cost_cents: i32,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
    pub grant_type: BonusGrantType,
    pub reason: String,
    pub user_facing_message: Option<String>,
    pub request_credits_granted: i32,
    pub request_credits_remaining: i32,
    pub scope: BonusGrantScope,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum RequestLimitRefreshDuration {
    Weekly,
    Monthly,
    EveryTwoWeeks,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct RequestLimitInfo {
    pub limit: usize,
    pub num_requests_used_since_refresh: usize,
    pub next_refresh_time: ServerTimestamp,
    pub is_unlimited: bool,
    pub request_limit_refresh_duration: RequestLimitRefreshDuration,
    pub is_unlimited_voice: bool,
    #[serde(default)]
    pub voice_request_limit: usize,
    #[serde(default)]
    pub voice_requests_used_since_last_refresh: usize,
    #[serde(default)]
    pub is_unlimited_codebase_indices: bool,
    #[serde(default)]
    pub max_codebase_indices: usize,
    #[serde(default)]
    pub max_files_per_repo: usize,
    #[serde(default)]
    pub embedding_generation_batch_size: usize,
}

impl Default for RequestLimitInfo {
    fn default() -> Self {
        Self {
            limit: usize::MAX,
            num_requests_used_since_refresh: 0,
            next_refresh_time: ServerTimestamp::new(Utc::now() + chrono::Duration::days(30)),
            is_unlimited: true,
            request_limit_refresh_duration: RequestLimitRefreshDuration::Monthly,
            is_unlimited_voice: true,
            voice_request_limit: usize::MAX,
            voice_requests_used_since_last_refresh: 0,
            is_unlimited_codebase_indices: true,
            max_codebase_indices: usize::MAX,
            max_files_per_repo: usize::MAX,
            embedding_generation_batch_size: 100,
        }
    }
}

impl RequestLimitInfo {
    pub fn new_for_evals() -> Self {
        Self::default()
    }
}

impl From<RequestLimitRefreshDurationGraphql> for RequestLimitRefreshDuration {
    fn from(value: RequestLimitRefreshDurationGraphql) -> Self {
        match value {
            RequestLimitRefreshDurationGraphql::Monthly => RequestLimitRefreshDuration::Monthly,
            RequestLimitRefreshDurationGraphql::Weekly => RequestLimitRefreshDuration::Weekly,
            RequestLimitRefreshDurationGraphql::EveryTwoWeeks => {
                RequestLimitRefreshDuration::EveryTwoWeeks
            }
        }
    }
}

impl From<RequestLimitInfoGraphql> for RequestLimitInfo {
    fn from(value: RequestLimitInfoGraphql) -> Self {
        RequestLimitInfo {
            is_unlimited: value.is_unlimited,
            limit: value.request_limit as usize,
            num_requests_used_since_refresh: value.requests_used_since_last_refresh as usize,
            next_refresh_time: value.next_refresh_time,
            request_limit_refresh_duration: value.request_limit_refresh_duration.into(),
            is_unlimited_voice: value.is_unlimited_voice,
            voice_request_limit: value.voice_request_limit as usize,
            voice_requests_used_since_last_refresh: value.voice_requests_used_since_last_refresh
                as usize,
            is_unlimited_codebase_indices: value.is_unlimited_codebase_indices,
            max_codebase_indices: value.max_codebase_indices as usize,
            max_files_per_repo: value.max_files_per_repo as usize,
            embedding_generation_batch_size: value.embedding_generation_batch_size as usize,
        }
    }
}

pub struct CodebaseContextUsageLimit {
    pub max_files_per_repo: usize,
    pub max_indices_allowed: Option<usize>,
    pub embedding_generation_batch_size: usize,
}

pub struct RequestUsageInfo {
    pub request_limit_info: RequestLimitInfo,
    pub bonus_grants: Vec<BonusGrant>,
}

pub struct AIRequestUsageModel {
    request_limit_info: RequestLimitInfo,
}

impl AIRequestUsageModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            request_limit_info: RequestLimitInfo::default(),
        }
    }

    pub fn refresh_request_usage_async(&mut self, _ctx: &mut ModelContext<Self>) {}

    pub fn has_any_ai_remaining(&self, _app: &AppContext) -> bool {
        true
    }

    pub fn has_voice_remaining(&self, _app: &AppContext) -> bool {
        true
    }

    pub fn can_request_voice(&self) -> bool {
        true
    }

    pub fn request_limit_info(&self) -> RequestLimitInfo {
        self.request_limit_info
    }

    pub fn next_refresh_time(&self) -> chrono::DateTime<Utc> {
        self.request_limit_info.next_refresh_time.utc()
    }

    pub fn codebase_context_usage_limit(&self) -> CodebaseContextUsageLimit {
        CodebaseContextUsageLimit {
            max_files_per_repo: usize::MAX,
            max_indices_allowed: None,
            embedding_generation_batch_size: 100,
        }
    }

    pub fn codebase_context_limits(&self) -> CodebaseContextUsageLimit {
        self.codebase_context_usage_limit()
    }

    pub fn hit_codebase_index_limit(&self, _total: usize) -> bool {
        false
    }
}

impl Entity for AIRequestUsageModel {
    type Event = ();
}

impl SingletonEntity for AIRequestUsageModel {}
