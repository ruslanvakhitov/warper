use std::path::Path;

use warpui::rendering::GPUDeviceInfo;
use warpui::AppContext;

use crate::antivirus::AntivirusInfo;
use crate::auth::UserUid;

#[cfg(linux_or_windows)]
pub fn run_minidump_server(socket_name: impl AsRef<Path>) -> anyhow::Result<()> {
    log::info!(
        "Crash upload is disabled; ignoring minidump server request for {}",
        socket_name.display()
    );
    Ok(())
}

pub(crate) fn set_tag<'a, 'b>(
    _key: impl Into<std::borrow::Cow<'a, str>>,
    _value: impl Into<std::borrow::Cow<'b, str>>,
) {
}

pub(crate) fn set_gpu_device_info(_gpu_device_info: GPUDeviceInfo) {}

pub fn set_antivirus_info(_antivirus_info: &AntivirusInfo) {}

pub(crate) fn init(_ctx: &mut AppContext) -> bool {
    log::info!("Crash upload is disabled; Sentry is not initialized.");
    false
}

pub fn uninit_sentry() {}

pub fn init_cocoa_sentry() {}

pub fn uninit_cocoa_sentry() {}

pub fn crash() {
    log::warn!("Crash reporting test crash requested, but crash upload is disabled.");
}

pub fn set_user_id(_user_id: UserUid, _email: Option<String>, _ctx: &mut AppContext) {
    log::debug!("Crash upload is disabled; not setting hosted crash-reporting user info.");
}

pub fn set_client_type_tag(_client_id: &str) {}
