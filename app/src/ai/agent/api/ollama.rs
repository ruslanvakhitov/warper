use async_stream::stream;
use chrono::Utc;
use http_client::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;
use warp_multi_agent_api::{
    self as api, client_action, message, response_event, response_event::stream_finished,
};

use crate::ai::agent::AIAgentInput;

use super::openrouter::command_looks_read_only;
use super::{RequestParams, ResponseStream};

const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434";
const DEFAULT_OLLAMA_MODEL_ID: &str = "llama3.1";
const OLLAMA_KEEP_ALIVE: &str = "5m";
const OLLAMA_REQUEST_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Clone, Debug, Serialize)]
struct OllamaMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    tools: Vec<OllamaTool>,
    stream: bool,
    keep_alive: &'static str,
}

#[derive(Debug, Serialize)]
struct OllamaTool {
    #[serde(rename = "type")]
    kind: &'static str,
    function: OllamaToolFunction,
}

#[derive(Debug, Serialize)]
struct OllamaToolFunction {
    name: &'static str,
    description: &'static str,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    #[serde(default)]
    message: Option<OllamaAssistantMessage>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaAssistantMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<OllamaToolCall>,
}

#[derive(Debug, Deserialize)]
struct OllamaToolCall {
    #[serde(default)]
    id: Option<String>,
    function: OllamaFunctionCall,
}

#[derive(Debug, Deserialize)]
struct OllamaFunctionCall {
    name: String,
    /// Ollama returns arguments as a JSON object for native tool calls and as a
    /// string for OpenAI-compat shims. Accept either, then normalize downstream.
    #[serde(default)]
    arguments: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct RunShellCommandArgs {
    command: String,
    #[serde(default)]
    is_read_only: Option<bool>,
}

#[derive(Debug)]
enum OllamaError {
    MissingBaseUrl,
    Request(String),
    Status { status: StatusCode, body: String },
    Response(String),
    EmptyResponse,
}

impl OllamaError {
    fn user_message(&self) -> String {
        match self {
            Self::MissingBaseUrl => {
                "Ollama base URL is missing. Add it in Settings > Ollama (e.g. http://localhost:11434), then try again."
                    .to_owned()
            }
            Self::Status { status, body } => {
                let body = body.trim();
                if body.is_empty() {
                    format!("Ollama request failed with status {status}.")
                } else {
                    format!("Ollama request failed with status {status}:\n{body}")
                }
            }
            Self::Request(error) => format!("Ollama request failed: {error}"),
            Self::Response(error) => format!("Ollama response could not be read: {error}"),
            Self::EmptyResponse => "Ollama returned an empty response.".to_owned(),
        }
    }

    fn finish_reason(&self) -> stream_finished::Reason {
        stream_finished::Reason::InternalError(stream_finished::InternalError {
            message: self.user_message(),
        })
    }
}

pub fn generate_ollama_output(params: RequestParams) -> ResponseStream {
    Box::pin(stream! {
        let task_id = params.primary_task_id.clone();
        let model_id = effective_ollama_model(&params);
        let request_id = Uuid::new_v4().to_string();
        let conversation_id = params
            .conversation_token
            .as_ref()
            .map(|token| token.as_str().to_owned())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        yield Ok(stream_init_event(
            conversation_id.clone(),
            request_id.clone(),
        ));
        if should_create_ollama_task(&params, &task_id) {
            yield Ok(create_task_event(&task_id));
        }

        let result = request_ollama_completion(&params, &model_id).await;
        match result {
            Ok(completion) => {
                yield Ok(add_messages_event(
                    task_id,
                    request_id,
                    model_id.clone(),
                    completion.text,
                    completion.tool_calls,
                ));
                yield Ok(done_event(completion.usage, model_id));
            }
            Err(error) => {
                yield Ok(add_messages_event(
                    task_id,
                    request_id,
                    model_id,
                    Some(error.user_message()),
                    Vec::new(),
                ));
                yield Ok(finished_event(error.finish_reason()));
            }
        }
    })
}

async fn request_ollama_completion(
    params: &RequestParams,
    model_id: &str,
) -> Result<OllamaCompletion, OllamaError> {
    let base_url = params
        .ollama_base_url
        .as_deref()
        .map(normalize_base_url)
        .unwrap_or_else(|| DEFAULT_OLLAMA_BASE_URL.to_owned());

    if base_url.is_empty() {
        return Err(OllamaError::MissingBaseUrl);
    }

    let url = format!("{base_url}/api/chat");

    let request = OllamaChatRequest {
        model: model_id.to_owned(),
        messages: build_ollama_messages(params),
        tools: ollama_tools(),
        stream: false,
        keep_alive: OLLAMA_KEEP_ALIVE,
    };

    log::info!(
        "Ollama request starting: url={url}, model={model_id}, messages={}, tools={}, task_id={}, timeout_secs={}",
        request.messages.len(),
        request.tools.len(),
        params.primary_task_id,
        OLLAMA_REQUEST_TIMEOUT.as_secs(),
    );

    let client = http_client::Client::new();
    let mut builder = client
        .post(&url)
        .json(&request)
        .timeout(OLLAMA_REQUEST_TIMEOUT)
        .prevent_sleep("Ollama agent request in-progress");

    if let Some(api_key) = params
        .ollama_api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        builder = builder.bearer_auth(api_key);
    }

    let response = builder.send().await.map_err(|error| {
        log::warn!("Ollama request transport failed: url={url}, model={model_id}, error={error}");
        OllamaError::Request(error.to_string())
    })?;

    let status = response.status();
    let body = response.text().await.map_err(|error| {
        log::warn!("Ollama response body read failed: model={model_id}, error={error}");
        OllamaError::Response(error.to_string())
    })?;

    log::info!(
        "Ollama response received: model={model_id}, status={status}, body_bytes={}",
        body.len(),
    );

    if !status.is_success() {
        log::warn!(
            "Ollama request failed: model={model_id}, status={status}, body={}",
            truncate_for_log(&body),
        );
        return Err(OllamaError::Status { status, body });
    }

    let parsed: OllamaChatResponse = serde_json::from_str(&body).map_err(|error| {
        log::warn!(
            "Ollama response parse failed: model={model_id}, error={error}, body={}",
            truncate_for_log(&body),
        );
        OllamaError::Response(error.to_string())
    })?;

    let assistant = parsed.message.ok_or(OllamaError::EmptyResponse)?;
    let raw_text = assistant
        .content
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty());
    let mut tool_calls = assistant.tool_calls;

    // Some Ollama-served models (Qwen base coder variants, certain Llama 3
    // quants) emit tool calls as text instead of populating message.tool_calls.
    // Recover those so they execute as real tool calls.
    let text = if tool_calls.is_empty() {
        if let Some(text) = raw_text {
            let (cleaned, recovered) = extract_text_mode_tool_calls(&text);
            tool_calls.extend(recovered);
            cleaned
        } else {
            None
        }
    } else {
        raw_text
    };

    if text.is_none() && tool_calls.is_empty() {
        return Err(OllamaError::EmptyResponse);
    }

    Ok(OllamaCompletion {
        text,
        tool_calls,
        usage: OllamaUsage {
            prompt_tokens: parsed.prompt_eval_count,
            completion_tokens: parsed.eval_count,
        },
    })
}

/// Scan assistant text for tool-call JSON the model emitted as plain text
/// instead of via `message.tool_calls`. Recognizes three shapes, in order:
///
/// 1. `<tool_call>{...}</tool_call>` blocks (Qwen-style).
/// 2. Triple-backtick fenced code blocks (` ```json {...}``` ` or just
///    ` ```{...}``` `).
/// 3. The entire trimmed text being a single JSON object.
///
/// Returns the cleaned text (with recognized tool-call regions stripped) and
/// any recovered tool calls.
fn extract_text_mode_tool_calls(text: &str) -> (Option<String>, Vec<OllamaToolCall>) {
    let mut tool_calls = Vec::new();

    // (1) <tool_call>...</tool_call> blocks.
    let stripped = strip_blocks(text, "<tool_call>", "</tool_call>", &mut tool_calls);
    if !tool_calls.is_empty() {
        return (non_empty(stripped.trim().to_owned()), tool_calls);
    }

    // (2) Fenced code blocks.
    let stripped = strip_fenced_blocks(text, &mut tool_calls);
    if !tool_calls.is_empty() {
        return (non_empty(stripped.trim().to_owned()), tool_calls);
    }

    // (3) The whole trimmed text as a JSON object.
    if let Some(tc) = parse_text_tool_call(text.trim()) {
        return (None, vec![tc]);
    }

    (Some(text.to_owned()), Vec::new())
}

fn strip_blocks(
    text: &str,
    open: &str,
    close: &str,
    tool_calls: &mut Vec<OllamaToolCall>,
) -> String {
    let mut out = String::new();
    let mut cursor = 0;
    while let Some(rel_start) = text[cursor..].find(open) {
        let abs_start = cursor + rel_start;
        out.push_str(&text[cursor..abs_start]);
        let inner_start = abs_start + open.len();
        match text[inner_start..].find(close) {
            Some(rel_end) => {
                let inner = text[inner_start..inner_start + rel_end].trim();
                if let Some(tc) = parse_text_tool_call(inner) {
                    tool_calls.push(tc);
                } else {
                    // Couldn't parse — preserve the block verbatim so the user
                    // sees what the model said.
                    out.push_str(&text[abs_start..inner_start + rel_end + close.len()]);
                }
                cursor = inner_start + rel_end + close.len();
            }
            None => {
                // Unterminated open tag — keep the rest as-is.
                out.push_str(&text[abs_start..]);
                return out;
            }
        }
    }
    out.push_str(&text[cursor..]);
    out
}

fn strip_fenced_blocks(text: &str, tool_calls: &mut Vec<OllamaToolCall>) -> String {
    let mut out = String::new();
    let mut cursor = 0;
    while let Some(rel_start) = text[cursor..].find("```") {
        let abs_start = cursor + rel_start;
        out.push_str(&text[cursor..abs_start]);
        let after_open = abs_start + 3;
        // Skip an optional language tag on the same line.
        let body_start = match text[after_open..].find('\n') {
            Some(nl) => after_open + nl + 1,
            None => {
                out.push_str(&text[abs_start..]);
                return out;
            }
        };
        match text[body_start..].find("```") {
            Some(rel_end) => {
                let inner = text[body_start..body_start + rel_end].trim();
                if let Some(tc) = parse_text_tool_call(inner) {
                    tool_calls.push(tc);
                } else {
                    out.push_str(&text[abs_start..body_start + rel_end + 3]);
                }
                cursor = body_start + rel_end + 3;
            }
            None => {
                out.push_str(&text[abs_start..]);
                return out;
            }
        }
    }
    out.push_str(&text[cursor..]);
    out
}

fn parse_text_tool_call(candidate: &str) -> Option<OllamaToolCall> {
    if !candidate.starts_with('{') {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(candidate).ok()?;
    let obj = value.as_object()?;
    let name = obj.get("name")?.as_str()?.to_owned();
    if name != "run_shell_command" {
        return None;
    }
    let arguments = obj
        .get("arguments")
        .or_else(|| obj.get("parameters"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    Some(OllamaToolCall {
        id: None,
        function: OllamaFunctionCall { name, arguments },
    })
}

fn non_empty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn ollama_tools() -> Vec<OllamaTool> {
    vec![OllamaTool {
        kind: "function",
        function: OllamaToolFunction {
            name: "run_shell_command",
            description: "Run a shell command in the user's active terminal session. Use this for commands that inspect the project, run tests, or perform an explicit change requested by the user.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The exact shell command to run."
                    },
                    "is_read_only": {
                        "type": "boolean",
                        "description": "True when the command only reads or inspects state."
                    }
                },
                "required": ["command"]
            }),
        },
    }]
}

fn ollama_tool_call_to_message(
    task_id: &str,
    request_id: &str,
    tool_call: OllamaToolCall,
    timestamp: Option<prost_types::Timestamp>,
) -> Option<api::Message> {
    if tool_call.function.name != "run_shell_command" {
        return None;
    }

    let args = parse_run_shell_command_args(&tool_call.function.arguments)?;
    let command = args.command.trim().to_owned();
    if command.is_empty() {
        return None;
    }

    let is_read_only = args
        .is_read_only
        .unwrap_or_else(|| command_looks_read_only(&command));
    let is_risky = !is_read_only;
    let tool_call_id = tool_call.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    Some(api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        request_id: request_id.to_owned(),
        timestamp,
        server_message_data: String::new(),
        citations: vec![],
        message: Some(message::Message::ToolCall(api::message::ToolCall {
            tool_call_id,
            tool: Some(api::message::tool_call::Tool::RunShellCommand(
                api::message::tool_call::RunShellCommand {
                    command,
                    is_read_only,
                    uses_pager: false,
                    citations: vec![],
                    is_risky,
                    wait_until_complete_value: None,
                    risk_category: 0,
                },
            )),
        })),
    })
}

fn parse_run_shell_command_args(arguments: &serde_json::Value) -> Option<RunShellCommandArgs> {
    match arguments {
        serde_json::Value::String(text) => serde_json::from_str(text).ok(),
        serde_json::Value::Object(_) => serde_json::from_value(arguments.clone()).ok(),
        _ => None,
    }
}

fn effective_ollama_model(params: &RequestParams) -> String {
    params
        .ollama_model
        .as_ref()
        .map(|model| model.trim())
        .filter(|model| !model.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            let selected = params.model.as_str();
            if selected == "auto" || selected.is_empty() {
                DEFAULT_OLLAMA_MODEL_ID.to_owned()
            } else {
                selected.to_owned()
            }
        })
}

fn normalize_base_url(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_owned()
    } else {
        format!("http://{trimmed}")
    }
}

fn build_ollama_messages(params: &RequestParams) -> Vec<OllamaMessage> {
    let mut system = "You are Warper, an AI agent inside an agentic terminal, running against a local Ollama model. Be concise, practical, and terminal-aware. Use the run_shell_command tool when you need command output or when the user asks you to run something. Prefer read-only inspection commands before making changes. Do not claim that you ran a command unless you used the tool and saw the result.".to_owned();

    if let Some(cwd) = params
        .session_context
        .current_working_directory()
        .as_deref()
    {
        system.push_str("\nCurrent working directory: ");
        system.push_str(cwd);
    }

    let mut user_content = params
        .input
        .iter()
        .map(input_to_prompt_text)
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    if user_content.trim().is_empty() {
        user_content = "Continue the conversation.".to_owned();
    }

    vec![
        OllamaMessage {
            role: "system",
            content: system,
        },
        OllamaMessage {
            role: "user",
            content: user_content,
        },
    ]
}

fn input_to_prompt_text(input: &AIAgentInput) -> String {
    match input {
        AIAgentInput::ActionResult { result, .. } => format!("Tool result:\n{result}"),
        AIAgentInput::AutoCodeDiffQuery { query, .. } => {
            format!("Code assistance request:\n{query}")
        }
        AIAgentInput::SummarizeConversation { prompt } => {
            format!(
                "Summarize the current conversation. Additional instructions: {}",
                prompt.clone().unwrap_or_default()
            )
        }
        AIAgentInput::PassiveSuggestionResult { suggestion, .. } => {
            format!("Passive suggestion result:\n{suggestion:?}")
        }
        _ => input.user_query().unwrap_or_else(|| input.to_string()),
    }
}

fn stream_init_event(conversation_id: String, request_id: String) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(response_event::Type::Init(response_event::StreamInit {
            run_id: conversation_id.clone(),
            conversation_id,
            request_id,
        })),
    }
}

fn should_create_ollama_task(params: &RequestParams, task_id: &str) -> bool {
    !task_id.is_empty() && !params.tasks.iter().any(|task| task.id == task_id)
}

fn create_task_event(task_id: &str) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(response_event::Type::ClientActions(
            response_event::ClientActions {
                actions: vec![api::ClientAction {
                    action: Some(client_action::Action::CreateTask(
                        client_action::CreateTask {
                            task: Some(api::Task {
                                id: task_id.to_owned(),
                                messages: vec![],
                                dependencies: None,
                                description: String::new(),
                                summary: String::new(),
                                server_data: String::new(),
                            }),
                        },
                    )),
                }],
            },
        )),
    }
}

fn add_messages_event(
    task_id: String,
    request_id: String,
    model_id: String,
    text: Option<String>,
    tool_calls: Vec<OllamaToolCall>,
) -> api::ResponseEvent {
    let now = Utc::now();
    let timestamp = Some(prost_types::Timestamp {
        seconds: now.timestamp(),
        nanos: now.timestamp_subsec_nanos() as i32,
    });

    let model_used = api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.clone(),
        request_id: request_id.clone(),
        timestamp,
        server_message_data: String::new(),
        citations: vec![],
        message: Some(message::Message::ModelUsed(message::ModelUsed {
            model_id: model_id.clone(),
            model_display_name: model_id,
            is_fallback: false,
        })),
    };

    let mut messages = vec![model_used];
    if let Some(text) = text {
        messages.push(api::Message {
            id: Uuid::new_v4().to_string(),
            task_id: task_id.clone(),
            request_id: request_id.clone(),
            timestamp,
            server_message_data: String::new(),
            citations: vec![],
            message: Some(message::Message::AgentOutput(message::AgentOutput { text })),
        });
    }

    messages.extend(tool_calls.into_iter().filter_map(|tool_call| {
        ollama_tool_call_to_message(&task_id, &request_id, tool_call, timestamp)
    }));

    api::ResponseEvent {
        r#type: Some(response_event::Type::ClientActions(
            response_event::ClientActions {
                actions: vec![api::ClientAction {
                    action: Some(client_action::Action::AddMessagesToTask(
                        client_action::AddMessagesToTask { task_id, messages },
                    )),
                }],
            },
        )),
    }
}

fn done_event(usage: OllamaUsage, model_id: String) -> api::ResponseEvent {
    let token_usage = if usage.prompt_tokens.is_some() || usage.completion_tokens.is_some() {
        vec![stream_finished::TokenUsage {
            model_id,
            total_input: usage.prompt_tokens.unwrap_or_default(),
            output: usage.completion_tokens.unwrap_or_default(),
            ..Default::default()
        }]
    } else {
        Vec::new()
    };

    let mut event = finished_event(stream_finished::Reason::Done(stream_finished::Done {}));
    if let Some(response_event::Type::Finished(finished)) = &mut event.r#type {
        finished.token_usage = token_usage;
    }
    event
}

fn finished_event(reason: stream_finished::Reason) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(response_event::Type::Finished(
            response_event::StreamFinished {
                reason: Some(reason),
                ..Default::default()
            },
        )),
    }
}

struct OllamaCompletion {
    text: Option<String>,
    tool_calls: Vec<OllamaToolCall>,
    usage: OllamaUsage,
}

struct OllamaUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
}

fn truncate_for_log(body: &str) -> String {
    const MAX_LOG_CHARS: usize = 2000;
    let body = body.trim();
    if body.chars().count() <= MAX_LOG_CHARS {
        return body.to_owned();
    }

    let mut truncated = body.chars().take(MAX_LOG_CHARS).collect::<String>();
    truncated.push_str("...<truncated>");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::blocklist::SessionContext;
    use crate::ai::llms::LLMId;
    use futures_lite::{future::block_on, StreamExt};

    fn request_params_for_test() -> RequestParams {
        let model = LLMId::from("test-model");

        RequestParams {
            input: vec![],
            primary_task_id: "test-task".to_owned(),
            conversation_token: None,
            forked_from_conversation_token: None,
            ambient_agent_task_id: None,
            tasks: vec![],
            existing_suggestions: None,
            metadata: None,
            session_context: SessionContext::new_for_test(),
            model: model.clone(),
            coding_model: model.clone(),
            cli_agent_model: model.clone(),
            computer_use_model: model,
            is_memory_enabled: false,
            warp_drive_context_enabled: false,
            mcp_context: None,
            planning_enabled: true,
            should_redact_secrets: false,
            api_keys: None,
            open_router_model: None,
            ollama_base_url: None,
            ollama_api_key: None,
            ollama_model: None,
            active_provider: ::ai::api_keys::AiProvider::Ollama,
            allow_use_of_warp_credits_with_byok: false,
            autonomy_level: api::AutonomyLevel::Supervised,
            isolation_level: api::IsolationLevel::None,
            web_search_enabled: false,
            computer_use_enabled: false,
            ask_user_question_enabled: false,
            research_agent_enabled: false,
            orchestration_enabled: false,
            supported_tools_override: None,
            parent_agent_id: None,
            agent_name: None,
        }
    }

    #[test]
    fn output_emits_init_then_create_task_for_new_conversation() {
        let params = request_params_for_test();
        let events = block_on(generate_ollama_output(params).collect::<Vec<_>>());

        assert!(matches!(
            events[0].as_ref().unwrap().r#type.as_ref().unwrap(),
            response_event::Type::Init(_)
        ));

        let response_event::Type::ClientActions(create_actions) =
            events[1].as_ref().unwrap().r#type.as_ref().unwrap()
        else {
            panic!("expected CreateTask client action");
        };
        assert!(matches!(
            create_actions.actions[0].action.as_ref().unwrap(),
            client_action::Action::CreateTask(_)
        ));
    }

    #[test]
    fn normalize_base_url_handles_common_inputs() {
        assert_eq!(
            normalize_base_url("http://localhost:11434"),
            "http://localhost:11434"
        );
        assert_eq!(
            normalize_base_url("http://localhost:11434/"),
            "http://localhost:11434"
        );
        assert_eq!(
            normalize_base_url("localhost:11434"),
            "http://localhost:11434"
        );
        assert_eq!(
            normalize_base_url("https://ollama.example.com/v1/"),
            "https://ollama.example.com/v1"
        );
        assert_eq!(normalize_base_url("   "), "");
    }

    #[test]
    fn extracts_qwen_style_tool_call_block() {
        let text = "I'll run a command.\n<tool_call>\n{\"name\": \"run_shell_command\", \"arguments\": {\"command\": \"ls -la\", \"is_read_only\": true}}\n</tool_call>";
        let (cleaned, calls) = extract_text_mode_tool_calls(text);
        assert_eq!(cleaned.as_deref(), Some("I'll run a command."));
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "run_shell_command");
        let args = calls[0].function.arguments.as_object().unwrap();
        assert_eq!(args.get("command").and_then(|v| v.as_str()), Some("ls -la"));
    }

    #[test]
    fn extracts_fenced_json_tool_call() {
        let text = "Here you go:\n```json\n{\"name\": \"run_shell_command\", \"arguments\": {\"command\": \"pwd\"}}\n```\nLet me know.";
        let (cleaned, calls) = extract_text_mode_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert!(cleaned.as_deref().is_some_and(|t| t.contains("Let me know.")));
    }

    #[test]
    fn extracts_naked_json_tool_call() {
        let text = "{\"name\": \"run_shell_command\", \"arguments\": {\"command\": \"ls\"}}";
        let (cleaned, calls) = extract_text_mode_tool_calls(text);
        assert!(cleaned.is_none());
        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn ignores_unknown_tool_name() {
        let text = "{\"name\": \"some_other_tool\", \"arguments\": {}}";
        let (cleaned, calls) = extract_text_mode_tool_calls(text);
        assert_eq!(cleaned.as_deref(), Some(text));
        assert!(calls.is_empty());
    }

    #[test]
    fn parse_run_shell_command_args_accepts_object_and_string() {
        let object_args = serde_json::json!({ "command": "ls", "is_read_only": true });
        let parsed = parse_run_shell_command_args(&object_args).unwrap();
        assert_eq!(parsed.command, "ls");
        assert_eq!(parsed.is_read_only, Some(true));

        let string_args = serde_json::Value::String(r#"{"command":"pwd"}"#.to_owned());
        let parsed = parse_run_shell_command_args(&string_args).unwrap();
        assert_eq!(parsed.command, "pwd");
        assert_eq!(parsed.is_read_only, None);
    }
}
