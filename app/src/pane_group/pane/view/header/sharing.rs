//! Support for pane contents that are shareable, like sessions and Warp Drive objects.
//!
//! This is tightly coupled to the pane header so that different overlays (context menus, the
//! sharing dialog, and so on) are correctly displayed.

use warp_core::ui::{appearance::Appearance, theme::Fill};
use warpui::{elements::ParentElement, AppContext, ViewContext};

use crate::{
    drive::sharing::ShareableObject, pane_group::BackingView,
    server::telemetry::SharingDialogSource,
};

use super::PaneHeader;

/// Pane header component for sharing the pane contents.
pub struct SharedPaneContent;

impl SharedPaneContent {
    pub fn new<P: BackingView>(_ctx: &mut ViewContext<PaneHeader<P>>) -> Self {
        Self
    }
}

impl<P: BackingView> PaneHeader<P> {
    pub fn set_shareable_object(
        &mut self,
        _shareable_object: Option<ShareableObject>,
        _ctx: &mut ViewContext<Self>,
    ) {
    }

    pub fn has_shareable_object<C: warpui::ViewAsRef>(&self, ctx: &C) -> bool {
        let _ = ctx;
        false
    }

    pub fn has_shareable_shared_session<C: warpui::ViewAsRef>(&self, ctx: &C) -> bool {
        let _ = ctx;
        false
    }

    pub fn is_sharing_dialog_enabled<C: warpui::ViewAsRef>(&self, ctx: &C) -> bool {
        let _ = ctx;
        false
    }

    /// Share the panes' contents.
    ///
    /// If the user can share the pane contents, this will bring up a sharing dialog. Otherwise, it copies
    /// the backing object's URL.
    pub fn share_pane_contents(
        &mut self,
        _source: SharingDialogSource,
        _ctx: &mut ViewContext<Self>,
    ) {
    }

    /// Render controls for sharing the pane contents. The controls shown depend on the current
    /// user's access level on the contents.
    pub fn render_sharing_controls(
        &self,
        element: &mut impl ParentElement,
        appearance: &Appearance,
        icon_color_override: Option<Fill>,
        button_size_override: Option<f32>,
        app: &AppContext,
    ) {
        let _ = (
            element,
            appearance,
            icon_color_override,
            button_size_override,
            app,
        );
    }
}
