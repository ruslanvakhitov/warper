#[derive(Debug)]
pub enum ManagedSecretValue {
    ManagedSecretRawValue(ManagedSecretRawValue),
    ManagedSecretAnthropicApiKeyValue(ManagedSecretAnthropicApiKeyValue),
    ManagedSecretAnthropicBedrockAccessKeyValue(ManagedSecretAnthropicBedrockAccessKeyValue),
    ManagedSecretAnthropicBedrockApiKeyValue(ManagedSecretAnthropicBedrockApiKeyValue),
    Unknown,
}

#[derive(Debug)]
pub struct ManagedSecretRawValue {
    pub value: String,
}

#[derive(Debug)]
pub struct ManagedSecretAnthropicApiKeyValue {
    pub api_key: String,
}

#[derive(Debug)]
pub struct ManagedSecretAnthropicBedrockAccessKeyValue {
    pub aws_access_key_id: String,
    pub aws_secret_access_key: String,
    pub aws_session_token: Option<String>,
    pub aws_region: String,
}

#[derive(Debug)]
pub struct ManagedSecretAnthropicBedrockApiKeyValue {
    pub aws_bearer_token_bedrock: String,
    pub aws_region: String,
}
