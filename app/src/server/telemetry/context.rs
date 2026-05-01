//! Module that builds local telemetry context used by retained local protocols.

use crate::server::OperatingSystemInfo;

use serde::Serialize;
use serde_json::{json, Value};

use std::sync::OnceLock;

#[cfg(target_family = "wasm")]
use warpui::platform::wasm;

static TELEMETRY_CONTEXT: OnceLock<TelemetryContext> = OnceLock::new();

#[derive(Serialize)]
struct TelemetryContextInfo {
    /// Info about the operating system of the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    os: Option<&'static OperatingSystemInfo>,
    /// The user agent provided by the browser, if running on Web. If not on
    /// Web, this is always `None`.
    #[serde(rename = "userAgent", skip_serializing_if = "Option::is_none")]
    user_agent: Option<String>,
}

/// Newtype representing a [`Value`] with serialized local runtime context.
pub struct TelemetryContext(Value);

impl TelemetryContext {
    pub fn as_value(&self) -> Value {
        self.0.clone()
    }
}

impl TelemetryContext {
    fn new() -> Self {
        let context = TelemetryContextInfo {
            os: OperatingSystemInfo::get().ok(),
            user_agent: user_agent(),
        };

        match serde_json::to_value(context) {
            Ok(value) => Self(value),
            Err(e) => {
                log::error!("Failed to serialize telemetry context info to JSON value: {e:?}");
                Self(json!({}))
            }
        }
    }
}

/// Returns the user agent provided by the browser, if on Web. If not on Web,
/// or if the user agent was not able to be read, returns None.
fn user_agent() -> Option<String> {
    cfg_if::cfg_if! {
        if #[cfg(target_family = "wasm")] {
            wasm::user_agent()
        } else {
            None
        }
    }
}

/// Returns local runtime context for retained local protocols.
pub fn telemetry_context() -> &'static TelemetryContext {
    TELEMETRY_CONTEXT.get_or_init(TelemetryContext::new)
}
