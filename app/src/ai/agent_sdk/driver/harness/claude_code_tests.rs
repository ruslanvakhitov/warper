use std::collections::HashMap;

use tempfile::TempDir;
use uuid::Uuid;

use super::*;

#[test]
fn claude_command_uses_session_id_for_local_runs() {
    let uuid = Uuid::new_v4();
    let cmd = claude_command("claude", &uuid, "/tmp/prompt.txt", None, false);
    assert!(
        cmd.contains(&format!("--session-id {uuid}")),
        "expected --session-id flag in local command, got: {cmd}"
    );
    assert!(
        !cmd.contains("--resume"),
        "local command should not contain hosted resume flag, got: {cmd}"
    );
}

#[test]
fn claude_command_keeps_resume_flag_helper_for_local_disk_sessions_only() {
    let uuid = Uuid::new_v4();
    let cmd = claude_command("claude", &uuid, "/tmp/prompt.txt", None, true);
    assert!(
        cmd.contains(&format!("--resume {uuid}")),
        "expected --resume flag when explicitly requested, got: {cmd}"
    );
    assert!(
        !cmd.contains("--session-id"),
        "resume command should not contain --session-id, got: {cmd}"
    );
}

#[test]
fn claude_command_pipes_prompt_path() {
    let uuid = Uuid::new_v4();
    let cmd = claude_command("claude", &uuid, "/tmp/prompt with spaces.txt", None, false);
    assert!(
        cmd.contains("< '/tmp/prompt with spaces.txt'"),
        "expected single-quoted stdin redirect of the prompt path, got: {cmd}"
    );
    assert!(
        cmd.contains("--dangerously-skip-permissions"),
        "expected --dangerously-skip-permissions, got: {cmd}"
    );
}

#[test]
fn prepare_claude_config_accepts_project_and_api_key_suffix_locally() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join(".claude.json");
    let working_dir = tmp.path().join("workspace");

    prepare_claude_config(&config_path, &working_dir, Some("abcdefghijklmnopqrst")).unwrap();

    let config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(config_path).unwrap()).unwrap();
    let working_dir_key = working_dir.to_string_lossy().to_string();
    assert_eq!(config["hasCompletedOnboarding"], true);
    assert_eq!(config["lspRecommendationDisabled"], true);
    assert_eq!(
        config["projects"][working_dir_key.as_str()]["hasTrustDialogAccepted"],
        true
    );
    assert_eq!(
        config["customApiKeyResponses"]["approved"][0],
        "abcdefghijklmnopqrst"
    );
}

#[test]
fn resolve_anthropic_api_key_suffix_prefers_managed_secret() {
    let mut secrets = HashMap::new();
    secrets.insert(
        "anthropic".to_string(),
        ManagedSecretValue::AnthropicApiKey {
            api_key: "sk-ant-api03-abcdefghijklmnopqrstuvwxyz".to_string(),
        },
    );

    assert_eq!(
        resolve_anthropic_api_key_suffix(&secrets).as_deref(),
        Some("ghijklmnopqrstuvwxyz")
    );
}
