use std::{
    panic::{catch_unwind, UnwindSafe},
    sync::{Mutex, MutexGuard},
};

use super::ChannelState;

static CHANNEL_STATE_TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_channel_state() -> MutexGuard<'static, ()> {
    CHANNEL_STATE_TEST_LOCK.lock().unwrap()
}

#[test]
fn default_channel_state_has_no_hosted_services() {
    let _guard = lock_channel_state();
    ChannelState::set(ChannelState::init());

    assert!(!ChannelState::is_warp_server_available());
    assert!(!ChannelState::is_oz_available());
    assert!(!ChannelState::show_autoupdate_menu_items());
    assert!(ChannelState::maybe_server_root_url().is_none());
    assert!(ChannelState::maybe_ws_server_url().is_none());
    assert!(ChannelState::session_sharing_server_url().is_none());
    assert!(ChannelState::maybe_oz_root_url().is_none());
    assert!(ChannelState::maybe_workload_audience_url().is_none());
    assert!(ChannelState::maybe_releases_base_url().is_none());
    assert!(!ChannelState::uses_staging_server());
}

#[test]
fn default_channel_state_contains_no_hosted_service_material() {
    let state = ChannelState::init();

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
    assert!(ChannelState::mcp_oauth_provider_by_client_id("any-client").is_none());
    assert!(ChannelState::mcp_oauth_provider_by_issuer("https://issuer.example").is_none());
}

fn assert_panics<F, T>(f: F)
where
    F: FnOnce() -> T + UnwindSafe,
{
    assert!(catch_unwind(f).is_err());
}
