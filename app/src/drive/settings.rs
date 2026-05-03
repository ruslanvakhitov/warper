use settings::{macros::define_settings_group, SupportedPlatforms, SyncToCloud};

use super::DriveSortOrder;

pub const HAS_AUTO_OPENED_WELCOME_FOLDER: &str = "HasAutoOpenedWelcomeFolder";

define_settings_group!(LocalDriveSettings, settings: [
    sorting_choice: LocalDriveSortingChoice {
        type: DriveSortOrder,
        default: DriveSortOrder::ByObjectType,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Never,
        private: false,
        toml_path: "warp_drive.sorting_choice",
        description: "Legacy local object sort order retained for old config migration.",
    },
    sharing_onboarding_block_shown: LocalDriveSharingOnboardingBlockShown {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Never,
        private: true,
    },
    enable_warp_drive: EnableLocalDrive {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Never,
        private: false,
        toml_path: "warp_drive.enabled",
        description: "Legacy Warp Drive enablement retained only to ignore old local config.",
    },
]);

impl LocalDriveSettings {
    pub fn is_local_drive_enabled(app: &warpui::AppContext) -> bool {
        let _ = app;
        false
    }
}
