use std::env;
use std::fs::File;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use blocking::unblock;
use mime_guess::from_path;
use warp_cli::artifact::UploadArtifactArgs;

use super::common::parse_ambient_task_id;
use crate::ai::agent::api::ServerConversationToken;
use crate::ai::agent::conversation::ServerAIConversationMetadata;
use crate::ai::ambient_agents::AmbientAgentTaskId;
use crate::server::server_api::ai::FileArtifactRecord;

const MIME_SNIFF_BYTES: usize = 8 * 1024;
const RUN_ID_ENV_VAR: &str = "WARPER_RUN_ID";

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct FileArtifactUploadRequest {
    pub(crate) path: PathBuf,
    pub(crate) run_id: Option<AmbientAgentTaskId>,
    pub(crate) conversation_id: Option<ServerConversationToken>,
    pub(crate) description: Option<String>,
}

impl TryFrom<UploadArtifactArgs> for FileArtifactUploadRequest {
    type Error = anyhow::Error;

    fn try_from(value: UploadArtifactArgs) -> Result<Self> {
        let run_id = match value.run_id {
            Some(run_id) => Some(parse_run_id(&run_id, "Invalid run ID")?),
            None => None,
        };

        Ok(Self {
            path: value.path,
            run_id,
            conversation_id: value.conversation_id.map(ServerConversationToken::new),
            description: value.description,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CompletedFileArtifactUpload {
    pub(crate) artifact: FileArtifactRecord,
    pub(crate) size_bytes: i64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct ResolvedUploadAssociation {
    conversation_id: Option<ServerConversationToken>,
    run_id: Option<AmbientAgentTaskId>,
    pub(crate) ambient_task_id: AmbientAgentTaskId,
}

#[derive(Debug, Clone)]
struct PreparedUploadArtifact {
    path: PathBuf,
    filepath: String,
    mime_type: String,
    file_size: u64,
}

impl PreparedUploadArtifact {
    fn from_path(path: PathBuf) -> Result<Self> {
        // `infer` only needs leading signature bytes, so avoid buffering the whole artifact
        // before we stream the file body to the upload target.
        let (file_size, mime_sniff_bytes) = file_size_and_prefix_for_path(&path, MIME_SNIFF_BYTES)?;

        Ok(Self {
            filepath: normalize_artifact_filepath(&path),
            mime_type: infer_mime_type(&path, &mime_sniff_bytes),
            file_size,
            path,
        })
    }

    fn graphql_size_bytes(&self) -> Option<i32> {
        checked_graphql_size_bytes_for_upload(&self.path, self.file_size)
    }
}

pub(crate) struct FileArtifactUploader;

impl FileArtifactUploader {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) async fn upload_with_association(
        &self,
        request: FileArtifactUploadRequest,
        association: ResolvedUploadAssociation,
    ) -> Result<CompletedFileArtifactUpload> {
        let _ = (request, association);
        Err(anyhow!(
            "Hosted artifact uploads are unavailable in local-only Warper"
        ))
    }

    async fn prepare_upload_artifact(&self, path: PathBuf) -> Result<PreparedUploadArtifact> {
        unblock(move || PreparedUploadArtifact::from_path(path)).await
    }

    pub(crate) async fn resolve_upload_association(
        &self,
        request: &FileArtifactUploadRequest,
    ) -> Result<ResolvedUploadAssociation> {
        let conversation_task_id = request.conversation_id.as_ref().map(|conversation_id| {
            Err(anyhow!(
                "Conversation '{}' cannot be resolved because hosted artifact uploads are unavailable in local-only Warper",
                conversation_id.as_str()
            ))
        });

        resolve_upload_association_from_sources(
            request.run_id,
            request.conversation_id.clone(),
            conversation_task_id,
            load_env_run_id()?,
        )
    }
}

fn normalize_artifact_filepath(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn infer_mime_type(path: &Path, file_bytes: &[u8]) -> String {
    infer::get(file_bytes)
        .map(|kind| kind.mime_type().to_string())
        .unwrap_or_else(|| from_path(path).first_or_octet_stream().to_string())
}

fn file_size_and_prefix_for_path(path: &Path, max_bytes: usize) -> Result<(u64, Vec<u8>)> {
    let mut file = File::open(path)
        .with_context(|| format!("Failed to open artifact file '{}'", path.display()))?;
    let file_size = file
        .metadata()
        .with_context(|| format!("Failed to stat artifact file '{}'", path.display()))?
        .len();
    let mut bytes = vec![0; max_bytes];
    let bytes_read = file
        .read(&mut bytes)
        .with_context(|| format!("Failed to read artifact file '{}'", path.display()))?;
    bytes.truncate(bytes_read);
    Ok((file_size, bytes))
}

fn checked_graphql_size_bytes_for_upload(path: &Path, size_bytes: u64) -> Option<i32> {
    let graphql_size_bytes = i32::try_from(size_bytes).ok();
    if graphql_size_bytes.is_none() {
        // The backing upload can handle large files, but the GraphQL field is still `Int`.
        // Dropping `size_bytes` preserves the upload request instead of failing on conversion.
        log::warn!(
            "Artifact file '{}' is {} bytes, which exceeds the GraphQL size_bytes limit of {} bytes; omitting size_bytes from the upload target request",
            path.display(),
            size_bytes,
            i32::MAX,
        );
    }

    graphql_size_bytes
}

fn single_conversation_metadata(
    conversation_id: &str,
    mut metadata: Vec<ServerAIConversationMetadata>,
) -> Result<ServerAIConversationMetadata> {
    match metadata.len() {
        0 => bail!("Conversation not found"),
        1 => Ok(metadata.pop().expect("metadata length checked")),
        _ => bail!("Multiple conversations found for '{conversation_id}'"),
    }
}

fn ambient_task_id_from_conversation_metadata(
    conversation_id: &str,
    metadata: ServerAIConversationMetadata,
) -> Result<AmbientAgentTaskId> {
    metadata.ambient_agent_task_id.ok_or_else(|| {
        anyhow!("Conversation '{conversation_id}' is not backed by a cloud agent task")
    })
}

fn parse_run_id(run_id: &str, error_prefix: &str) -> Result<AmbientAgentTaskId> {
    parse_ambient_task_id(run_id, error_prefix)
}

fn load_env_run_id() -> Result<Option<String>> {
    match env::var(RUN_ID_ENV_VAR) {
        Ok(run_id) => Ok(Some(run_id)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => {
            Err(anyhow!("{RUN_ID_ENV_VAR} is set but is not valid Unicode"))
        }
    }
}

fn resolve_env_run_id(env_run_id: Option<String>) -> Result<AmbientAgentTaskId> {
    let Some(run_id) = env_run_id else {
        bail!("{RUN_ID_ENV_VAR} is not set");
    };

    parse_run_id(&run_id, "Invalid WARPER_RUN_ID")
}

fn resolve_upload_association_from_sources(
    explicit_run_id: Option<AmbientAgentTaskId>,
    explicit_conversation_id: Option<ServerConversationToken>,
    conversation_task_id: Option<Result<AmbientAgentTaskId>>,
    env_run_id: Option<String>,
) -> Result<ResolvedUploadAssociation> {
    // Precedence is deliberate:
    // 1. An explicit run ID is authoritative and must not silently fall back.
    // 2. A conversation ID stays attached to the artifact even if we have to borrow the ambient
    //    task ID from `WARPER_RUN_ID` because the conversation lacks task metadata.
    // 3. `WARPER_RUN_ID` becomes the sole source of truth only when the caller supplied nothing else.
    if let Some(run_id) = explicit_run_id {
        let ambient_task_id = run_id;
        return Ok(ResolvedUploadAssociation {
            conversation_id: None,
            run_id: Some(run_id),
            ambient_task_id,
        });
    }

    if let Some(conversation_id) = explicit_conversation_id {
        match conversation_task_id
            .ok_or_else(|| anyhow!("conversation resolution should be provided"))?
        {
            Ok(ambient_task_id) => {
                return Ok(ResolvedUploadAssociation {
                    conversation_id: Some(conversation_id),
                    run_id: None,
                    ambient_task_id,
                });
            }
            Err(conversation_err) => {
                let env_err = match resolve_env_run_id(env_run_id) {
                    Ok(ambient_task_id) => {
                        log::warn!(
                            "Conversation '{}' task resolution failed ({conversation_err}); falling back to {RUN_ID_ENV_VAR} for ambient task context",
                            conversation_id.as_str()
                        );
                        return Ok(ResolvedUploadAssociation {
                            conversation_id: Some(conversation_id),
                            run_id: None,
                            ambient_task_id,
                        });
                    }
                    Err(env_err) => env_err,
                };

                return Err(anyhow!(
                    "Failed to resolve artifact upload association for conversation '{}': {conversation_err}; also failed to use {RUN_ID_ENV_VAR}: {env_err}",
                    conversation_id.as_str()
                ));
            }
        }
    }

    let ambient_task_id = resolve_env_run_id(env_run_id).map_err(|env_err| {
        anyhow!(
            "Failed to resolve artifact upload association: no usable run or conversation id was provided, and {RUN_ID_ENV_VAR}: {env_err}"
        )
    })?;

    Ok(ResolvedUploadAssociation {
        conversation_id: None,
        run_id: Some(ambient_task_id),
        ambient_task_id,
    })
}

#[cfg(test)]
#[path = "artifact_upload_tests.rs"]
mod tests;
