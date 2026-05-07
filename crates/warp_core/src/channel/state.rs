use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::{borrow::Cow, collections::HashSet};

use crate::AppId;
use crate::{
    channel::config::{ChannelConfig, McpOAuthProviderConfig},
    features::FeatureFlag,
};

use super::Channel;

lazy_static! {
    static ref CHANNEL_STATE: Mutex<ChannelState> = Mutex::new(ChannelState::init());
}

#[cfg(feature = "test-util")]
lazy_static! {
    static ref APP_VERSION: Mutex<Option<&'static str>> = Mutex::new(None);
}

#[derive(Debug)]
pub struct ChannelState {
    channel: Channel,

    /// The set of additional features to enable (on top of default-enabled ones).
    additional_features: HashSet<FeatureFlag>,

    config: ChannelConfig,
}

impl ChannelState {
    pub fn init() -> Self {
        #[cfg(any(test, feature = "test-util"))]
        let channel = Channel::Local;
        #[cfg(not(any(test, feature = "test-util")))]
        let channel = Channel::Oss;
        let app_id = AppId::new("dev", "warper", "Warper");
        Self {
            channel,
            additional_features: Default::default(),
            config: ChannelConfig {
                app_id,
                logfile_name: "".into(),
                mcp_static_config: None,
            },
        }
    }

    pub fn new(channel: Channel, mut config: ChannelConfig) -> Self {
        if let Some(app_id) = app_id_from_bundle() {
            config.app_id = app_id;
        }
        Self {
            channel,
            additional_features: Default::default(),
            config,
        }
    }

    pub fn with_additional_features(mut self, overrides: &[FeatureFlag]) -> Self {
        self.additional_features.extend(overrides);
        self
    }

    pub fn set(state: ChannelState) {
        *CHANNEL_STATE.lock() = state;
    }

    pub fn is_release_bundle() -> bool {
        cfg!(feature = "release_bundle")
    }

    pub fn enable_debug_features() -> bool {
        cfg!(debug_assertions) || matches!(Self::channel(), Channel::Local | Channel::Dev)
    }

    pub fn uses_staging_server() -> bool {
        false
    }

    /// Returns the canonical identifier for the application.
    ///
    /// This should not be used for namespacing persisted data - such use cases
    /// should make use of [`Self::data_domain`] instead.
    pub fn app_id() -> AppId {
        CHANNEL_STATE.lock().config.app_id.clone()
    }

    /// Returns a profile name for isolating user data. This should be used to
    /// sandbox how user data is stored.
    ///
    /// This is a debugging tool for isolating development instances of Warp, and is not
    /// supported in release builds.
    pub fn data_profile() -> Option<String> {
        if cfg!(debug_assertions) {
            std::env::var("WARP_DATA_PROFILE").ok()
        } else {
            None
        }
    }

    /// Returns a value that should be used for namespacing persisted data.
    ///
    /// In release builds, this is identical to the app ID; in debug builds,
    /// it optionally includes a suffix derived from the `WARP_DATA_PROFILE`
    /// environment variable.
    pub fn data_domain() -> String {
        match Self::data_profile() {
            Some(profile) => format!("{}-{profile}", Self::app_id()),
            None => Self::app_id().to_string(),
        }
    }

    /// Returns the data domain if overridden from the default, otherwise None.
    pub fn data_domain_if_not_default() -> Option<String> {
        Self::data_profile().map(|_| Self::data_domain())
    }

    pub fn additional_features() -> HashSet<FeatureFlag> {
        CHANNEL_STATE
            .lock()
            .additional_features
            .iter()
            .cloned()
            .collect()
    }

    pub fn debug_str() -> String {
        format!("{:?}", *CHANNEL_STATE.lock())
    }

    pub fn logfile_name() -> Cow<'static, str> {
        CHANNEL_STATE.lock().config.logfile_name.clone()
    }

    pub fn is_warp_server_available() -> bool {
        false
    }

    pub fn is_oz_available() -> bool {
        false
    }

    pub fn releases_base_url() -> Cow<'static, str> {
        Self::maybe_releases_base_url().expect("Warp hosted autoupdate config is unavailable")
    }

    pub fn maybe_releases_base_url() -> Option<Cow<'static, str>> {
        None
    }

    pub fn maybe_ws_server_url() -> Option<Cow<'static, str>> {
        None
    }

    pub fn maybe_server_root_url() -> Option<Cow<'static, str>> {
        None
    }

    pub fn maybe_oz_root_url() -> Option<Cow<'static, str>> {
        None
    }

    pub fn maybe_workload_audience_url() -> Option<Cow<'static, str>> {
        None
    }

    pub fn ws_server_url() -> Cow<'static, str> {
        Self::maybe_ws_server_url().expect("Warp RTC server config is unavailable")
    }

    /// Returns the HTTP(S) root URL for the RTC server.
    pub fn rtc_http_url() -> Cow<'static, str> {
        panic!("Warp RTC server config is unavailable")
    }

    pub fn session_sharing_server_url() -> Option<Cow<'static, str>> {
        None
    }

    pub fn oz_root_url() -> Cow<'static, str> {
        Self::maybe_oz_root_url().expect("Hosted agent config is unavailable")
    }

    pub fn server_root_url() -> Cow<'static, str> {
        Self::maybe_server_root_url().expect("Warp server config is unavailable")
    }

    pub fn workload_audience_url() -> Cow<'static, str> {
        Self::maybe_workload_audience_url().expect("Workload audience config is unavailable")
    }

    pub fn channel() -> Channel {
        CHANNEL_STATE.lock().channel
    }

    #[cfg(feature = "test-util")]
    pub fn app_version() -> Option<&'static str> {
        let version = APP_VERSION.lock();

        version.or_else(|| option_env!("GIT_RELEASE_TAG"))
    }

    #[cfg(feature = "test-util")]
    pub fn set_app_version(version: Option<&'static str>) {
        *APP_VERSION.lock() = version;
    }

    #[cfg(not(feature = "test-util"))]
    pub fn app_version() -> Option<&'static str> {
        option_env!("GIT_RELEASE_TAG")
    }

    pub fn show_autoupdate_menu_items() -> bool {
        false
    }

    /// Returns the MCP OAuth provider config matching the given client ID, if any.
    pub fn mcp_oauth_provider_by_client_id(client_id: &str) -> Option<McpOAuthProviderConfig> {
        CHANNEL_STATE
            .lock()
            .config
            .mcp_static_config
            .as_ref()
            .and_then(|c| c.providers.iter().find(|p| p.client_id == client_id))
            .cloned()
    }

    /// Returns the MCP OAuth provider config matching the given issuer URL, if any.
    pub fn mcp_oauth_provider_by_issuer(issuer: &str) -> Option<McpOAuthProviderConfig> {
        CHANNEL_STATE
            .lock()
            .config
            .mcp_static_config
            .as_ref()
            .and_then(|c| c.providers.iter().find(|p| p.issuer == issuer))
            .cloned()
    }

    pub fn url_scheme() -> &'static str {
        match Self::channel() {
            Channel::Stable => "warp",
            Channel::Preview => "warppreview",
            Channel::Dev => "warpdev",
            // Dummy value--integration tests shouldn't support URL schemes.
            Channel::Integration => "warpintegration",
            Channel::Local => "warplocal",
            Channel::Oss => "warper",
        }
    }
}

#[cfg(all(test, not(feature = "test-util")))]
#[path = "state_tests.rs"]
mod tests;

fn app_id_from_bundle() -> Option<AppId> {
    // On macOS, attempt to determine the app ID from the containing bundle,
    // falling back to the channel-keyed "default" ID if we cannot retrieve
    // bundle information.
    //
    // We skip this for tests, as the call to `mainBundle` can take 30+ms,
    // which is a significant portion of the total test runtime.
    #[cfg(all(target_os = "macos", not(feature = "test-util")))]
    #[allow(deprecated)]
    unsafe {
        use cocoa::{
            base::{id, nil},
            foundation::NSBundle,
        };
        use objc::{msg_send, sel, sel_impl};
        use warpui::platform::mac::utils::nsstring_as_str;

        let bundle = id::mainBundle();
        if bundle != nil {
            let nsstring: id = msg_send![bundle, bundleIdentifier];
            if nsstring != nil {
                let app_id = nsstring_as_str(nsstring)
                    .expect("bundle IDs should always be valid UTF-8 strings");

                if !app_id.is_empty() {
                    return Some(
                        AppId::parse(app_id)
                            .expect("macOS bundle identifier has an unexpected format"),
                    );
                }
            }
        }
    }

    None
}
