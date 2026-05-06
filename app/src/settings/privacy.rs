use std::fmt::Display;

use regex::Regex;
use warp_core::user_preferences::GetUserPreferences as _;
use warpui::{AppContext, Entity, ModelContext, SingletonEntity, UpdateModel};

use crate::terminal::safe_mode_settings::SafeModeSettings;

use settings::{
    macros::{define_settings_group, maybe_define_setting, register_settings_events},
    Setting, SupportedPlatforms, SyncToCloud,
};

use serde::{Deserialize, Serialize};

use crate::workspaces::workspace::EnterpriseSecretRegex;

pub trait RegexDisplayInfo {
    fn pattern(&self) -> &str;
    fn name(&self) -> Option<&str>;
}

pub const CLOUD_CONVERSATION_STORAGE_ENABLED_DEFAULTS_KEY: &str = "CloudConversationStorageEnabled";

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "A custom regex pattern for detecting and redacting secrets.")]
pub struct CustomSecretRegex {
    #[serde(with = "serde_regex")]
    #[schemars(with = "String", description = "The regex pattern to match secrets.")]
    pub pattern: Regex,
    #[serde(default)]
    #[schemars(description = "Optional display name for this secret pattern.")]
    pub name: Option<String>,
}

impl CustomSecretRegex {
    pub fn pattern(&self) -> &Regex {
        &self.pattern
    }
}

impl RegexDisplayInfo for CustomSecretRegex {
    fn pattern(&self) -> &str {
        self.pattern.as_str()
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl RegexDisplayInfo for EnterpriseSecretRegex {
    fn pattern(&self) -> &str {
        &self.pattern
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl Display for CustomSecretRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pattern.as_str())
    }
}

impl PartialEq for CustomSecretRegex {
    /// We do not factor in the name to equality checks --
    /// if the regex is the same, then the regex is the same.
    /// This allows us to avoid adding duplicate regexes.
    fn eq(&self, other: &Self) -> bool {
        self.pattern.as_str() == other.pattern.as_str()
    }
}

impl settings_value::SettingsValue for CustomSecretRegex {}

define_settings_group!(LocalPrivacySettings, settings: [
    is_cloud_conversation_storage_enabled: IsCloudConversationStorageEnabled {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Never,
        private: false,
        storage_key: "CloudConversationStorageEnabled",
        toml_path: "agents.cloud_conversation_storage_enabled",
        description: "Whether hosted conversation storage is enabled.",
    },
]);

maybe_define_setting!(CustomSecretRegexList, group: PrivacySettings, {
    type: Vec<CustomSecretRegex>,
    default: Vec::new(),
    supported_platforms: SupportedPlatforms::ALL,
    sync_to_cloud: SyncToCloud::Never,
    private: false,
    toml_path: "privacy.custom_secret_regex_list",
    description: "Custom regex patterns for detecting and redacting secrets.",
});

maybe_define_setting!(HasInitializedDefaultSecretRegexes, group: PrivacySettings, {
    type: bool,
    default: false,
    supported_platforms: SupportedPlatforms::ALL,
    sync_to_cloud: SyncToCloud::Never,
    private: true,
});

/// Singleton model for local privacy settings. Warper keeps these settings local and does not
/// upload diagnostics, sync privacy settings, or send crash diagnostics to hosted services.
pub struct PrivacySettings {
    pub is_cloud_conversation_storage_enabled: bool,
    pub has_initialized_default_secret_regexes: HasInitializedDefaultSecretRegexes,
    /// List of user defined secret regexes.
    /// Enterprise-level secret regexes will always take precedence over user-level secrets,
    /// but they both used to support additive behavior.
    /// It's a [Vec<CustomSecretRegex>], but also a user setting.
    pub user_secret_regex_list: CustomSecretRegexList,
    /// List of enterprise-level secret regexes provided by the organization.
    /// These are kept separate from user-level secrets to support additive behavior.
    pub enterprise_secret_regex_list: Vec<CustomSecretRegex>,
    /// Enterprise hosted policy is amputated; user-defined local regexes still work.
    pub is_enterprise_secret_redaction_enabled: bool,
}

/// A snapshot of a user's [`PrivacySettings`] settings at some point in time.
#[derive(Clone, Copy)]
pub struct PrivacySettingsSnapshot {
    // This is an option so that, if a user has not set this value (and it's set to its default value of true),
    // the default value won't override a value that the user previously set on a different device.
    // This is set to a non-option once the user manually changes this setting.
    cloud_conversation_storage_enabled: Option<bool>,
}

impl PrivacySettingsSnapshot {
    pub fn cloud_conversation_storage_enabled(&self) -> Option<bool> {
        self.cloud_conversation_storage_enabled
    }

    #[cfg(test)]
    pub fn mock() -> Self {
        Self {
            cloud_conversation_storage_enabled: None,
        }
    }
}

impl PrivacySettings {
    /// Registers a singleton PrivacySettings model on `app`.
    ///
    /// We expose this function publicly (while keeping the constructor private) to prevent
    /// instantiation another PrivacySettings struct, in the case where a developer might be
    /// unaware that it is registered as a singleton model.
    pub fn register_singleton(ctx: &mut AppContext) {
        let handle = ctx.add_singleton_model(PrivacySettings::new);

        register_settings_events!(
            PrivacySettings,
            user_secret_regex_list,
            CustomSecretRegexList,
            handle,
            ctx
        );
    }

    /// Returns a new PrivacySettings object initialized from locally cached values.
    fn new(ctx: &mut ModelContext<Self>) -> Self {
        let is_cloud_conversation_storage_enabled: bool = ctx
            .private_user_preferences()
            .read_value(CLOUD_CONVERSATION_STORAGE_ENABLED_DEFAULTS_KEY)
            .unwrap_or_default()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(false);

        // Make sure the user-preferences stores match what's in memory.
        // Needed for warp drive preferences to work and no harm in doing in general.
        let _ = ctx.private_user_preferences().write_value(
            CLOUD_CONVERSATION_STORAGE_ENABLED_DEFAULTS_KEY,
            serde_json::to_string(&is_cloud_conversation_storage_enabled)
                .expect("is_cloud_conversation_storage_enabled is a boolean."),
        );

        // Keep the generated settings model and this singleton in sync locally.
        ctx.subscribe_to_model(&LocalPrivacySettings::handle(ctx), |me, event, ctx| {
            let privacy_settings = LocalPrivacySettings::as_ref(ctx);
            match event {
                LocalPrivacySettingsChangedEvent::IsCloudConversationStorageEnabled { .. } => {
                    me.set_is_cloud_conversation_storage_enabled(
                        *privacy_settings
                            .is_cloud_conversation_storage_enabled
                            .value(),
                        ctx,
                    );
                }
            }
        });

        let user_secret_regex_list: CustomSecretRegexList =
            CustomSecretRegexList::new_from_storage(ctx);
        let has_initialized_default_secret_regexes: HasInitializedDefaultSecretRegexes =
            HasInitializedDefaultSecretRegexes::new_from_storage(ctx);

        Self {
            is_cloud_conversation_storage_enabled,
            user_secret_regex_list,
            has_initialized_default_secret_regexes,
            is_enterprise_secret_redaction_enabled: false,
            enterprise_secret_regex_list: Vec::new(),
        }
    }

    pub fn is_enterprise_secret_redaction_enabled(&self) -> bool {
        self.is_enterprise_secret_redaction_enabled
    }

    pub fn set_enterprise_secret_redaction_settings(
        &mut self,
        enabled: bool,
        enterprise_regexes: Vec<EnterpriseSecretRegex>,
        change_event_reason: ChangeEventReason,
        ctx: &mut ModelContext<Self>,
    ) {
        if enabled {
            // First time: Force enable secret redaction setting (safe mode).
            if !self.is_enterprise_secret_redaction_enabled {
                let safe_mode_settings = SafeModeSettings::handle(ctx);
                ctx.update_model(&safe_mode_settings, |safe_mode_settings, ctx| {
                    let _ = safe_mode_settings.safe_mode_enabled.set_value(true, ctx);
                });
            }

            // Convert EnterpriseSecretRegex to CustomSecretRegex for internal use
            let mut enterprise_secrets = Vec::new();
            for enterprise_regex in enterprise_regexes {
                if let Ok(regex) = Regex::new(&enterprise_regex.pattern) {
                    enterprise_secrets.push(CustomSecretRegex {
                        pattern: regex,
                        name: enterprise_regex.name,
                    });
                } else {
                    log::error!(
                        "Invalid enterprise secret regex pattern: {}",
                        enterprise_regex.pattern
                    );
                }
            }
            self.enterprise_secret_regex_list = enterprise_secrets;
        } else {
            // Clear enterprise secrets when disabled
            self.enterprise_secret_regex_list.clear();
        }

        self.is_enterprise_secret_redaction_enabled = enabled;

        ctx.emit(PrivacySettingsChangedEvent::CustomSecretRegexList {
            change_event_reason,
        });
        ctx.notify();
    }

    pub fn refresh_to_default(&mut self) {
        // TODO(zach): this seems incorrect - should we also update the values on disk?
        self.is_cloud_conversation_storage_enabled = false;
        self.is_enterprise_secret_redaction_enabled = false;
    }

    /// Hosted privacy sync has been amputated. This method remains for retained callers and only
    /// initializes local secret redaction defaults.
    pub fn fetch_or_update_settings(&mut self, ctx: &mut ModelContext<Self>) {
        self.initialize_default_regexes_once(ctx);
    }

    /// Constructor for tests only.
    #[cfg(test)]
    pub fn mock(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            is_cloud_conversation_storage_enabled: false,
            user_secret_regex_list: CustomSecretRegexList::new(None),
            has_initialized_default_secret_regexes: HasInitializedDefaultSecretRegexes::new(None),
            is_enterprise_secret_redaction_enabled: false,
            enterprise_secret_regex_list: Vec::new(),
        }
    }

    /// Returns a snapshot of the user's privacy settings.
    ///
    /// The returned snapshot is not stateful, thus its values should be used shortly after the
    /// snapshot is returned.
    pub fn get_snapshot(&self, app: &AppContext) -> PrivacySettingsSnapshot {
        let _ = app;
        PrivacySettingsSnapshot {
            cloud_conversation_storage_enabled: (!self.is_cloud_conversation_storage_enabled)
                .then_some(false),
        }
    }

    pub fn set_is_cloud_conversation_storage_enabled(
        &mut self,
        new_value: bool,
        ctx: &mut ModelContext<PrivacySettings>,
    ) {
        let old_value = self.is_cloud_conversation_storage_enabled;
        if new_value == old_value {
            return;
        }

        self.is_cloud_conversation_storage_enabled = new_value;

        LocalPrivacySettings::handle(ctx).update(ctx, |settings, ctx| {
            log::info!("Setting is_cloud_conversation_storage_enabled to {new_value}");
            let _ = settings
                .is_cloud_conversation_storage_enabled
                .set_value(new_value, ctx);
        });

        ctx.emit(
            PrivacySettingsChangedEvent::UpdateIsCloudConversationStorageEnabled {
                old_value,
                new_value,
            },
        );
        ctx.notify();
    }

    pub fn remove_user_secret_regex(&mut self, idx: &usize, ctx: &mut ModelContext<Self>) {
        let mut new_user_secret_regex_list = self.user_secret_regex_list.to_vec();
        new_user_secret_regex_list.remove(*idx);
        if self
            .user_secret_regex_list
            .set_value(new_user_secret_regex_list, ctx)
            .is_err()
        {
            log::error!("Custom Secret Regex List failed to serialize")
        }
    }

    /// Initializes the custom secret regex list with the default regexes, provided
    /// non matches can be found.
    /// This can be called when a user first enables secret redaction.
    pub fn add_all_recommended_regex(&mut self, ctx: &mut ModelContext<Self>) {
        let mut new_user_secret_regex_list = self.user_secret_regex_list.to_vec();
        let num_existing_regexes = new_user_secret_regex_list.len();

        // Add all the default regexes if they don't already exist
        for default_regex in crate::terminal::model::secrets::regexes::DEFAULT_REGEXES_WITH_NAMES {
            if let Ok(regex) = Regex::new(default_regex.pattern) {
                let custom_regex = CustomSecretRegex {
                    pattern: regex,
                    name: Some(default_regex.name.to_string()),
                };
                if !new_user_secret_regex_list.contains(&custom_regex) {
                    new_user_secret_regex_list.push(custom_regex);
                }
            } else {
                log::error!("Failed to compile default regex: {}", default_regex.pattern);
            }
        }

        if num_existing_regexes == new_user_secret_regex_list.len() {
            return;
        }

        if self
            .user_secret_regex_list
            .set_value(new_user_secret_regex_list, ctx)
            .is_err()
        {
            log::error!("Failed to serialize default regexes to custom secret regex list")
        }

        ctx.notify();
    }

    /// Disables the default regex trigger, so that it will not be executed.
    pub fn disable_default_regex_trigger(&mut self, ctx: &mut ModelContext<Self>) {
        if self
            .has_initialized_default_secret_regexes
            .set_value(true, ctx)
            .is_err()
        {
            log::error!("Failed to disable default regex trigger");
        }
    }

    /// Initializes the custom secret regex list with the default regexes.
    /// This will only be executed once per user, and only if they haven't already initialized.
    pub fn initialize_default_regexes_once(&mut self, ctx: &mut ModelContext<Self>) {
        // Only initialize if we haven't done so before
        if !*self.has_initialized_default_secret_regexes.value() {
            self.add_all_recommended_regex(ctx);

            // Mark as initialized
            if self
                .has_initialized_default_secret_regexes
                .set_value(true, ctx)
                .is_err()
            {
                log::error!("Failed to set has_initialized_default_secret_regexes flag");
            }
        }
    }

    pub fn maybe_initialize_local_privacy_defaults(&mut self, ctx: &mut ModelContext<Self>) {
        self.initialize_default_regexes_once(ctx);
    }
}

/// Events emitted when PrivacySettings is updated.
#[derive(Clone, Copy)]
pub enum PrivacySettingsChangedEvent {
    UpdateIsCloudConversationStorageEnabled {
        old_value: bool,
        new_value: bool,
    },
    CustomSecretRegexList {
        change_event_reason: ChangeEventReason,
    },
    HasInitializedDefaultSecretRegexes {
        change_event_reason: ChangeEventReason,
    },
}

impl Entity for PrivacySettings {
    type Event = PrivacySettingsChangedEvent;
}

impl SingletonEntity for PrivacySettings {}
