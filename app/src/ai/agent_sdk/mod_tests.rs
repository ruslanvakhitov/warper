use serde_json::json;
use warp_cli::{
    agent::{HiddenComputerUseArgs, PromptArg, RunAgentArgs},
    CliCommand,
};
use warp_core::telemetry::TelemetryEvent;

use super::command_to_telemetry_event;

#[test]
fn agent_run_telemetry_is_local_only() {
    let event = command_to_telemetry_event(&CliCommand::Agent(warp_cli::agent::AgentCommand::Run(
        RunAgentArgs {
            prompt_arg: PromptArg {
                prompt: Some("do local work".to_string()),
                saved_prompt: None,
            },
            model: Default::default(),
            config_file: Default::default(),
            skill: None,
            name: None,
            cwd: None,
            gui: false,
            mcp_specs: vec![],
            mcp_servers: vec![],
            idle_on_complete: None,
            bedrock_inference_role: None,
            computer_use: HiddenComputerUseArgs::default(),
            profile: None,
        },
    )));

    assert_eq!(
        event.payload(),
        Some(json!({
            "gui": false,
            "requested_mcp_servers": 0,
            "has_environment": false,
            "task_id": null,
            "harness": "local",
        }))
    );
}
