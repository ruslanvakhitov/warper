use crate::server::server_api::ai::AIClient;
use crate::settings::AISettings;
use crate::workspaces::workspace::WorkspaceUid;
use chrono::{DateTime, Utc};
use instant::Instant;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp_graphql::scalars::time::ServerTimestamp;
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

pub use warp_graphql::billing::BonusGrantType;

/// Threshold of ambient-only credits at which we surface upgrade/CTA UI.
pub const AMBIENT_AGENT_TRIAL_CREDIT_THRESHOLD: i32 = 20;

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

/// The current rate limit info for the user.
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

fn default_voice_requests_limit() -> usize {
    10000
}

impl Default for RequestLimitInfo {
    /// This is the default rate limit for the free tier imposed by the server as of 02/10/25.
    fn default() -> Self {
        Self {
            limit: 150,
            num_requests_used_since_refresh: 0,
            next_refresh_time: ServerTimestamp::new(Utc::now() + chrono::Duration::days(30)),
            is_unlimited: false,
            request_limit_refresh_duration: RequestLimitRefreshDuration::Monthly,
            is_unlimited_voice: false,
            voice_request_limit: default_voice_requests_limit(),
            voice_requests_used_since_last_refresh: 0,
            is_unlimited_codebase_indices: false,
            max_codebase_indices: 3,
            max_files_per_repo: 5000,
            embedding_generation_batch_size: 100,
        }
    }
}

#[cfg(test)]
impl RequestLimitInfo {
    pub fn new_for_test(limit: usize, num_requests_used_since_refresh: usize) -> Self {
        Self {
            limit,
            num_requests_used_since_refresh,
            ..Self::default()
        }
    }
}

pub struct CodebaseContextUsageLimit {
    pub max_files_per_repo: usize,
    pub max_indices_allowed: Option<usize>,
    pub embedding_generation_batch_size: usize,
}

/// Contains all usage-related information fetched from the server.
pub struct RequestUsageInfo {
    pub request_limit_info: RequestLimitInfo,
    pub bonus_grants: Vec<BonusGrant>,
}

#[cfg(feature = "agent_mode_evals")]
impl RequestLimitInfo {
    pub fn new_for_evals() -> Self {
        Self {
            limit: 999999,
            num_requests_used_since_refresh: 0,
            next_refresh_time: ServerTimestamp::new(Utc::now() + chrono::Duration::days(30)),
            is_unlimited: true,
            request_limit_refresh_duration: RequestLimitRefreshDuration::Monthly,
            is_unlimited_voice: true,
            voice_request_limit: 999999,
            voice_requests_used_since_last_refresh: 0,
            is_unlimited_codebase_indices: false,
            max_codebase_indices: 40,
            max_files_per_repo: 10000,
            embedding_generation_batch_size: 100,
        }
    }
}

pub struct AIRequestUsageModel {
    /// The last time at which `request_limit_info` was updated.
    last_update_time: Option<Instant>,

    request_limit_info: RequestLimitInfo,

    bonus_grants: Vec<BonusGrant>,
}

impl Entity for AIRequestUsageModel {
    type Event = AIRequestUsageModelEvent;
}

pub enum AIRequestUsageModelEvent {
    RequestUsageUpdated,
    RequestBonusRefunded {
        requests_refunded: i32,
        server_conversation_id: String,
        request_id: String,
    },
}

impl AIRequestUsageModel {
    pub fn new(_ai_client: Arc<dyn AIClient>, _ctx: &mut ModelContext<Self>) -> Self {
        Self {
            request_limit_info: RequestLimitInfo::default(),
            last_update_time: None,
            bonus_grants: vec![],
        }
    }

    #[cfg(test)]
    pub fn new_for_test(ai_client: Arc<dyn AIClient>, _ctx: &mut ModelContext<Self>) -> Self {
        let _ = ai_client;
        Self {
            last_update_time: None,
            request_limit_info: RequestLimitInfo::default(),
            bonus_grants: vec![],
        }
    }

    pub fn last_update_time(&self) -> Option<Instant> {
        self.last_update_time
    }

    /// Hosted request usage is not fetched in the OSS build.
    pub fn refresh_request_usage_async(&mut self, _ctx: &mut ModelContext<Self>) {
        self.last_update_time = Some(Instant::now());
    }

    pub fn update_request_limit_info(
        &mut self,
        request_limit_info: RequestLimitInfo,
        ctx: &mut ModelContext<Self>,
    ) {
        self.last_update_time = Some(Instant::now());
        self.request_limit_info = request_limit_info;

        AISettings::handle(ctx).update(ctx, |ai_settings, ctx| {
            ai_settings.update_quota_info(&request_limit_info, ctx);
        });

        ctx.emit(AIRequestUsageModelEvent::RequestUsageUpdated);
    }

    pub fn provide_negative_feedback_response_for_ai_conversation(
        &mut self,
        _client_conversation_id: crate::ai::agent::conversation::AIConversationId,
        _request_id: String,
        _client_exchange_id: crate::ai::agent::AIAgentExchangeId,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    /// Returns the number of remaining requests the user has based on their latest rate limit info.
    /// If the current time is past the next refresh time, then the number of remaining reqs is the limit.
    fn requests_remaining(&self) -> usize {
        if self.next_refresh_time() <= Utc::now() || self.is_unlimited() {
            self.request_limit_info.limit
        } else {
            self.request_limit_info
                .limit
                .saturating_sub(self.request_limit_info.num_requests_used_since_refresh)
        }
    }

    /// Returns `true` if the user has at least one request remaining before hitting the AI request
    /// limit.
    ///
    /// WARNING: This method doesn't account for add-on credits. Consider if you want
    /// [`Self::has_any_ai_remaining`] instead.
    pub fn has_requests_remaining(&self) -> bool {
        self.requests_remaining() > 0
    }

    /// Hosted billing quotas are not enforced in the OSS build.
    pub fn has_any_ai_remaining(&self, _ctx: &AppContext) -> bool {
        true
    }

    pub fn requests_used(&self) -> usize {
        if self.next_refresh_time() <= Utc::now() {
            return 0;
        }
        self.request_limit_info.num_requests_used_since_refresh
    }

    pub fn request_percentage_used(&self) -> f32 {
        self.requests_used() as f32 / self.request_limit() as f32
    }

    pub fn request_limit(&self) -> usize {
        self.request_limit_info.limit
    }

    /// Returns the number of indices the user's tier allows them to create and the number of files
    /// the user's tier allows them to index. If the user is allowed unlimited indices, then the
    /// max_indices_allowed is None.
    pub fn codebase_context_limits(&self) -> CodebaseContextUsageLimit {
        CodebaseContextUsageLimit {
            max_files_per_repo: self.request_limit_info.max_files_per_repo,
            max_indices_allowed: if self.request_limit_info.is_unlimited_codebase_indices {
                None
            } else {
                Some(self.request_limit_info.max_codebase_indices)
            },
            embedding_generation_batch_size: self
                .request_limit_info
                .embedding_generation_batch_size,
        }
    }

    /// Returns whether the user has hit their maximum codebase allowance.
    /// (If the user is allowed unlimited indices, this is vacuously false.)
    pub fn hit_codebase_index_limit(&self, current_indices: usize) -> bool {
        self.codebase_context_limits()
            .max_indices_allowed
            .map(|lim| current_indices >= lim)
            .unwrap_or(false)
    }

    pub fn next_refresh_time(&self) -> DateTime<Utc> {
        self.request_limit_info.next_refresh_time.utc()
    }

    pub fn is_unlimited(&self) -> bool {
        self.request_limit_info.is_unlimited
    }

    pub fn refresh_duration_to_string(&self) -> String {
        match self.request_limit_info.request_limit_refresh_duration {
            RequestLimitRefreshDuration::Weekly => "weekly".to_string(),
            RequestLimitRefreshDuration::Monthly => "monthly".to_string(),
            RequestLimitRefreshDuration::EveryTwoWeeks => "biweekly".to_string(),
        }
    }

    pub fn bonus_grants(&self) -> &[BonusGrant] {
        &self.bonus_grants
    }

    /// Returns the total remaining ambient-only credits for the user.
    /// Returns None if the user has never received any ambient-only grants.
    pub fn ambient_only_credits_remaining(&self) -> Option<i32> {
        let ambient_grants: Vec<_> = self
            .bonus_grants
            .iter()
            .filter(|g| g.grant_type == BonusGrantType::AmbientOnly)
            .collect();
        if ambient_grants.is_empty() {
            None
        } else {
            Some(
                ambient_grants
                    .iter()
                    .map(|g| g.request_credits_remaining)
                    .sum(),
            )
        }
    }

    pub fn total_workspace_bonus_credits_remaining(&self, uid: WorkspaceUid) -> i32 {
        let now = Utc::now();
        self.bonus_grants
            .iter()
            .filter(|grant| grant.scope == BonusGrantScope::Workspace(uid))
            .filter(|grant| grant.expiration.is_none_or(|exp| now < exp))
            .map(|grant| grant.request_credits_remaining)
            .sum()
    }

    pub fn total_current_workspace_bonus_credits_remaining(&self, ctx: &AppContext) -> i32 {
        let _ = ctx;
        0
    }
}

/// Voice request usage, only available if built with voice input support.
#[cfg(feature = "voice_input")]
impl AIRequestUsageModel {
    fn voice_requests(&self) -> usize {
        self.request_limit_info
            .voice_requests_used_since_last_refresh
    }

    fn voice_requests_limit(&self) -> usize {
        self.request_limit_info.voice_request_limit
    }

    fn is_unlimited_voice_requests(&self) -> bool {
        self.request_limit_info.is_unlimited_voice
    }

    /// Returns the number of remaining requests the user has based on their latest rate limit info.
    /// If the current time is past the next refresh time, then the number of remaining reqs is the limit.
    fn voice_requests_remaining(&self) -> usize {
        if self.next_refresh_time() <= Utc::now() || self.is_unlimited_voice_requests() {
            self.voice_requests_limit()
        } else {
            self.voice_requests_limit()
                .saturating_sub(self.voice_requests())
        }
    }

    /// Returns `true` if the user has at least one voice request before hitting the
    /// limit. Returns `false` otherwise.
    fn has_voice_requests_remaining(&self) -> bool {
        self.voice_requests_remaining() > 0
    }

    /// Checks request limits to see if the user can make a voice request.
    /// Returns true if the user can make a voice request, false otherwise.
    pub fn can_request_voice(&self) -> bool {
        self.has_voice_requests_remaining()
    }
}

impl SingletonEntity for AIRequestUsageModel {}

#[cfg(test)]
#[path = "request_usage_model_test.rs"]
mod tests;
