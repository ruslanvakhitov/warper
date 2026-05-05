pub mod auth_state;

pub use auth_state::{AuthState, AuthStateProvider, UserUid};

use warpui::AppContext;

pub fn maybe_log_out(_app: &mut AppContext) {}

pub fn log_out(_app: &mut AppContext) {}
