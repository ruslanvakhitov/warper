use std::path::{Path, PathBuf};

use futures::{future::BoxFuture, FutureExt};
use warpui::{Entity, EntityId, ModelContext, ModelHandle, SingletonEntity};

use crate::{
    ai::{
        agent::{
            AIAgentAction, AIAgentActionResultType, AIAgentActionType, ReadFilesRequest,
            ReadFilesResult,
        },
        blocklist::BlocklistAIPermissions,
        paths::host_native_absolute_path,
    },
    terminal::model::session::{active_session::ActiveSession, SessionType},
};

use super::{
    read_local_file_context, ActionExecution, AnyActionExecution, ExecuteActionInput,
    PreprocessActionInput,
};

pub struct ReadFilesExecutor {
    active_session: ModelHandle<ActiveSession>,
    terminal_view_id: EntityId,
}

impl ReadFilesExecutor {
    pub fn new(active_session: ModelHandle<ActiveSession>, terminal_view_id: EntityId) -> Self {
        Self {
            active_session,
            terminal_view_id,
        }
    }

    pub(super) fn should_autoexecute(
        &self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        let ExecuteActionInput {
            action:
                AIAgentAction {
                    action: AIAgentActionType::ReadFiles(ReadFilesRequest { locations }),
                    ..
                },
            conversation_id,
        } = input
        else {
            return false;
        };

        // TODO: figure out how to avoid constructing the full paths in `should_execute`
        // and then again in `execute`, and then again on every render.
        let current_working_directory = self
            .active_session
            .as_ref(ctx)
            .current_working_directory()
            .cloned();
        let shell = self.active_session.as_ref(ctx).shell_launch_data(ctx);

        BlocklistAIPermissions::as_ref(ctx)
            .can_read_files_with_conversation(
                &conversation_id,
                locations
                    .iter()
                    .map(|file| {
                        PathBuf::from(host_native_absolute_path(
                            &file.name,
                            &shell,
                            &current_working_directory,
                        ))
                    })
                    .collect(),
                Some(self.terminal_view_id),
                ctx,
            )
            .is_allowed()
    }

    pub(super) fn execute(
        &mut self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> impl Into<AnyActionExecution> {
        let ExecuteActionInput {
            action,
            conversation_id,
            ..
        } = input;
        let AIAgentAction {
            action: AIAgentActionType::ReadFiles(ReadFilesRequest { locations }),
            ..
        } = action
        else {
            return ActionExecution::InvalidAction;
        };

        BlocklistAIPermissions::handle(ctx).update(ctx, |model, _ctx| {
            model.add_temporary_file_read_permissions(
                conversation_id,
                locations.iter().map(|file| Path::new(&file.name)),
            );
        });

        let current_working_directory = self
            .active_session
            .as_ref(ctx)
            .current_working_directory()
            .cloned();
        let shell = self.active_session.as_ref(ctx).shell_launch_data(ctx);

        let locations = locations.clone();

        let session_type = self.active_session.as_ref(ctx).session_type(ctx);

        if matches!(session_type, Some(SessionType::WarpifiedRemote { .. })) {
            return ActionExecution::Sync(AIAgentActionResultType::ReadFiles(
                ReadFilesResult::Error(
                    "The file read/edit tool is not available on this remote session. \
                     Try using a different tool."
                        .to_string(),
                ),
            ));
        }

        // Local path.
        ActionExecution::Async {
            execute_future: Box::pin(async move {
                let result = read_local_file_context(
                    &locations,
                    current_working_directory,
                    shell,
                    None,
                    None,
                )
                .await?;
                if result.missing_files.is_empty() {
                    Ok(ReadFilesResult::Success {
                        files: result.file_contexts,
                    })
                } else {
                    let missing_files = result.missing_files.join(", ");
                    Ok(ReadFilesResult::Error(format!(
                        "These files do not exist: {missing_files}"
                    )))
                }
            }),
            on_complete: Box::new(|res: Result<ReadFilesResult, anyhow::Error>, _ctx| {
                let action_result = res.unwrap_or_else(|e| ReadFilesResult::Error(e.to_string()));
                AIAgentActionResultType::ReadFiles(action_result)
            }),
        }
    }

    pub(super) fn preprocess_action(
        &mut self,
        _input: PreprocessActionInput,
        _ctx: &mut ModelContext<Self>,
    ) -> BoxFuture<'static, ()> {
        futures::future::ready(()).boxed()
    }
}

impl Entity for ReadFilesExecutor {
    type Event = ();
}
