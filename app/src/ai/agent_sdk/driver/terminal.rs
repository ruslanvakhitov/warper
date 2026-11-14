use std::{
    collections::HashMap,
    ffi::OsString,
    future::Future,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures::channel::oneshot;
use warp_core::command::ExitCode;
use warpui::{r#async::FutureExt, AppContext, Entity, ModelContext, ModelHandle, ViewHandle};

use crate::{
    pane_group::NewTerminalOptions,
    root_view::{open_new_with_workspace_source, NewWorkspaceSource},
    terminal::{model::block::BlockId, view::ConversationRestorationInNewPaneType, TerminalView},
    util::sync::Condition,
};

use super::AgentDriverError;

const TERMINAL_SESSION_BOOTSTRAP_TIMEOUT: Duration = Duration::from_secs(60);

/// Options for creating the terminal view before constructing a [`TerminalDriver`].
pub(crate) struct TerminalDriverOptions {
    pub working_dir: PathBuf,
    pub env_vars: HashMap<OsString, OsString>,
    pub conversation_restoration: Option<ConversationRestorationInNewPaneType>,
}

/// Events emitted by [`TerminalDriver`] for [`super::AgentDriver`] to react to.
pub(crate) enum TerminalDriverEvent {
    /// Terminal bootstrap is taking unusually long.
    SlowBootstrap,
}

/// Manages the terminal session lifecycle for the agent driver.
///
/// Responsibilities:
/// - Monitoring for terminal bootstrapping to be done
/// - Executing commands in the session
/// - Detecting block completion
pub(crate) struct TerminalDriver {
    terminal_view: ViewHandle<TerminalView>,
    session_bootstrapped: Condition,
    waiting_command: Option<oneshot::Sender<ExitCode>>,

    /// State for the pending command we're expecting to start executing.
    /// The `String` is the expected command text, and the sender is used
    /// to send the block ID to the waiting caller.
    pending_command_start: Option<(String, oneshot::Sender<BlockId>)>,
}

impl Entity for TerminalDriver {
    type Event = TerminalDriverEvent;
}

/// Create the terminal window and extract the [`ViewHandle<TerminalView>`].
///
/// This is separate from [`TerminalDriver::new`] because [`AppContext::add_model`]
/// requires an infallible constructor; the fallible window/view creation must happen first.
fn create_terminal_view(
    options: TerminalDriverOptions,
    ctx: &mut AppContext,
) -> Result<ViewHandle<TerminalView>, AgentDriverError> {
    let (_, root_view) = open_new_with_workspace_source(
        NewWorkspaceSource::Session {
            options: Box::new(NewTerminalOptions {
                initial_directory: Some(options.working_dir),
                env_vars: options.env_vars,
                conversation_restoration: options.conversation_restoration,
                ..Default::default()
            }),
        },
        ctx,
    );

    root_view
        .as_ref(ctx)
        .workspace_view()
        .ok_or(AgentDriverError::TerminalUnavailable)?
        .as_ref(ctx)
        .active_tab_pane_group()
        .as_ref(ctx)
        .active_session_view(ctx)
        .ok_or(AgentDriverError::TerminalUnavailable)
}

impl TerminalDriver {
    /// Create a terminal view from the given options and wrap it in a new `TerminalDriver` model.
    pub(crate) fn create(
        options: TerminalDriverOptions,
        ctx: &mut AppContext,
    ) -> Result<ModelHandle<Self>, AgentDriverError> {
        let working_dir = options.working_dir.clone();
        let terminal_view = create_terminal_view(options, ctx)?;
        Ok(ctx.add_model(|ctx| Self::new(terminal_view, working_dir, ctx)))
    }

    /// Set up event subscriptions for an already-created terminal view.
    fn new(
        terminal_view: ViewHandle<TerminalView>,
        _working_dir: PathBuf,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        let session_bootstrapped = Condition::new();

        ctx.subscribe_to_view(&terminal_view, move |me, event, ctx| {
            me.handle_terminal_view_event(event, ctx);
        });

        // If the session already bootstrapped before we subscribed, set the
        // condition immediately so callers of `wait_for_session_bootstrapped`
        // don't block forever.
        let already_bootstrapped = terminal_view.read(ctx, |terminal, _| {
            terminal
                .model
                .lock()
                .block_list()
                .is_bootstrapping_precmd_done()
        });
        if already_bootstrapped {
            session_bootstrapped.set();
        }

        Self {
            terminal_view,
            session_bootstrapped,
            waiting_command: None,
            pending_command_start: None,
        }
    }

    /// Get a handle to the backing terminal view.
    pub fn terminal_view(&self) -> &ViewHandle<TerminalView> {
        &self.terminal_view
    }

    /// Submit `text` to the active CLI agent on the terminal PTY using the
    /// agent-specific submission strategy.
    ///
    /// Used to send exit commands to third-party harnesses.
    pub(super) fn send_text_to_cli(&self, text: String, ctx: &mut ModelContext<Self>) {
        self.terminal_view.update(ctx, |terminal, ctx| {
            terminal.submit_text_to_cli_agent_pty(text, ctx);
        });
    }

    /// Execute a command in the terminal and return a future that resolves to a
    /// [`CommandHandle`] once the command starts executing.
    pub fn execute_command(
        &mut self,
        command: &str,
        ctx: &mut ModelContext<Self>,
    ) -> Result<impl Future<Output = Result<CommandHandle, AgentDriverError>>, AgentDriverError>
    {
        let (exit_tx, exit_rx) = oneshot::channel::<ExitCode>();
        let (start_tx, start_rx) = oneshot::channel::<BlockId>();

        // We should not be able to execute a command while we are still waiting on another one.
        // This is enforced by the caller by waiting on rx before continuing.
        if self.waiting_command.is_some() || self.pending_command_start.is_some() {
            return Err(AgentDriverError::InvalidRuntimeState);
        }

        let command_string = command.to_string();
        self.terminal_view.update(ctx, |terminal, ctx| {
            self.waiting_command = Some(exit_tx);
            self.pending_command_start = Some((command_string, start_tx));
            terminal.execute_command_or_set_pending(command, ctx);
        });

        Ok(async move {
            let block_id = start_rx
                .await
                .map_err(|_| AgentDriverError::InvalidRuntimeState)?;
            Ok(CommandHandle {
                exit_status_rx: exit_rx,
                block_id,
            })
        })
    }

    /// Returns a future that resolves when the session has bootstrapped.
    ///
    /// This only waits for the `SessionBootstrapped` terminal view event.
    pub fn wait_for_session_bootstrapped(
        &self,
    ) -> impl Future<Output = Result<(), AgentDriverError>> {
        let session_bootstrapped = self.session_bootstrapped.clone();

        async move {
            session_bootstrapped
                .wait()
                .with_timeout(TERMINAL_SESSION_BOOTSTRAP_TIMEOUT)
                .await
                .map_err(|_| {
                    log::error!("Timed out waiting for session bootstrap");
                    AgentDriverError::BootstrapFailed
                })
        }
    }
}

/// A handle to a running terminal command.
///
/// Resolves to the command's [`ExitCode`] when the block completes.
/// Also carries the [`BlockId`] for local process bookkeeping after completion.
pub(crate) struct CommandHandle {
    exit_status_rx: oneshot::Receiver<ExitCode>,
    block_id: BlockId,
}

impl CommandHandle {
    /// The block ID of the command that was executed.
    pub fn block_id(&self) -> &BlockId {
        &self.block_id
    }
}

impl Future for CommandHandle {
    type Output = Result<ExitCode, AgentDriverError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.exit_status_rx)
            .poll(cx)
            .map(|result| result.map_err(|_| AgentDriverError::InvalidRuntimeState))
    }
}

impl TerminalDriver {
    /// Handle terminal view events.
    fn handle_terminal_view_event(
        &mut self,
        event: &crate::terminal::view::Event,
        ctx: &mut ModelContext<Self>,
    ) {
        match event {
            crate::terminal::view::Event::SessionBootstrapped => {
                self.session_bootstrapped.set();
            }
            crate::terminal::view::Event::SlowBootstrap => {
                ctx.emit(TerminalDriverEvent::SlowBootstrap);
            }
            crate::terminal::view::Event::ExecuteCommand(event) => {
                if let Some((_expected_command, sender)) = self
                    .pending_command_start
                    .take_if(|(cmd, _)| *cmd == event.command)
                {
                    let block_id = self.terminal_view.read(ctx, |terminal, _| {
                        terminal.model.lock().block_list().active_block_id().clone()
                    });
                    let _ = sender.send(block_id);
                }
            }
            crate::terminal::view::Event::BlockCompleted { block, .. } => {
                if let Some(sender) = self.waiting_command.take_if(|_| {
                    let bootstrapping_done = self.terminal_view.read(ctx, |terminal, _| {
                        terminal
                            .model
                            .lock()
                            .block_list()
                            .is_bootstrapping_precmd_done()
                    });
                    // This was originally checking `bootstrapping_done && block.did_execute`.
                    // Oddly, we've seen cases where we missed the preexec hook, so
                    // `block.did_execute` is false even though the command actually did run.
                    // To hedge against this while we're still figuring out the root cause,
                    // we instead simply make sure it was not a background block.
                    bootstrapping_done && !block.is_background
                }) {
                    let _ = sender.send(block.exit_code);
                }
            }
            _ => (),
        }
    }
}
