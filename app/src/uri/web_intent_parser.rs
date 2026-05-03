#[cfg(target_family = "wasm")]
use crate::uri::browser_url_handler::parse_current_url;
use anyhow::{anyhow, Result};
use url::Url;

#[cfg(target_family = "wasm")]
use warp_core::context_flag::ContextFlag;

#[derive(Debug)]
/// Represents an intent parsed from a web url
pub enum WebIntent {
    SettingsView(Url),
    Home(Url),
    Action(Url),
}

impl WebIntent {
    pub fn try_from_url(url: &Url) -> Result<Self> {
        let _ = url;
        Err(anyhow!("Hosted Warp web URLs are not supported in Warper"))
    }

    /// Convert this web intent into the underlying native desktop URL.
    pub fn into_intent_url(self) -> Url {
        match self {
            WebIntent::SettingsView(url) => url,
            WebIntent::Home(url) => url,
            WebIntent::Action(url) => url,
        }
    }
}

/// Attempts to rewrite a Warp web URL into a native desktop intent URL (warp://...).
/// Returns `None` if the URL is not a recognized Warp web intent.
pub fn maybe_rewrite_web_url_to_intent(url: &Url) -> Option<Url> {
    WebIntent::try_from_url(url)
        .ok()
        .map(WebIntent::into_intent_url)
}

/// On WASM warp, fires an event to try and open the given link on the desktop app.
#[cfg(target_family = "wasm")]
pub fn open_url_on_desktop(url: &Url) {
    match WebIntent::try_from_url(url) {
        Ok(WebIntent::Action(intent)) => {
            crate::platform::wasm::emit_event(crate::platform::wasm::WarpEvent::OpenOnNative {
                url: intent.into(),
            });
        }
        _ => {
            log::warn!("Attempting to open invalid url on desktop app:{url}");
        }
    };
}

#[cfg(target_family = "wasm")]
fn set_context_flags_from_url(url: Url) {
    match WebIntent::try_from_url(&url) {
        Ok(WebIntent::SettingsView(_)) => ContextFlag::set_settings_link_only(),
        Ok(WebIntent::Home(_)) => ContextFlag::set_warp_home_link_only(),
        Ok(WebIntent::Action(_)) => {} // No special context flag for actions
        _ => {}
    }
}

/// Looks at the current URL and converts it into an app intent.
#[cfg(target_family = "wasm")]
pub fn current_web_intent() -> Option<WebIntent> {
    let Some(current_url) = parse_current_url() else {
        log::warn!("Unable to parse the current url");
        return None;
    };

    WebIntent::try_from_url(&current_url).ok()
}

// Looks at the current url and converts it into an app intent.
// NOTE: This is only intended for use with target_family = "wasm"
#[cfg(target_family = "wasm")]
pub fn parse_web_intent_from_current_url() -> Option<Url> {
    current_web_intent().map(WebIntent::into_intent_url)
}

#[cfg(target_family = "wasm")]
pub fn set_context_flags_from_current_url() {
    let Some(current_url) = parse_current_url() else {
        log::warn!("Unable to parse the current url");
        return;
    };

    set_context_flags_from_url(current_url);
}
