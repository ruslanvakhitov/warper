use serde::{Deserialize, Serialize};

mod command_dialog_view;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnvVarSecretCommand {
    pub name: String,
    pub command: String,
}
