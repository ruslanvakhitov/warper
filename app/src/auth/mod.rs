pub mod anonymous_id;
pub mod auth_manager;
pub mod auth_state;
pub mod auth_view_modal;
pub mod credentials;
pub mod user;
pub mod user_uid;

pub use auth_manager::AuthManager;
pub use auth_state::AuthStateProvider;
pub use user_uid::UserUid;

use warpui::AppContext;

/// Hosted Warp auth UI and logout flows are amputated for Warper. Retained
/// callers route here from stale actions, menus, or tests; the local app state
/// is intentionally left unchanged.
pub fn maybe_log_out(_app: &mut AppContext) {}

/// Hosted Warp auth is absent, so logout is an inert compatibility endpoint.
pub fn log_out(_app: &mut AppContext) {}
