use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// Defines the platform that a preference was set on.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Platform {
    Mac,
    Linux,
    Windows,
    Web,

    /// This implies the preference applies on all supported platforms
    Global,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mac => write!(f, "Mac"),
            Self::Linux => write!(f, "Linux"),
            Self::Windows => write!(f, "Windows"),
            Self::Web => write!(f, "Web"),
            Self::Global => write!(f, "Global"),
        }
    }
}

impl Platform {
    pub fn applies_to_current_platform(&self) -> bool {
        *self == Platform::current_platform() || *self == Platform::Global
    }
}

impl Platform {
    pub fn current_platform() -> Self {
        if cfg!(all(not(target_family = "wasm"), target_os = "macos")) {
            return Self::Mac;
        }

        if cfg!(all(not(target_family = "wasm"), target_os = "linux")) {
            return Self::Linux;
        }

        if cfg!(all(not(target_family = "wasm"), target_os = "windows")) {
            return Self::Windows;
        }
        if cfg!(target_family = "wasm") {
            return Self::Web;
        }
        panic!("Unsupported platform");
    }
}

/// Legacy serialized preference object retained for reading old local rows.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Preference {
    /// The storage key (unique identifier for this preference).
    pub storage_key: String,

    /// The value of the preference, which can be any JSON value.
    pub value: Value,

    /// The platform that this preference was set on.
    /// If the preference is global, this will be set to Platform::Global.
    pub platform: Platform,
}

impl Preference {
    /// Creates a legacy preference object with the given storage key and value.
    pub fn new(storage_key: String, value: &str, syncing_mode: SyncToCloud) -> Result<Self> {
        let platform = match syncing_mode {
            SyncToCloud::PerPlatform(_) => Platform::current_platform(),
            SyncToCloud::Globally(_) => Platform::Global,
            SyncToCloud::Never => Err(anyhow!(
                "Cannot create a preference with SyncToCloud::Never"
            ))?,
        };
        match serde_json::from_str(value) {
            Ok(value) => Ok(Self {
                storage_key,
                value,
                platform,
            }),
            Err(err) => Err(anyhow!("Failed to parse preference value {}", err)),
        }
    }
}
