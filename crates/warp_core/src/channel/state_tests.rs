use std::panic::{catch_unwind, UnwindSafe};

use super::{derive_http_origin_from_ws_url, ChannelState};

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
    ChannelState::set(ChannelState::init());

    assert!(!ChannelState::is_warp_server_available());
    assert!(!ChannelState::is_oz_available());
    assert!(!ChannelState::is_telemetry_available());
    assert!(!ChannelState::is_crash_reporting_available());
    assert!(!ChannelState::show_autoupdate_menu_items());
    assert!(ChannelState::maybe_server_root_url().is_none());
    assert!(ChannelState::maybe_ws_server_url().is_none());
    assert!(ChannelState::session_sharing_server_url().is_none());
    assert!(ChannelState::maybe_firebase_api_key().is_none());
    assert!(ChannelState::maybe_oz_root_url().is_none());
    assert!(ChannelState::maybe_workload_audience_url().is_none());
    assert!(ChannelState::maybe_releases_base_url().is_none());
    assert!(ChannelState::maybe_sentry_url().is_none());
    assert!(!ChannelState::uses_staging_server());
}

#[test]
fn legacy_hosted_accessors_fail_closed_without_placeholders() {
    ChannelState::set(ChannelState::init());

    assert_panics(|| ChannelState::server_root_url());
    assert_panics(|| ChannelState::ws_server_url());
    assert_panics(|| ChannelState::rtc_http_url());
    assert_panics(|| ChannelState::session_sharing_server_url().unwrap());
    assert_panics(|| ChannelState::firebase_api_key());
    assert_panics(|| ChannelState::oz_root_url());
    assert_panics(|| ChannelState::workload_audience_url());
    assert_panics(|| ChannelState::releases_base_url());
    assert_panics(|| ChannelState::sentry_url());
}

#[test]
fn offline_local_startup_config_requires_no_network_credentials() {
    ChannelState::set(ChannelState::init());

    assert_eq!(ChannelState::app_id().to_string(), "dev.warper.Warper");
    assert_eq!(ChannelState::logfile_name(), "");
    assert_eq!(ChannelState::telemetry_file_name(), "");
    assert!(ChannelState::maybe_firebase_api_key().is_none());
    assert!(ChannelState::mcp_oauth_provider_by_client_id("any-client").is_none());
    assert!(ChannelState::mcp_oauth_provider_by_issuer("https://issuer.example").is_none());
}

fn assert_panics<F, T>(f: F)
where
    F: FnOnce() -> T + UnwindSafe,
{
    assert!(catch_unwind(f).is_err());
}
