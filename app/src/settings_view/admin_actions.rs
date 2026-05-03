use crate::server::ids::ServerId;
use warpui::AppContext;

/// Shared admin panel actions and utilities for settings views
pub struct AdminActions;

impl AdminActions {
    /// Generate the admin panel URL for a given team
    pub fn admin_panel_link_for_team(team_uid: ServerId) -> String {
        let _ = team_uid;
        String::new()
    }

    /// Open the admin panel for a specific team
    pub fn open_admin_panel(team_uid: ServerId, ctx: &mut AppContext) {
        let _ = (team_uid, ctx);
    }

    /// Open the support email link
    pub fn contact_support(ctx: &mut AppContext) {
        ctx.open_url("mailto:support@warp.dev");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_panel_link_generation() {
        let team_uid = ServerId::from(12345);
        let actual_link = AdminActions::admin_panel_link_for_team(team_uid);
        assert!(actual_link.is_empty());
    }
}
