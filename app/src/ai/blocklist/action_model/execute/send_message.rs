use futures::{future::BoxFuture, FutureExt};
use warpui::{Entity, ModelContext, SingletonEntity};

use crate::ai::agent::{
    AIAgentAction, AIAgentActionResultType, AIAgentActionType, SendMessageToAgentResult,
};
use crate::ai::blocklist::orchestration_events::{OrchestrationEventService, SendMessageResult};

use super::{ActionExecution, AnyActionExecution, ExecuteActionInput, PreprocessActionInput};

pub struct SendMessageToAgentExecutor;

impl SendMessageToAgentExecutor {
    pub fn new() -> Self {
        Self
    }

    pub(super) fn should_autoexecute(
        &self,
        _input: ExecuteActionInput,
        _ctx: &mut ModelContext<Self>,
    ) -> bool {
        true
    }

    pub(super) fn execute(
        &mut self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> AnyActionExecution {
        let AIAgentAction {
            action:
                AIAgentActionType::SendMessageToAgent {
                    addresses,
                    subject,
                    message,
                },
            ..
        } = input.action
        else {
            return ActionExecution::<()>::InvalidAction.into();
        };

        let conversation_id = input.conversation_id;
        let addresses = addresses.clone();
        let subject = subject.clone();
        let message_body = message.clone();

        let result = OrchestrationEventService::handle(ctx).update(ctx, |svc, ctx| {
            svc.send_message(conversation_id, &addresses, subject, message_body, ctx)
        });
        let result = match result {
            SendMessageResult::MessageSent { message_id } => {
                SendMessageToAgentResult::Success { message_id }
            }
            SendMessageResult::Error(error) => SendMessageToAgentResult::Error(error),
        };

        ActionExecution::<()>::Sync(AIAgentActionResultType::SendMessageToAgent(result)).into()
    }

    pub(super) fn preprocess_action(
        &mut self,
        _action: PreprocessActionInput,
        _ctx: &mut ModelContext<Self>,
    ) -> BoxFuture<'static, ()> {
        futures::future::ready(()).boxed()
    }
}

impl Default for SendMessageToAgentExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for SendMessageToAgentExecutor {
    type Event = ();
}
