use super::{
    team::Team,
    workspace::{
        AdminEnablementSetting, EnterpriseSecretRegex, HostEnablementSetting,
        UgcCollectionEnablementSetting, Workspace, WorkspaceUid,
    },
};
use crate::{
    ai::llms::LLMModelHost,
    channel::ChannelState,
    settings::{
        AISettings, AISettingsChangedEvent, CodeSettings, CodeSettingsChangedEvent, PrivacySettings,
    },
    workspaces::workspace::{AiAutonomySettings, SandboxedAgentSettings},
};
use regex::Regex;
use warp_core::{
    features::FeatureFlag,
    settings::{ChangeEventReason, Setting},
};
use warpui::{AppContext, Entity, ModelContext, SingletonEntity, Tracked};

#[derive(Debug)]
pub enum UserWorkspacesEvent {
    UpdateWorkspaceSettingsSuccess,
    /// Fired whenever the set of teams the user is on changes.
    TeamsChanged,
    CodebaseContextEnablementChanged,
}

/// UserWorkspaces is a singleton model that holds workspace metadata (name, members, etc).
/// It should be used for getting information about the workspaces, teams, current teams,
/// and all other things related to local workspace capability data.
pub struct UserWorkspaces {
    current_workspace_uid: Tracked<Option<WorkspaceUid>>,
    workspaces: Tracked<Vec<Workspace>>,
}

impl UserWorkspaces {
    pub fn local_only(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            current_workspace_uid: None.into(),
            workspaces: Vec::new().into(),
        }
    }

    #[cfg(test)]
    pub fn mock(cached_workspaces: Vec<Workspace>, _ctx: &mut ModelContext<Self>) -> Self {
        Self {
            current_workspace_uid: cached_workspaces.first().map(|w| w.uid).into(),
            workspaces: cached_workspaces.into(),
        }
    }

    #[cfg(test)]
    pub fn default_mock(ctx: &mut ModelContext<Self>) -> Self {
        Self::mock(vec![], ctx)
    }

    pub fn new(
        _cached_workspaces: Vec<Workspace>,
        _current_workspace_uid: Option<WorkspaceUid>,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        ctx.subscribe_to_model(&CodeSettings::handle(ctx), |_, code_settings_event, ctx| {
            match code_settings_event {
                CodeSettingsChangedEvent::CodebaseContextEnabled { .. }
                | CodeSettingsChangedEvent::AutoIndexingEnabled { .. } => {
                    ctx.emit(UserWorkspacesEvent::CodebaseContextEnablementChanged);
                }
                _ => {}
            }
        });

        ctx.subscribe_to_model(&AISettings::handle(ctx), |_, ai_settings_event, ctx| {
            if let AISettingsChangedEvent::IsAnyAIEnabled { .. } = ai_settings_event {
                ctx.emit(UserWorkspacesEvent::CodebaseContextEnablementChanged);
            }
        });

        Self {
            current_workspace_uid: None.into(),
            workspaces: Vec::new().into(),
        }
    }

    pub fn team_from_uid(&self, _team_uid: crate::server::ids::ServerId) -> Option<&Team> {
        None
    }

    pub fn team_from_uid_across_all_workspaces(
        &self,
        _team_uid: crate::server::ids::ServerId,
    ) -> Option<&Team> {
        None
    }

    pub fn workspace_from_uid(&self, workspace_uid: WorkspaceUid) -> Option<&Workspace> {
        self.workspaces.iter().find(|w| w.uid == workspace_uid)
    }

    pub fn workspace_from_uid_mut(
        &mut self,
        workspace_uid: WorkspaceUid,
    ) -> Option<&mut Workspace> {
        self.workspaces.iter_mut().find(|w| w.uid == workspace_uid)
    }

    /// Return the uid of user's current team (if any) without refreshing.
    pub fn current_team_uid(&self) -> Option<crate::server::ids::ServerId> {
        None
    }

    pub fn current_team_mut(&mut self) -> Option<&mut Team> {
        None
    }

    pub fn current_team(&self) -> Option<&Team> {
        None
    }

    pub fn current_workspace(&self) -> Option<&Workspace> {
        self.current_workspace_uid
            .and_then(|workspace_uid| self.workspace_from_uid(workspace_uid))
    }

    pub fn current_workspace_mut(&mut self) -> Option<&mut Workspace> {
        self.current_workspace_uid
            .and_then(|workspace_uid| self.workspace_from_uid_mut(workspace_uid))
    }

    pub fn workspaces(&self) -> &Vec<Workspace> {
        &self.workspaces
    }

    pub fn set_current_workspace_uid(
        &mut self,
        _workspace_uid: WorkspaceUid,
        ctx: &mut ModelContext<Self>,
    ) {
        *self.current_workspace_uid = None;
        self.notify_and_emit_teams_changed(ctx);
    }

    pub fn is_active_ai_allowed(&self) -> bool {
        true
    }

    pub fn ai_allowed_for_current_team(&self) -> bool {
        true
    }

    pub fn is_prompt_suggestions_toggleable(&self) -> bool {
        true
    }

    pub fn is_code_suggestions_toggleable(&self) -> bool {
        true
    }

    pub fn is_next_command_enabled(&self) -> bool {
        true
    }

    /// If voice input support is not compiled into this build, always returns `false`.
    pub fn is_voice_enabled(&self) -> bool {
        cfg!(feature = "voice_input")
    }

    /// Whether BYO API key is enabled for the current local user.
    /// For non-OSS builds, this is controlled by the `SoloUserByok` feature flag.
    pub fn is_byo_api_key_enabled(&self) -> bool {
        if ChannelState::channel() == warp_core::channel::Channel::Oss {
            return true;
        }

        FeatureFlag::SoloUserByok.is_enabled()
    }

    pub fn aws_bedrock_host_settings(&self) -> Option<&super::workspace::LlmHostSettings> {
        self.current_workspace().and_then(|workspace| {
            workspace
                .settings
                .llm_settings
                .host_configs
                .get(&LLMModelHost::AwsBedrock)
        })
    }

    /// Did the admin enable AWS Bedrock for the current workspace?
    pub fn is_aws_bedrock_available_from_workspace(&self) -> bool {
        self.current_workspace().is_some_and(|workspace| {
            workspace.settings.llm_settings.enabled
                && self
                    .aws_bedrock_host_settings()
                    .is_some_and(|settings| settings.enabled)
        })
    }
    pub fn aws_bedrock_host_enablement_setting(&self) -> HostEnablementSetting {
        self.aws_bedrock_host_settings()
            .map(|settings| settings.enablement_setting.clone())
            .unwrap_or_default()
    }

    pub fn is_aws_bedrock_credentials_toggleable(&self) -> bool {
        matches!(
            self.aws_bedrock_host_enablement_setting(),
            HostEnablementSetting::RespectUserSetting
        )
    }

    pub fn is_aws_bedrock_credentials_enabled(&self, app: &AppContext) -> bool {
        // i.e. did the admin go and toggle on aws bedrock in the admin panel?
        if !self.is_aws_bedrock_available_from_workspace() {
            return false;
        }

        match self.aws_bedrock_host_enablement_setting() {
            HostEnablementSetting::Enforce => true,
            HostEnablementSetting::RespectUserSetting => *AISettings::as_ref(app)
                .aws_bedrock_credentials_enabled
                .value(),
        }
    }

    /// Returns the AI autonomy settings that are enforced by the workspace for all its members.
    /// If a setting is `None`, the workspace doesn't enforce a particular setting.
    pub fn ai_autonomy_settings(&self) -> AiAutonomySettings {
        self.current_team()
            .map(|team| team.organization_settings.ai_autonomy_settings.clone())
            .unwrap_or_default()
    }

    /// Returns the sandboxed agent settings enforced by the workspace, if any.
    pub fn sandboxed_agent_settings(&self) -> Option<SandboxedAgentSettings> {
        self.current_team()
            .and_then(|team| team.organization_settings.sandboxed_agent_settings.clone())
    }

    pub fn is_ai_autonomy_allowed(&self) -> bool {
        true
    }

    pub fn has_teams(&self) -> bool {
        false
    }

    pub fn has_workspaces(&self) -> bool {
        !self.workspaces.is_empty()
    }

    pub fn update_workspaces(&mut self, _workspaces: Vec<Workspace>, ctx: &mut ModelContext<Self>) {
        *self.current_workspace_uid = None;
        self.workspaces.clear();
        self.notify_and_emit_teams_changed(ctx);
    }

    fn notify_and_emit_teams_changed(&self, ctx: &mut ModelContext<Self>) {
        // Update session-sharing enablement since it depends on what teams the user
        // is part of.
        self.update_session_sharing_enablement(ctx);

        // PrivacySettings can't observe UserWorkspaces for updates, as it's initialized too early in
        // the app initialization flow. So, we update it manually whenever teams data changes.
        PrivacySettings::handle(ctx).update(ctx, |settings, ctx| {
            settings.set_is_telemetry_force_enabled(self.is_telemetry_force_enabled());
            settings.set_enterprise_secret_redaction_settings(
                self.is_enterprise_secret_redaction_enabled(),
                self.get_enterprise_secret_redaction_regex_list(),
                ChangeEventReason::CloudSync,
                ctx,
            );
        });

        ctx.emit(UserWorkspacesEvent::TeamsChanged);
        ctx.emit(UserWorkspacesEvent::CodebaseContextEnablementChanged);
        ctx.notify();
    }

    pub fn is_telemetry_force_enabled(&self) -> bool {
        self.current_team()
            .map(|team| team.organization_settings.telemetry_settings.force_enabled)
            .unwrap_or(false)
    }

    pub fn is_enterprise_secret_redaction_enabled(&self) -> bool {
        self.current_team()
            .map(|team| team.organization_settings.secret_redaction_settings.enabled)
            .unwrap_or(false)
    }

    pub fn get_enterprise_secret_redaction_regex_list(&self) -> Vec<EnterpriseSecretRegex> {
        self.current_team()
            .map(|team| {
                team.organization_settings
                    .secret_redaction_settings
                    .regexes
                    .clone()
            })
            .unwrap_or_default()
    }

    pub fn get_ugc_collection_enablement_setting(&self) -> UgcCollectionEnablementSetting {
        self.current_team()
            .map(|team| {
                team.organization_settings
                    .ugc_collection_settings
                    .setting
                    .clone()
            })
            .unwrap_or_default()
    }

    pub fn get_cloud_conversation_storage_enablement_setting(&self) -> AdminEnablementSetting {
        self.current_team()
            .map(|team| {
                team.organization_settings
                    .cloud_conversation_storage_settings
                    .setting
                    .clone()
            })
            .unwrap_or_default()
    }

    pub fn is_ai_allowed_in_remote_sessions(&self) -> bool {
        self.current_team()
            .map(|team| {
                team.organization_settings
                    .ai_permissions_settings
                    .allow_ai_in_remote_sessions
            })
            .unwrap_or(true)
    }

    pub fn get_remote_session_regex_list(&self) -> Vec<Regex> {
        self.current_team()
            .map(|team| {
                team.organization_settings
                    .ai_permissions_settings
                    .remote_session_regex_list
                    .clone()
            })
            .unwrap_or_default()
    }

    pub fn is_anyone_with_link_sharing_enabled(&self) -> bool {
        self.current_team()
            .map(|team| {
                team.organization_settings
                    .link_sharing_settings
                    .anyone_with_link_sharing_enabled
            })
            .unwrap_or(true)
    }

    pub fn is_direct_link_sharing_enabled(&self) -> bool {
        self.current_team()
            .map(|team| {
                team.organization_settings
                    .link_sharing_settings
                    .direct_link_sharing_enabled
            })
            .unwrap_or(true)
    }

    /// Returns the codebase context settings, taking into account the organization,
    /// global AI settings, and codebase-specific settings.
    /// Prefer this function to determine whether to show indexing-related functionality.
    pub fn is_codebase_context_enabled(&self, app: &AppContext) -> bool {
        // If the organization has an explicit setting, respect it and make user toggle irrelevant.
        // - Enable: forced ON by org, regardless of user preference.
        // - Disable: forced OFF by org.
        // - RespectUserSetting: respect the user setting.
        let org_setting = self.team_allows_codebase_context();
        let ai_globally_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);

        match org_setting {
            AdminEnablementSetting::Enable => ai_globally_enabled,
            AdminEnablementSetting::Disable => false,
            AdminEnablementSetting::RespectUserSetting => {
                ai_globally_enabled && *CodeSettings::as_ref(app).codebase_context_enabled.value()
            }
        }
    }

    /// Returns the team-level agent attribution setting.
    ///
    /// Use this to decide whether the user's attribution toggle should be locked
    /// (`Enable`/`Disable`) or editable (`RespectUserSetting`).
    pub fn get_agent_attribution_setting(&self) -> AdminEnablementSetting {
        self.current_team()
            .map(|team| team.organization_settings.enable_warp_attribution.clone())
            .unwrap_or_default()
    }

    /// Returns only the organization-specific codebase context enablement setting.
    /// Do not use this function to determine whether codebase context is generally enabled --
    /// use `is_codebase_context_enabled` instead.
    pub fn team_allows_codebase_context(&self) -> AdminEnablementSetting {
        self.current_team()
            .map(|team| {
                team.organization_settings
                    .codebase_context_settings
                    .setting
                    .clone()
            })
            .unwrap_or_default()
    }

    /// Updates whether or not session sharing is enabled based on the current team's tier policy.
    fn update_session_sharing_enablement(&self, _ctx: &AppContext) {
        if cfg!(any(test, feature = "integration_tests")) {
            return;
        }

        // Session sharing is amputated in Warper; there is no feature flag or
        // hosted policy to update.
    }
}

#[cfg(test)]
impl UserWorkspaces {
    pub fn setup_test_workspace(&mut self, ctx: &mut ModelContext<Self>) {
        self.update_workspaces(vec![], ctx);
    }

    pub fn update_current_workspace<F>(&mut self, f: F, ctx: &mut ModelContext<Self>)
    where
        F: FnOnce(&mut Workspace),
    {
        let _ = (f, ctx);
    }

    pub fn update_sandboxed_agent_settings<F>(&mut self, f: F, ctx: &mut ModelContext<Self>)
    where
        F: FnOnce(&mut Option<SandboxedAgentSettings>),
    {
        let _ = (f, ctx);
    }

    pub fn update_ai_autonomy_settings<F>(&mut self, f: F, ctx: &mut ModelContext<Self>)
    where
        F: FnOnce(&mut AiAutonomySettings),
    {
        let _ = (f, ctx);
    }
}

impl Entity for UserWorkspaces {
    type Event = UserWorkspacesEvent;
}

/// Mark UserWorkspaces as global application state.
impl SingletonEntity for UserWorkspaces {}
