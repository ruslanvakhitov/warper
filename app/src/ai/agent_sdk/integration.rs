use futures::future;
use warp_cli::{
    integration::{CreateIntegrationArgs, IntegrationCommand, UpdateIntegrationArgs},
    provider::ProviderType,
    GlobalOptions,
};
use warpui::{platform::TerminationMode, AppContext, ModelContext, SingletonEntity};

use super::common::{EnvironmentChoice, ResolveConfigurationError};

pub fn run(
    ctx: &mut AppContext,
    global_options: GlobalOptions,
    command: IntegrationCommand,
) -> anyhow::Result<()> {
    let runner = ctx.add_singleton_model(|_ctx| IntegrationCommandRunner);
    match command {
        IntegrationCommand::Create(args) => {
            runner.update(ctx, |runner, ctx| runner.create(args, ctx));
        }
        IntegrationCommand::Update(args) => {
            runner.update(ctx, |runner, ctx| runner.update(args, ctx));
        }
        IntegrationCommand::List => {
            runner.update(ctx, |runner, ctx| runner.list(global_options, ctx));
        }
    }
    Ok(())
}

struct IntegrationCommandRunner;

impl IntegrationCommandRunner {
    fn list(&self, _global_options: GlobalOptions, ctx: &mut ModelContext<Self>) {
        println!("Hosted integrations are unavailable in local-only Warper.");
        ctx.terminate_app(TerminationMode::ForceTerminate, None);
    }

    fn create(&self, args: CreateIntegrationArgs, ctx: &mut ModelContext<Self>) {
        let refresh_future = super::common::refresh_workspace_metadata(ctx);
        let warp_drive_sync_future = super::common::refresh_warp_drive(ctx);
        let setup_future = future::try_join(refresh_future, warp_drive_sync_future);

        ctx.spawn(setup_future, move |runner, setup_result, ctx| {
            if let Err(err) = setup_result {
                ctx.terminate_app(TerminationMode::ForceTerminate, Some(Err(err)));
                return;
            }

            let loaded_file = match args.config_file.file.as_deref() {
                Some(path) => match super::config_file::load_config_file(path) {
                    Ok(file) => Some(file),
                    Err(err) => {
                        ctx.terminate_app(TerminationMode::ForceTerminate, Some(Err(err)));
                        return;
                    }
                },
                None => None,
            };

            let integration_type = args.provider.slug();
            let enabled = true;
            let is_update = false;

            let cli_mcp_servers =
                match super::mcp_config::build_mcp_servers_from_specs(&args.mcp_specs) {
                    Ok(mcp_servers) => mcp_servers,
                    Err(err) => {
                        ctx.terminate_app(TerminationMode::ForceTerminate, Some(Err(err)));
                        return;
                    }
                };

            let mut merged_config = super::config_file::merge_with_precedence(
                loaded_file.as_ref(),
                crate::ai::ambient_agents::AgentConfigSnapshot {
                    name: None,
                    environment_id: args.environment.environment.clone(),
                    model_id: args.model.model.clone(),
                    base_prompt: args.prompt.clone(),
                    mcp_servers: cli_mcp_servers,
                    profile_id: None,
                    worker_host: args.worker_host.clone(),
                    skill_spec: None,
                    // TODO(QUALITY-295): Support computer use flag in integrations.
                    computer_use_enabled: None,
                    // TODO(REMOTE-1134): Support harness selection for integrations.
                    harness: None,
                    harness_auth_secrets: None,
                },
            );

            // We must wait until after workspace metadata is refreshed to check available LLMs.
            let model_id = match merged_config
                .model_id
                .as_deref()
                .map(|model_id| super::common::validate_agent_mode_base_model_id(model_id, ctx))
                .transpose()
            {
                Ok(model_id) => model_id.map(|model_id| model_id.to_string()),
                Err(err) => {
                    ctx.terminate_app(TerminationMode::ForceTerminate, Some(Err(err)));
                    return;
                }
            };

            let base_prompt = merged_config.base_prompt.take();
            let worker_host = merged_config.worker_host.take();

            let mcp_servers_json = match merged_config.mcp_servers.take() {
                Some(map) => match serde_json::to_string(&map) {
                    Ok(json) => Some(json),
                    Err(err) => {
                        ctx.terminate_app(TerminationMode::ForceTerminate, Some(Err(err.into())));
                        return;
                    }
                },
                None => None,
            };

            //If the user didn't explicitly request no environment, load environment from the config
            let mut environment_args = args.environment;
            if environment_args.environment.is_none() && !environment_args.no_environment {
                environment_args.environment = merged_config.environment_id.take();
            }

            let environment_uid = match EnvironmentChoice::resolve_for_create(environment_args, ctx)
            {
                Ok(EnvironmentChoice::None) => {
                    eprintln!("Creating integration without an environment.");
                    None
                }
                Ok(EnvironmentChoice::Environment { id, .. }) => {
                    eprintln!("Creating integration with environment {id}.");
                    Some(id)
                }
                Err(ResolveConfigurationError::Canceled) => {
                    eprintln!("Integration creation canceled.");
                    ctx.terminate_app(TerminationMode::ForceTerminate, None);
                    return;
                }
                Err(err) => {
                    super::report_fatal_error(anyhow::anyhow!(err), ctx);
                    return;
                }
            };

            runner.start_create_or_update_flow(
                ctx,
                integration_type,
                environment_uid,
                base_prompt,
                model_id,
                mcp_servers_json,
                None,
                worker_host,
                enabled,
                is_update,
                1,
            );
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn start_create_or_update_flow(
        &self,
        ctx: &mut ModelContext<Self>,
        _integration_type: String,
        _environment_uid: Option<String>,
        _base_prompt: Option<String>,
        _model_id: Option<String>,
        _mcp_servers_json: Option<String>,
        _remove_mcp_server_names: Option<Vec<String>>,
        _worker_host: Option<String>,
        _enabled: bool,
        is_update: bool,
        _attempt: u32,
    ) {
        let action = if is_update { "update" } else { "creation" };
        ctx.terminate_app(
            TerminationMode::ForceTerminate,
            Some(Err(anyhow::anyhow!(
                "Hosted integration {action} is unavailable in local-only Warper"
            ))),
        );
    }

    fn update(&self, args: UpdateIntegrationArgs, ctx: &mut ModelContext<Self>) {
        let refresh_future = super::common::refresh_workspace_metadata(ctx);
        let warp_drive_sync_future = super::common::refresh_warp_drive(ctx);
        let setup_future = future::try_join(refresh_future, warp_drive_sync_future);

        ctx.spawn(setup_future, move |runner, setup_result, ctx| {
            if let Err(err) = setup_result {
                ctx.terminate_app(TerminationMode::ForceTerminate, Some(Err(err)));
                return;
            }

            let loaded_file = match args.config_file.file.as_deref() {
                Some(path) => match super::config_file::load_config_file(path) {
                    Ok(file) => Some(file),
                    Err(err) => {
                        ctx.terminate_app(TerminationMode::ForceTerminate, Some(Err(err)));
                        return;
                    }
                },
                None => None,
            };

            let remove_mcp = args.remove_mcp.clone();

            let integration_type = args.provider.slug();
            let enabled = true;
            let is_update = true;

            let cli_mcp_servers =
                match super::mcp_config::build_mcp_servers_from_specs(&args.mcp_specs) {
                    Ok(mcp_servers) => mcp_servers,
                    Err(err) => {
                        ctx.terminate_app(TerminationMode::ForceTerminate, Some(Err(err)));
                        return;
                    }
                };

            let mut merged_config = super::config_file::merge_with_precedence(
                loaded_file.as_ref(),
                crate::ai::ambient_agents::AgentConfigSnapshot {
                    name: None,
                    environment_id: args.environment.environment.clone(),
                    model_id: args.model.model.clone(),
                    base_prompt: args.prompt.clone(),
                    mcp_servers: cli_mcp_servers,
                    profile_id: None,
                    worker_host: args.worker_host.clone(),
                    skill_spec: None,
                    // TODO(QUALITY-295): Support computer use flag in integrations.
                    computer_use_enabled: None,
                    // TODO(REMOTE-1134): Support harness selection for integrations.
                    harness: None,
                    harness_auth_secrets: None,
                },
            );

            // We must wait until after workspace metadata is refreshed to check available LLMs.
            let model_id = match merged_config
                .model_id
                .as_deref()
                .map(|model_id| super::common::validate_agent_mode_base_model_id(model_id, ctx))
                .transpose()
            {
                Ok(model_id) => model_id.map(|model_id| model_id.to_string()),
                Err(err) => {
                    ctx.terminate_app(TerminationMode::ForceTerminate, Some(Err(err)));
                    return;
                }
            };

            let base_prompt = merged_config.base_prompt.take();
            let worker_host = merged_config.worker_host.take();

            // MCP update semantics are patch-only:
            // - `mcp_servers_json` adds/overwrites MCP servers.
            // - `remove_mcp_server_names` removes MCP servers.
            // If both are present, removals win by filtering removed names out of the JSON payload.
            let mcp_servers_json = match merged_config.mcp_servers.take() {
                Some(mut map) => {
                    for name in &remove_mcp {
                        map.remove(name);
                    }

                    if map.is_empty() {
                        None
                    } else {
                        match serde_json::to_string(&map) {
                            Ok(json) => Some(json),
                            Err(err) => {
                                ctx.terminate_app(
                                    TerminationMode::ForceTerminate,
                                    Some(Err(err.into())),
                                );
                                return;
                            }
                        }
                    }
                }
                None => None,
            };

            let remove_mcp_server_names = if args.remove_mcp.is_empty() {
                None
            } else {
                Some(args.remove_mcp)
            };

            if args.environment.remove_environment {
                // Explicitly requested to update without an environment.
                runner.start_create_or_update_flow(
                    ctx,
                    integration_type,
                    Some(String::new()),
                    base_prompt,
                    model_id,
                    mcp_servers_json,
                    remove_mcp_server_names,
                    worker_host,
                    enabled,
                    is_update,
                    1,
                );
                return;
            }

            let environment_uid = merged_config.environment_id.take();

            runner.start_create_or_update_flow(
                ctx,
                integration_type,
                environment_uid,
                base_prompt,
                model_id,
                mcp_servers_json,
                remove_mcp_server_names,
                worker_host,
                enabled,
                is_update,
                1,
            );
        });
    }
}

impl warpui::Entity for IntegrationCommandRunner {
    type Event = ();
}
impl SingletonEntity for IntegrationCommandRunner {}
