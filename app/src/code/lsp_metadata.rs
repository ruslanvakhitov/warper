use serde::{Deserialize, Serialize};

/// The source from which the user enabled an LSP server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LspEnablementSource {
    #[serde(rename = "init_flow")]
    InitFlow,
    #[serde(rename = "footer_button")]
    FooterButton,
    #[serde(rename = "settings")]
    Settings,
}

/// The control action the user performed on an LSP server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LspControlActionType {
    #[serde(rename = "open_logs")]
    OpenLogs,
    #[serde(rename = "restart")]
    Restart,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "restart_all")]
    RestartAll,
    #[serde(rename = "stop_all")]
    StopAll,
}
