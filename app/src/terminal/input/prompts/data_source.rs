use crate::search::data_source::{Query, QueryResult};
use crate::search::mixer::DataSourceRunErrorWrapper;
use crate::search::SyncDataSource;
use crate::terminal::input::inline_menu::{
    default_navigation_message_items, InlineMenuAction, InlineMenuMessageArgs, InlineMenuType,
};
use crate::terminal::input::message_bar::Message;
use warp_server_client::ids::SyncId;
use warpui::{AppContext, Entity, ModelContext};

#[derive(Clone, Debug)]
pub struct AcceptPrompt {
    pub id: SyncId,
}

impl InlineMenuAction for AcceptPrompt {
    const MENU_TYPE: InlineMenuType = InlineMenuType::PromptsMenu;

    fn produce_inline_menu_message<T>(args: InlineMenuMessageArgs<'_, Self, T>) -> Option<Message> {
        Some(Message::new(default_navigation_message_items(&args)))
    }
}

pub struct PromptsMenuDataSource {}

impl PromptsMenuDataSource {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {}
    }
}

impl SyncDataSource for PromptsMenuDataSource {
    type Action = AcceptPrompt;

    fn run_query(
        &self,
        query: &Query,
        app: &AppContext,
    ) -> Result<Vec<QueryResult<Self::Action>>, DataSourceRunErrorWrapper> {
        let _ = (query, app);
        Ok(Vec::new())
    }
}

impl Entity for PromptsMenuDataSource {
    type Event = ();
}
