use serde_json::json;
use warp_cli::{
    agent::{HiddenComputerUseArgs, PromptArg, RunAgentArgs},
    artifact::{ArtifactCommand, UploadArtifactArgs},
    CliCommand,
};
use warp_core::telemetry::TelemetryEvent;

use super::{command_to_telemetry_event, telemetry::CliTelemetryEvent};

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

#[test]
fn artifact_upload_telemetry_still_maps() {
    let event = command_to_telemetry_event(&CliCommand::Artifact(ArtifactCommand::Upload(
        UploadArtifactArgs {
            path: "artifact.txt".into(),
            run_id: Some("run-123".to_string()),
            conversation_id: None,
            description: None,
        },
    )));

    assert!(matches!(event, CliTelemetryEvent::ArtifactUpload));
}
