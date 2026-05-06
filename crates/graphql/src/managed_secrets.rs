use crate::scalars::Time;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ManagedSecretType {
    AnthropicApiKey,
    AnthropicBedrockAccessKey,
    AnthropicBedrockApiKey,
    Dotenvx,
    RawValue,
}

impl ManagedSecretType {
    /// The identifier for this secret type as used in the client-side upload envelope.
    pub fn envelope_name(&self) -> &str {
        match self {
            ManagedSecretType::AnthropicApiKey => "anthropic_api_key",
            ManagedSecretType::AnthropicBedrockAccessKey => "anthropic_bedrock_access_key",
            ManagedSecretType::AnthropicBedrockApiKey => "anthropic_bedrock_api_key",
            ManagedSecretType::Dotenvx => "dotenvx",
            ManagedSecretType::RawValue => "raw_value",
        }
    }
}

#[derive(Debug)]
pub struct ManagedSecretConfig {
    /// The base64-encoded public key.
    pub public_key: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ManagedSecretOwner {
    CurrentUser,
}

#[derive(Debug, Clone)]
pub struct ManagedSecret {
    pub name: String,
    pub description: Option<String>,
    pub created_at: Time,
    pub updated_at: Time,
    pub owner: ManagedSecretOwner,
    pub type_: ManagedSecretType,
}
