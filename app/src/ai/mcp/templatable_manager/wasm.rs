use warpui::AppContext;
use warpui::ModelContext;

use super::TemplatableMCPServerManager;
use crate::ai::mcp::templatable::TemplatableMCPServer;
use crate::ai::mcp::templatable_installation::{TemplatableMCPServerInstallation, VariableValue};
use std::collections::HashMap;
use uuid::Uuid;

impl TemplatableMCPServerManager {
    /// Creates a new [`TemplatableMCPServerManager`] instance.
    pub fn new(
        _locally_installed_servers: HashMap<Uuid, TemplatableMCPServerInstallation>,
        _running_server_uuids: Vec<Uuid>,
        _running_legacy_servers: &[Uuid],
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        Default::default()
    }

    /// Updates a templatable MCP server.
    ///
    /// This is a no-op in WASM, as MCP servers are not supported in WASM.
    pub fn update_templatable_mcp_server(
        &mut self,
        _server: TemplatableMCPServer,
        _ctx: &mut ModelContext<Self>,
    ) {
        log::warn!("Templatable MCP server update not supported in WASM");
    }

    /// Gets all templatable MCP servers.
    ///
    /// This is a no-op in WASM, as MCP servers are not supported in WASM.
    pub fn get_all_templatable_mcp_servers(&self) -> Vec<&TemplatableMCPServer> {
        log::warn!("Getting all TemplatableMCPServers is not supported in WASM");
        vec![]
    }

    /// Gets a TemplatableMCPServer by its UUID.
    /// Returns the TemplatableMCPServer model if found, otherwise None.
    ///
    /// This is a no-op in WASM, as MCP servers are not supported in WASM.
    pub fn get_templatable_mcp_server(&self, _uuid: Uuid) -> Option<&TemplatableMCPServer> {
        log::warn!("Getting a TemplatableMCPServer by UUID is not supported in WASM");
        None
    }

    /// Creates a new templatable MCP server.
    ///
    /// This is a no-op in WASM, as MCP servers are not supported in WASM.
    pub fn create_templatable_mcp_server(
        &mut self,
        _server: TemplatableMCPServer,
        _ctx: &mut ModelContext<Self>,
    ) {
        log::warn!("Creating a TemplatableMCPServer is not supported in WASM");
    }

    /// Deletes a templatable MCP server.
    ///
    /// This is a no-op in WASM, as MCP servers are not supported in WASM.
    pub fn delete_templatable_mcp_server(&mut self, _uuid: Uuid, _ctx: &mut ModelContext<Self>) {
        log::warn!("Deleting a TemplatableMCPServer is not supported in WASM");
    }

    /// Spawns a new MCP server from a given [`TemplatableMCPServer`] instance.
    ///
    /// This is a no-op in WASM, as MCP servers are not supported in WASM.
    pub fn spawn_server(&mut self, _uuid: Uuid, _ctx: &mut ModelContext<Self>) {
        log::warn!("MCP server spawning not supported in WASM");
    }

    /// Shuts down a running MCP server.
    ///
    /// This is a no-op in WASM, as MCP servers are not supported in WASM.
    pub fn shutdown_server(&mut self, _uuid: Uuid, _ctx: &mut ModelContext<Self>) {
        log::warn!("MCP server shutdown not supported in WASM");
    }

    /// Deletes a locally installed MCP server installation.
    ///
    /// This is a no-op in WASM, as MCP servers are not supported in WASM.
    pub fn delete_templatable_mcp_server_installation(
        &mut self,
        _installation_uuid: Uuid,
        _ctx: &mut ModelContext<Self>,
    ) {
        log::warn!("Templatable MCP server installation deletion not supported in WASM");
    }

    pub fn install_from_template(
        &mut self,
        _templatable_mcp_server: TemplatableMCPServer,
        _variable_values: HashMap<String, VariableValue>,
        _start_automatically: bool,
        _ctx: &mut ModelContext<Self>,
    ) -> Option<TemplatableMCPServerInstallation> {
        log::warn!("Templatable MCP server installation not supported in WASM");
        None
    }

    /// Enables (starts) the installed Figma MCP server.
    ///
    /// This is a no-op in WASM, as MCP servers are not supported in WASM.
    pub fn enable_figma_mcp(&mut self, _ctx: &mut ModelContext<Self>) {
        log::warn!("Enabling Figma MCP server is not supported in WASM");
    }

    /// Delete oauth credentials from secure storage
    ///
    /// No-op in WASM, as MCP servers are not supported in WASM
    pub fn delete_credentials_from_secure_storage(
        &mut self,
        _sync_id: Uuid,
        _app: &mut warpui::AppContext,
    ) {
        log::warn!("Deleting credentials for MCP servers is not supported in WASM")
    }

    pub fn is_update_available_for_installation(
        &self,
        _installation_uuid: Uuid,
        _app: &AppContext,
    ) -> bool {
        false
    }

    pub fn get_installation_by_template_uuid(
        &self,
        _template_uuid: Uuid,
    ) -> Option<&TemplatableMCPServerInstallation> {
        None
    }

    pub fn get_all_runnable_mcp_servers(_ctx: &AppContext) -> Vec<(Uuid, String)> {
        Default::default()
    }

    pub fn get_mcp_name(_uuid: &Uuid, _app: &AppContext) -> Option<String> {
        Default::default()
    }

    pub fn has_oauth_credentials_for_server(&self, _template_uuid: Uuid) -> bool {
        false
    }

    pub fn spawn_ephemeral_server(
        &mut self,
        _installation: TemplatableMCPServerInstallation,
        _ctx: &mut ModelContext<Self>,
    ) {
        log::warn!("Ephemeral MCP server spawning not supported in WASM");
    }

    pub fn purge_file_based_server_credentials(
        &mut self,
        _installation_hashes: &Vec<u64>,
        _ctx: &mut ModelContext<Self>,
    ) {
        log::warn!("Purging file-based server credentials not supported in WASM");
    }

    pub fn has_oauth_credentials_for_file_based_server(&self, _hash: u64) -> bool {
        false
    }
}
