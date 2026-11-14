use crate::ai::agent::redaction;
use futures_util::StreamExt;

use super::{openrouter, ConvertToAPITypeError, RequestParams, ResponseStream};

pub async fn generate_multi_agent_output(
    mut params: RequestParams,
    cancellation_rx: futures::channel::oneshot::Receiver<()>,
) -> Result<ResponseStream, ConvertToAPITypeError> {
    if params.should_redact_secrets {
        redaction::redact_inputs(&mut params.input);
    }

    let output_stream = openrouter::generate_openrouter_output(params).take_until(cancellation_rx);
    Ok(Box::pin(output_stream))
}
