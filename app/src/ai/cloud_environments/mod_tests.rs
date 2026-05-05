use super::*;

#[test]
fn deserialize_environment_ignores_legacy_provider_config() {
    let json = serde_json::json!({
        "name": "my-env",
        "github_repos": [{"owner": "warpdotdev", "repo": "warp"}],
        "docker_image": "ubuntu:latest",
        "setup_commands": ["echo hello"],
        "providers": {
            "aws": {
                "role_arn": "arn:aws:iam::123456789012:role/my-role"
            }
        }
    });

    let env: AmbientAgentEnvironment = serde_json::from_value(json).unwrap();
    assert_eq!(env.name, "my-env");
    assert_eq!(
        env.github_repos,
        vec![GithubRepo::new("warpdotdev".into(), "warp".into())]
    );
    assert_eq!(
        env.base_image,
        BaseImage::DockerImage("ubuntu:latest".into())
    );
    assert_eq!(env.setup_commands, vec!["echo hello"]);
}

#[test]
fn serialize_environment_omits_provider_config() {
    let env = AmbientAgentEnvironment::new(
        "test-env".into(),
        Some("desc".into()),
        vec![GithubRepo::new("owner".into(), "repo".into())],
        "alpine:latest".into(),
        vec!["make build".into()],
    );

    let json = serde_json::to_value(&env).unwrap();
    assert!(!json.as_object().unwrap().contains_key("providers"));
}
