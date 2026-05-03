use crate::server::ids::HashedSqliteId;
use warpui::{elements::Empty, AppContext, Element, Entity, TypedActionView, View, ViewContext};

#[derive(Clone, Debug)]
pub enum ImportModalAction {
    Close,
}

#[derive(Clone, Debug)]
pub enum ImportModalEvent {
    OpenTargetWithHashedId(HashedSqliteId),
    Close,
}

pub struct ImportModal;

impl ImportModal {
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self {
        Self
    }
}

impl Entity for ImportModal {
    type Event = ImportModalEvent;
}

impl TypedActionView for ImportModal {
    type Action = ImportModalAction;

    fn handle_action(&mut self, _action: &Self::Action, ctx: &mut ViewContext<Self>) {
        ctx.emit(ImportModalEvent::Close);
    }
}

impl View for ImportModal {
    fn ui_name() -> &'static str {
        "ImportModal"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}
