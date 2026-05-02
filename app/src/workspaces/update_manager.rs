use super::workspace::WorkspaceUid;
use crate::cloud_object::CloudObjectEventEntrypoint;
use crate::persistence::ModelEvent;
use crate::server::ids::ServerId;
#[cfg(test)]
use crate::server::server_api::team::TeamClient;
use crate::workspaces::user_workspaces::UserWorkspaces;
use futures::channel::oneshot::{self, Receiver};
use std::sync::mpsc::SyncSender;
#[cfg(test)]
use std::sync::Arc;
use warpui::{Entity, ModelContext, SingletonEntity};

pub enum TeamUpdateManagerEvent {
    LeaveSuccess,
    LeaveError,
    RenameTeamSuccess,
    RenameTeamError,
}

/// Inert compatibility shell for removed hosted workspace update runtime.
pub struct TeamUpdateManager {
    model_event_sender: Option<SyncSender<ModelEvent>>,
}

impl TeamUpdateManager {
    #[cfg(test)]
    pub fn new(
        _team_client: Arc<dyn TeamClient>,
        model_event_sender: Option<SyncSender<ModelEvent>>,
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        Self { model_event_sender }
    }

    #[cfg(test)]
    pub fn mock(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            model_event_sender: None,
        }
    }

    pub fn refresh_workspace_metadata(&mut self, _ctx: &mut ModelContext<Self>) -> Receiver<()> {
        let (tx, rx) = oneshot::channel::<()>();
        let _ = tx.send(());
        rx
    }

    pub fn stop_polling_for_workspace_metadata_updates(&mut self) {}

    pub fn create_team(
        &mut self,
        _team_name: String,
        _entrypoint: CloudObjectEventEntrypoint,
        _discoverable: Option<bool>,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn leave_team(
        &mut self,
        _team_uid: ServerId,
        _entrypoint: CloudObjectEventEntrypoint,
        ctx: &mut ModelContext<Self>,
    ) {
        ctx.emit(TeamUpdateManagerEvent::LeaveError);
    }

    pub fn rename_team(&mut self, _new_name: String, ctx: &mut ModelContext<Self>) {
        ctx.emit(TeamUpdateManagerEvent::RenameTeamError);
        ctx.notify();
    }

    pub fn set_current_workspace_uid(
        &mut self,
        workspace_uid: WorkspaceUid,
        ctx: &mut ModelContext<Self>,
    ) {
        UserWorkspaces::handle(ctx).update(ctx, |user_workspaces, ctx| {
            user_workspaces.set_current_workspace_uid(workspace_uid, ctx);
        });

        self.save_to_db([ModelEvent::SetCurrentWorkspace { workspace_uid }]);
    }

    fn save_to_db(&self, events: impl IntoIterator<Item = ModelEvent>) {
        if let Some(model_event_sender) = &self.model_event_sender {
            for event in events {
                if let Err(err) = model_event_sender.send(event) {
                    log::warn!("Unable to save workspace metadata to sqlite: {err}");
                }
            }
        }
    }
}

impl Entity for TeamUpdateManager {
    type Event = TeamUpdateManagerEvent;
}

impl SingletonEntity for TeamUpdateManager {}
