//! Warp Home
//!
//! This is the landing page for new tabs if session creation isn't supported (e.g. on the web).
//! It's barebones at the moment, but may grow into a more full-featured admin experience.

use warpui::ViewContext;

use super::view::Workspace;
use crate::pane_group::{AnyPaneContent, FilePane};

const WARP_HOME_TITLE: &str = "Welcome to Warper";
const WARP_HOME_CONTENT: &str = r#"
Warper is running in local-only mode.

Use this window for local terminal sessions, local files, settings, keybindings, launch configs, and local workflows."#;

/// Create a static "home page" pane.
pub fn create_home_pane(ctx: &mut ViewContext<Workspace>) -> Box<dyn AnyPaneContent> {
    let pane = FilePane::new(
        None,
        None,
        #[cfg(feature = "local_fs")]
        None,
        ctx,
    );
    pane.file_view(ctx).update(ctx, |pane, ctx| {
        pane.open_static(WARP_HOME_TITLE, WARP_HOME_CONTENT, ctx);
    });
    Box::new(pane)
}
