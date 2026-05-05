use settings::{macros::define_settings_group, SupportedPlatforms, SyncToCloud};

define_settings_group!(LocalPreferencesSettings, settings: [
   settings_sync_enabled: IsSettingsSyncEnabled {
       type: bool,
       default: false,
       supported_platforms: SupportedPlatforms::ALL,
       sync_to_cloud: SyncToCloud::Never,
       private: false,
       toml_path: "account.is_settings_sync_enabled",
       description: "Legacy settings sync value retained only to ignore old local config.",
   },
]);
