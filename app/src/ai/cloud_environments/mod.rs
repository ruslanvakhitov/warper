use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GithubRepo {
    /// Repository owner (e.g. "warpdotdev")
    pub owner: String,
    /// Repository name (e.g. "warp-internal")
    pub repo: String,
}

impl GithubRepo {
    pub fn new(owner: String, repo: String) -> Self {
        Self { owner, repo }
    }
}

impl fmt::Display for GithubRepo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.repo)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BaseImage {
    DockerImage(String),
}

impl fmt::Display for BaseImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BaseImage::DockerImage(s) => s.fmt(f),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Environment settings used when preparing a local agent workspace.
pub struct AmbientAgentEnvironment {
    /// Environment name
    #[serde(default)]
    pub name: String,
    /// Optional description of the environment (max 240 characters)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// List of GitHub repositories
    #[serde(default)]
    pub github_repos: Vec<GithubRepo>,
    /// Base image specification
    #[serde(flatten)]
    pub base_image: BaseImage,
    /// List of setup commands to run after cloning
    #[serde(default)]
    pub setup_commands: Vec<String>,
}

impl AmbientAgentEnvironment {
    pub fn new(
        name: String,
        description: Option<String>,
        github_repos: Vec<GithubRepo>,
        docker_image: String,
        setup_commands: Vec<String>,
    ) -> Self {
        Self {
            name,
            description,
            github_repos,
            base_image: BaseImage::DockerImage(docker_image),
            setup_commands,
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
