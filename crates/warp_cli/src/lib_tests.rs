use super::*;
use clap::Parser;
use std::ffi::OsString;

use crate::agent::AgentCommand;
use crate::provider::ProviderCommand;

fn set_env_var(name: &str, value: &str) -> Option<OsString> {
    let previous = std::env::var_os(name);
    // Safety: tests that mutate process environment are marked `serial` so we
    // do not race with other environment readers/writers in this crate.
    unsafe { std::env::set_var(name, value) };
    previous
}

fn restore_env_var(name: &str, previous: Option<OsString>) {
    match previous {
        // Safety: tests that mutate process environment are marked `serial` so
        // we do not race with other environment readers/writers in this crate.
        Some(value) => unsafe { std::env::set_var(name, value) },
        // Safety: tests that mutate process environment are marked `serial` so
        // we do not race with other environment readers/writers in this crate.
        None => unsafe { std::env::remove_var(name) },
    }
}

fn assert_parse_fails(args: &[&str]) {
    let mut argv = vec!["warp"];
    argv.extend_from_slice(args);
    assert!(
        Args::try_parse_from(argv).is_err(),
        "expected parse failure for: {args:?}"
    );
}

#[test]
fn retained_agent_run_parses_local_options() {
    let args = Args::try_parse_from([
        "warp",
        "agent",
        "run",
        "--prompt",
        "hello",
        "--model",
        "gpt-4o",
        "--file",
        "config.yaml",
        "--skill",
        "my-skill",
        "--mcp",
        r#"{"local":{"command":"echo"}}"#,
        "--profile",
        "default",
    ])
    .unwrap();

    let Some(Command::CommandLine(boxed_cmd)) = args.command else {
        panic!("Expected `warp agent run` command");
    };
    let CliCommand::Agent(AgentCommand::Run(run_args)) = boxed_cmd.as_ref() else {
        panic!("Expected `warp agent run` command");
    };

    assert_eq!(run_args.prompt_arg.prompt.as_deref(), Some("hello"));
    assert_eq!(run_args.model.model.as_deref(), Some("gpt-4o"));
    assert_eq!(
        run_args.config_file.file.as_ref().and_then(|p| p.to_str()),
        Some("config.yaml")
    );
    assert!(run_args.skill.is_some());
    assert_eq!(run_args.mcp_specs.len(), 1);
    assert_eq!(run_args.profile.as_deref(), Some("default"));
}

#[test]
fn retained_agent_profile_and_list_parse() {
    let args = Args::try_parse_from(["warp", "agent", "profile", "list"]).unwrap();
    let Some(Command::CommandLine(boxed_cmd)) = args.command else {
        panic!("Expected `warp agent profile list` command");
    };
    assert!(matches!(
        boxed_cmd.as_ref(),
        CliCommand::Agent(AgentCommand::Profile(
            crate::agent::AgentProfileCommand::List
        ))
    ));

    let args = Args::try_parse_from(["warp", "agent", "list"]).unwrap();
    let Some(Command::CommandLine(boxed_cmd)) = args.command else {
        panic!("Expected `warp agent list` command");
    };
    assert!(matches!(
        boxed_cmd.as_ref(),
        CliCommand::Agent(AgentCommand::List(_))
    ));
}

#[test]
fn retained_model_provider_and_mcp_parse() {
    let args = Args::try_parse_from(["warp", "model", "list"]).unwrap();
    let Some(Command::CommandLine(boxed_cmd)) = args.command else {
        panic!("Expected `warp model list` command");
    };
    assert!(matches!(
        boxed_cmd.as_ref(),
        CliCommand::Model(crate::model::ModelCommand::List)
    ));

    let args = Args::try_parse_from(["warp", "provider", "list"]).unwrap();
    let Some(Command::CommandLine(boxed_cmd)) = args.command else {
        panic!("Expected `warp provider list` command");
    };
    assert!(matches!(
        boxed_cmd.as_ref(),
        CliCommand::Provider(ProviderCommand::List)
    ));

    assert!(Args::try_parse_from(["warp", "mcp", "list"]).is_ok());
}

#[test]
fn hosted_commands_are_not_registered() {
    for args in [
        &["artifact"][..],
        &["artifact", "get", "artifact-123"][..],
        &["artifact", "download", "artifact-123"][..],
        &["environment"][..],
        &["e"][..],
        &["integration"][..],
        &["integration", "create", "slack"][..],
        &["integration", "list"][..],
        &["schedule"][..],
        &["run"][..],
        &["task"][..],
        &["run", "list"][..],
        &["run", "message", "list", "run-123"][..],
        &["federate", "issue-token", "--run-id", "run-1"][..],
        &[
            "harness-support",
            "--run-id",
            "run-1",
            "finish-task",
            "--status",
            "success",
            "--summary",
            "ok",
        ][..],
    ] {
        assert_parse_fails(args);
    }
}

#[test]
fn hosted_agent_run_flags_are_not_registered() {
    for args in [
        &["agent", "run", "--prompt", "hello", "--share"][..],
        &[
            "agent",
            "run",
            "--prompt",
            "hello",
            "--environment",
            "env-1",
        ][..],
        &["agent", "run", "--prompt", "hello", "-e", "env-1"][..],
        &[
            "agent",
            "run",
            "--prompt",
            "hello",
            "--conversation",
            "conv-1",
        ][..],
        &["agent", "run", "--task-id", "task-1"][..],
        &["agent", "run", "--prompt", "hello", "--sandboxed"][..],
        &["agent", "run", "--prompt", "hello", "--harness", "oz"][..],
        &["agent", "run", "--prompt", "hello", "--no-snapshot"][..],
        &[
            "agent",
            "run",
            "--prompt",
            "hello",
            "--snapshot-upload-timeout",
            "90s",
        ][..],
        &[
            "agent",
            "run",
            "--prompt",
            "hello",
            "--snapshot-script-timeout",
            "45s",
        ][..],
    ] {
        assert_parse_fails(args);
    }
}

#[test]
fn hosted_url_override_flags_are_not_registered() {
    for args in [
        &[
            "--server-root-url",
            "http://localhost:8080",
            "model",
            "list",
        ][..],
        &[
            "--ws-server-url",
            "ws://localhost:8082/graphql/v2",
            "model",
            "list",
        ][..],
        &[
            "--session-sharing-server-url",
            "ws://127.0.0.1:8081",
            "model",
            "list",
        ][..],
    ] {
        assert_parse_fails(args);
    }
}

#[test]
#[serial_test::serial]
fn hosted_url_override_env_vars_are_ignored() {
    let previous_server_root = set_env_var("WARP_SERVER_ROOT_URL", "http://localhost:8080");
    let previous_ws = set_env_var("WARP_WS_SERVER_URL", "ws://localhost:8082/graphql/v2");
    let previous_session_sharing =
        set_env_var("WARP_SESSION_SHARING_SERVER_URL", "ws://127.0.0.1:8081");

    let args = Args::try_parse_from(["warp", "model", "list"]).unwrap();

    restore_env_var("WARP_SERVER_ROOT_URL", previous_server_root);
    restore_env_var("WARP_WS_SERVER_URL", previous_ws);
    restore_env_var("WARP_SESSION_SHARING_SERVER_URL", previous_session_sharing);

    assert!(matches!(
        args.command.as_ref(),
        Some(Command::CommandLine(command)) if matches!(command.as_ref(), CliCommand::Model(_))
    ));
}

#[test]
fn help_does_not_expose_hosted_cli_surfaces() {
    warp_core::features::mark_initialized();

    let root_help = Args::clap_command().render_help().to_string();
    for removed in [
        "environment",
        "artifact",
        "integration",
        "schedule",
        "harness-support",
        "federate",
        "session sharing",
        "session-sharing",
        "cloud",
        "Oz",
        "docs.warp.dev",
    ] {
        assert!(
            !root_help.contains(removed),
            "root help should not contain {removed:?}:\n{root_help}"
        );
    }

    let mut command = Args::clap_command();
    let agent = command.find_subcommand_mut("agent").unwrap();
    let agent_run_help = agent
        .find_subcommand_mut("run")
        .unwrap()
        .render_help()
        .to_string();
    for removed in [
        "--share",
        "--environment",
        "--conversation",
        "--task-id",
        "--sandboxed",
        "--harness",
        "--no-snapshot",
        "--snapshot-upload-timeout",
        "--snapshot-script-timeout",
        "cloud",
        "Oz",
    ] {
        assert!(
            !agent_run_help.contains(removed),
            "agent run help should not contain {removed:?}:\n{agent_run_help}"
        );
    }
}
