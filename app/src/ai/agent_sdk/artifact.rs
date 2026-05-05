use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Serialize;
use warp_cli::agent::OutputFormat;
use warp_cli::artifact::{
    ArtifactCommand, DownloadArtifactArgs, GetArtifactArgs, UploadArtifactArgs,
};
use warp_cli::GlobalOptions;
use warpui::{AppContext, ModelContext, SingletonEntity};

use crate::server::server_api::ai::ArtifactDownloadResponse;
#[cfg(test)]
use crate::server::server_api::ai::FileArtifactRecord;

/// Run artifact-related commands.
pub fn run(
    ctx: &mut AppContext,
    global_options: GlobalOptions,
    command: ArtifactCommand,
) -> Result<()> {
    let runner = ctx.add_singleton_model(|_| ArtifactCommandRunner);
    match command {
        ArtifactCommand::Upload(args) => {
            runner.update(ctx, |runner, ctx| {
                runner.upload(args, global_options.output_format, ctx);
            });
            Ok(())
        }
        ArtifactCommand::Get(args) => {
            runner.update(ctx, |runner, ctx| {
                runner.get(args, global_options.output_format, ctx);
            });
            Ok(())
        }
        ArtifactCommand::Download(args) => {
            runner.update(ctx, |runner, ctx| {
                runner.download(args, global_options.output_format, ctx);
            });
            Ok(())
        }
    }
}

struct ArtifactCommandRunner;

impl ArtifactCommandRunner {
    fn get(
        &self,
        args: GetArtifactArgs,
        output_format: OutputFormat,
        ctx: &mut ModelContext<Self>,
    ) {
        let _ = (args, output_format);
        super::report_fatal_error(hosted_artifacts_unavailable(), ctx);
    }

    fn download(
        &self,
        args: DownloadArtifactArgs,
        output_format: OutputFormat,
        ctx: &mut ModelContext<Self>,
    ) {
        let _ = (args, output_format);
        super::report_fatal_error(hosted_artifacts_unavailable(), ctx);
    }

    fn upload(
        &self,
        args: UploadArtifactArgs,
        output_format: OutputFormat,
        ctx: &mut ModelContext<Self>,
    ) {
        let _ = (args, output_format);
        super::report_fatal_error(hosted_artifacts_unavailable(), ctx);
    }
}

impl warpui::Entity for ArtifactCommandRunner {
    type Event = ();
}

impl SingletonEntity for ArtifactCommandRunner {}

fn hosted_artifacts_unavailable() -> anyhow::Error {
    anyhow::anyhow!("Hosted artifacts are unavailable in local-only Warper")
}

#[derive(Debug, Serialize)]
struct ArtifactMetadataOutput {
    artifact_uid: String,
    artifact_type: String,
    created_at: String,
    download_url: String,
    expires_at: String,
    content_type: String,
    filepath: Option<String>,
    filename: Option<String>,
    description: Option<String>,
    size_bytes: Option<i64>,
}

impl ArtifactMetadataOutput {
    fn new(artifact: &ArtifactDownloadResponse) -> Self {
        Self {
            artifact_uid: artifact.artifact_uid().to_string(),
            artifact_type: artifact.artifact_type().to_string(),
            created_at: artifact.created_at().to_rfc3339(),
            download_url: artifact.download_url().to_string(),
            expires_at: artifact.expires_at().to_rfc3339(),
            content_type: artifact.content_type().to_string(),
            filepath: artifact.filepath().map(ToString::to_string),
            filename: artifact.filename().map(ToString::to_string),
            description: artifact.description().map(ToString::to_string),
            size_bytes: artifact.size_bytes(),
        }
    }
}

#[derive(Debug, Serialize)]
struct DownloadArtifactOutput {
    artifact_uid: String,
    artifact_type: String,
    path: PathBuf,
}

impl DownloadArtifactOutput {
    fn new(artifact: &ArtifactDownloadResponse, path: PathBuf) -> Self {
        Self {
            artifact_uid: artifact.artifact_uid().to_string(),
            artifact_type: artifact.artifact_type().to_string(),
            path,
        }
    }
}

fn write_get_output(
    artifact: &ArtifactDownloadResponse,
    output_format: OutputFormat,
) -> Result<()> {
    let mut stdout = std::io::stdout();
    write_get_output_to(&mut stdout, artifact, output_format)
}

fn write_get_output_to<W: std::io::Write>(
    output: &mut W,
    artifact: &ArtifactDownloadResponse,
    output_format: OutputFormat,
) -> Result<()> {
    let output_record = ArtifactMetadataOutput::new(artifact);

    match output_format {
        OutputFormat::Json | OutputFormat::Ndjson => {
            serde_json::to_writer(&mut *output, &output_record)
                .context("unable to write JSON output")?;
            writeln!(&mut *output)?;
        }
        OutputFormat::Pretty => {
            writeln!(&mut *output, "Artifact UID: {}", output_record.artifact_uid)?;
            writeln!(
                &mut *output,
                "Artifact type: {}",
                output_record.artifact_type
            )?;
            writeln!(&mut *output, "Created at: {}", output_record.created_at)?;
            writeln!(&mut *output, "Download URL: {}", output_record.download_url)?;
            writeln!(&mut *output, "Expires at: {}", output_record.expires_at)?;
            writeln!(&mut *output, "Content type: {}", output_record.content_type)?;
            if let Some(filepath) = output_record.filepath {
                writeln!(&mut *output, "Filepath: {filepath}")?;
            }
            if let Some(filename) = output_record.filename {
                writeln!(&mut *output, "Filename: {filename}")?;
            }
            if let Some(description) = output_record.description {
                writeln!(&mut *output, "Description: {description}")?;
            }
            if let Some(size_bytes) = output_record.size_bytes {
                writeln!(&mut *output, "Size bytes: {size_bytes}")?;
            }
        }
        OutputFormat::Text => {
            writeln!(
                &mut *output,
                "Artifact UID\tArtifact type\tCreated at\tDownload URL\tExpires at\tContent type\tFilepath\tFilename\tDescription\tSize bytes"
            )?;
            writeln!(
                &mut *output,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                output_record.artifact_uid,
                output_record.artifact_type,
                output_record.created_at,
                output_record.download_url,
                output_record.expires_at,
                output_record.content_type,
                output_record.filepath.unwrap_or_default(),
                output_record.filename.unwrap_or_default(),
                output_record.description.unwrap_or_default(),
                output_record
                    .size_bytes
                    .map(|size| size.to_string())
                    .unwrap_or_default()
            )?;
        }
    }

    Ok(())
}

fn write_download_output(
    output_record: &DownloadArtifactOutput,
    output_format: OutputFormat,
) -> Result<()> {
    let mut stdout = std::io::stdout();
    write_download_output_to(&mut stdout, output_record, output_format)
}

fn write_download_output_to<W: std::io::Write>(
    output: &mut W,
    output_record: &DownloadArtifactOutput,
    output_format: OutputFormat,
) -> Result<()> {
    match output_format {
        OutputFormat::Json | OutputFormat::Ndjson => {
            serde_json::to_writer(&mut *output, output_record)
                .context("unable to write JSON output")?;
            writeln!(&mut *output)?;
        }
        OutputFormat::Pretty => {
            writeln!(&mut *output, "Artifact downloaded")?;
            writeln!(&mut *output, "Artifact UID: {}", output_record.artifact_uid)?;
            writeln!(
                &mut *output,
                "Artifact type: {}",
                output_record.artifact_type
            )?;
            writeln!(&mut *output, "Path: {}", output_record.path.display())?;
        }
        OutputFormat::Text => {
            writeln!(&mut *output, "Artifact UID\tArtifact type\tPath")?;
            writeln!(
                &mut *output,
                "{}\t{}\t{}",
                output_record.artifact_uid,
                output_record.artifact_type,
                output_record.path.display()
            )?;
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "artifact_tests.rs"]
mod tests;
