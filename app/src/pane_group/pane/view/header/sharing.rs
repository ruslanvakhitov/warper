//! Local-only pane header sharing stubs retained for pane configuration compatibility.
//!
//! Warper does not expose hosted sharing controls.

use warpui::ViewContext;

use crate::{
    drive::sharing::ShareableObject, pane_group::BackingView,
    server::telemetry::SharingDialogSource,
};

use super::PaneHeader;

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

    pub fn is_sharing_dialog_enabled<C: warpui::ViewAsRef>(&self, ctx: &C) -> bool {
        let _ = ctx;
        false
    }

    pub fn share_pane_contents(
        &mut self,
        _source: SharingDialogSource,
        _ctx: &mut ViewContext<Self>,
    ) {
    }
}
