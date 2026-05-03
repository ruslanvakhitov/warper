use std::{
    panic::{catch_unwind, UnwindSafe},
    sync::{Mutex, MutexGuard},
};

use crate::AppId;

use super::{derive_http_origin_from_ws_url, Channel, ChannelConfig, ChannelState};

static CHANNEL_STATE_TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_channel_state() -> MutexGuard<'static, ()> {
    CHANNEL_STATE_TEST_LOCK.lock().unwrap()
}

fn empty_config() -> ChannelConfig {
    ChannelConfig {
        app_id: AppId::new("dev", "warper", "Warper"),
        logfile_name: "".into(),
        server_config: None,
        oz_config: None,
        telemetry_config: None,
        autoupdate_config: None,
        mcp_static_config: None,
    }
}

#[test]
fn wss_becomes_https_and_strips_path() {
    let got = derive_http_origin_from_ws_url("wss://127.0.0.1:8080/graphql/v2");
    assert_eq!(got.as_deref(), Some("https://127.0.0.1:8080"));
}

#[test]
fn ws_becomes_http_and_preserves_port() {
    let got = derive_http_origin_from_ws_url("ws://localhost:8080/graphql/v2");
    assert_eq!(got.as_deref(), Some("http://localhost:8080"));
}

#[test]
fn unparseable_input_returns_none() {
    assert!(derive_http_origin_from_ws_url("not a url").is_none());
    assert!(derive_http_origin_from_ws_url("https://127.0.0.1:8080").is_none());
}

#[test]
fn default_channel_state_has_no_hosted_services() {
    let _guard = lock_channel_state();
    ChannelState::set(ChannelState::init());

    assert!(!ChannelState::is_warp_server_available());
    assert!(!ChannelState::is_oz_available());
    assert!(!ChannelState::is_telemetry_available());
    assert!(!ChannelState::show_autoupdate_menu_items());
    assert!(ChannelState::maybe_server_root_url().is_none());
    assert!(ChannelState::maybe_ws_server_url().is_none());
    assert!(ChannelState::session_sharing_server_url().is_none());
    assert!(ChannelState::maybe_firebase_api_key().is_none());
    assert!(ChannelState::maybe_oz_root_url().is_none());
    assert!(ChannelState::maybe_workload_audience_url().is_none());
    assert!(ChannelState::maybe_releases_base_url().is_none());
    assert!(!ChannelState::uses_staging_server());
}

#[test]
fn default_channel_state_contains_no_hosted_service_material() {
    let state = ChannelState::init();

    assert!(state.config.server_config.is_none());
    assert!(state.config.oz_config.is_none());
    assert!(state.config.telemetry_config.is_none());
    assert!(state.config.autoupdate_config.is_none());
    assert!(state.config.mcp_static_config.is_none());

    let debug = format!("{state:?}");
    for forbidden in [
        concat!("app", ".warp.dev"),
        concat!("rtc", ".app", ".warp.dev"),
        concat!("sessions", ".app", ".warp.dev"),
        concat!("oz", ".warp.dev"),
        concat!("identitytoolkit", ".googleapis.com"),
        concat!("securetoken", ".googleapis.com"),
        "rudderstack",
        "sentry",
        concat!("releases", ".warp.dev"),
        concat!("channel", "_versions.json"),
    ] {
        assert!(
            !debug.contains(forbidden),
            "default ChannelState must not contain hosted service material: {forbidden}"
        );
    }
}

#[test]
fn legacy_hosted_accessors_fail_closed_without_placeholders() {
    let _guard = lock_channel_state();
    ChannelState::set(ChannelState::init());

    assert_panics(|| ChannelState::server_root_url());
    assert_panics(|| ChannelState::ws_server_url());
    assert_panics(|| ChannelState::rtc_http_url());
    assert_panics(|| ChannelState::session_sharing_server_url().unwrap());
    assert_panics(|| ChannelState::firebase_api_key());
    assert_panics(|| ChannelState::oz_root_url());
    assert_panics(|| ChannelState::workload_audience_url());
    assert_panics(|| ChannelState::releases_base_url());
}

#[test]
fn offline_local_startup_config_requires_no_network_credentials() {
    let _guard = lock_channel_state();
    ChannelState::set(ChannelState::init());

    assert_eq!(ChannelState::app_id().to_string(), "dev.warper.Warper");
    assert_eq!(ChannelState::logfile_name(), "");
    assert_eq!(ChannelState::telemetry_file_name(), "");
    assert!(ChannelState::maybe_firebase_api_key().is_none());
    assert!(ChannelState::mcp_oauth_provider_by_client_id("any-client").is_none());
    assert!(ChannelState::mcp_oauth_provider_by_issuer("https://issuer.example").is_none());
}

#[test]
fn oss_channel_ignores_hosted_url_overrides() {
    let _guard = lock_channel_state();
    ChannelState::set(ChannelState::new(Channel::Oss, empty_config()));

    ChannelState::override_server_root_url("http://localhost:8080").unwrap();
    ChannelState::override_ws_server_url("ws://localhost:8081/graphql/v2").unwrap();
    ChannelState::override_session_sharing_server_url("ws://localhost:8082").unwrap();

    assert!(!ChannelState::is_warp_server_available());
    assert!(ChannelState::maybe_server_root_url().is_none());
    assert!(ChannelState::maybe_ws_server_url().is_none());
    assert!(ChannelState::session_sharing_server_url().is_none());
}

#[test]
fn malformed_stale_hosted_overrides_do_not_install_service_config() {
    let _guard = lock_channel_state();
    ChannelState::set(ChannelState::new(Channel::Oss, empty_config()));

    assert!(ChannelState::override_server_root_url("not a url").is_err());
    assert!(ChannelState::override_ws_server_url("not a websocket url").is_err());
    assert!(ChannelState::override_session_sharing_server_url("").is_err());

    assert!(!ChannelState::is_warp_server_available());
    assert!(ChannelState::maybe_server_root_url().is_none());
    assert!(ChannelState::maybe_ws_server_url().is_none());
    assert!(ChannelState::session_sharing_server_url().is_none());
    assert!(!ChannelState::debug_str().contains("not a url"));
    assert!(!ChannelState::debug_str().contains("not a websocket url"));
}

#[test]
fn local_channel_can_install_local_server_overrides() {
    let _guard = lock_channel_state();
    ChannelState::set(ChannelState::new(Channel::Local, empty_config()));

    ChannelState::override_server_root_url("http://localhost:8080").unwrap();
    ChannelState::override_ws_server_url("ws://localhost:8081/graphql/v2").unwrap();
    ChannelState::override_session_sharing_server_url("ws://localhost:8082").unwrap();

    assert!(ChannelState::is_warp_server_available());
    assert_eq!(
        ChannelState::maybe_server_root_url().as_deref(),
        Some("http://localhost:8080")
    );
    assert_eq!(
        ChannelState::maybe_ws_server_url().as_deref(),
        Some("ws://localhost:8081/graphql/v2")
    );
    assert_eq!(
        ChannelState::session_sharing_server_url().as_deref(),
        Some("ws://localhost:8082")
    );
}

fn assert_panics<F, T>(f: F)
where
    F: FnOnce() -> T + UnwindSafe,
{
    assert!(catch_unwind(f).is_err());
}
