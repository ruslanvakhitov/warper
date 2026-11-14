use std::{collections::HashMap, future::Future, sync::Arc, time::Duration};

use vec1::vec1;

use warpui::{Entity, SingletonEntity};

use crate::{
    ManagedSecretValue,
    client::{
        IdentityTokenOptions, ManagedSecret, ManagedSecretsClient, SecretOwner, TaskIdentityToken,
        TaskManagedSecretValue,
    },
    gcp::{self, GcpWorkloadIdentityFederationError, GcpWorkloadIdentityFederationToken},
};

/// Singleton model for working with Warp-managed secrets.
pub struct ManagedSecretManager {
    client: Arc<dyn ManagedSecretsClient>,
}

impl ManagedSecretManager {
    pub fn new(client: Arc<dyn ManagedSecretsClient>) -> Self {
        crate::envelope::init();
        Self { client }
    }

    pub fn create_secret(
        &self,
        owner: SecretOwner,
        name: String,
        value: ManagedSecretValue,
        description: Option<String>,
    ) -> impl Future<Output = anyhow::Result<ManagedSecret>> + use<> {
        async move {
            let _ = (owner, name, value, description);
            Err(anyhow::anyhow!(
                "Warp-managed secrets are not available in Warper"
            ))
        }
    }

    pub fn delete_secret(
        &self,
        owner: SecretOwner,
        name: String,
    ) -> impl Future<Output = anyhow::Result<()>> + use<> {
        async move {
            let _ = (owner, name);
            Err(anyhow::anyhow!(
                "Warp-managed secrets are not available in Warper"
            ))
        }
    }

    pub fn update_secret(
        &self,
        owner: SecretOwner,
        name: String,
        value: Option<ManagedSecretValue>,
        description: Option<String>,
    ) -> impl Future<Output = anyhow::Result<ManagedSecret>> + use<> {
        async move {
            let _ = (owner, name, value, description);
            Err(anyhow::anyhow!(
                "Warp-managed secrets are not available in Warper"
            ))
        }
    }

    /// List all managed secrets accessible to the current user.
    pub fn list_secrets(&self) -> impl Future<Output = anyhow::Result<Vec<ManagedSecret>>> + use<> {
        let client = self.client.clone();
        async move {
            let secrets = client.list_secrets().await?;
            Ok(secrets)
        }
    }

    /// Get Warp-managed secrets scoped to the currently-executing task.
    ///
    /// This will fail if not in an ambient agent.
    pub fn get_task_secrets(
        &self,
        task_id: String,
    ) -> impl Future<Output = anyhow::Result<HashMap<String, ManagedSecretValue>>> + use<> {
        let client = self.client.clone();
        async move {
            // We only need the workload token for the duration of the request.
            let workload_token =
                warp_isolation_platform::issue_workload_token(Some(Duration::from_mins(5))).await?;
            let task_secrets = client
                .get_task_secrets(task_id, workload_token.token)
                .await?;

            // Convert task-scoped secret values into the local secret shape used by harnesses.
            let mut secrets = HashMap::new();
            for (name, task_value) in task_secrets {
                let value = match task_value {
                    TaskManagedSecretValue::RawValue { value } => {
                        ManagedSecretValue::raw_value(value)
                    }
                    TaskManagedSecretValue::AnthropicApiKey { api_key } => {
                        ManagedSecretValue::anthropic_api_key(api_key)
                    }
                    TaskManagedSecretValue::AnthropicBedrockAccessKey {
                        aws_access_key_id,
                        aws_secret_access_key,
                        aws_session_token,
                        aws_region,
                    } => ManagedSecretValue::anthropic_bedrock_access_key(
                        aws_access_key_id,
                        aws_secret_access_key,
                        aws_session_token,
                        aws_region,
                    ),
                    TaskManagedSecretValue::AnthropicBedrockApiKey {
                        aws_bearer_token_bedrock,
                        aws_region,
                    } => ManagedSecretValue::anthropic_bedrock_api_key(
                        aws_bearer_token_bedrock,
                        aws_region,
                    ),
                    TaskManagedSecretValue::Unknown => {
                        return Err(anyhow::anyhow!(
                            "Unknown secret value type for secret: {}",
                            name
                        ));
                    }
                };
                secrets.insert(name, value);
            }
            Ok(secrets)
        }
    }

    /// Issue a short-lived OIDC identity token for the current task.
    pub fn issue_task_identity_token(
        &self,
        options: IdentityTokenOptions,
    ) -> impl Future<Output = anyhow::Result<TaskIdentityToken>> + use<> {
        let client = self.client.clone();
        async move { client.issue_task_identity_token(options).await }
    }

    /// Issue a short-lived OIDC identity token in the JSON shape expected by
    /// GCP executable-sourced Workload Identity Federation credentials.
    pub fn issue_gcp_workload_identity_federation_token(
        &self,
        audience: String,
        token_type: String,
        requested_duration: Duration,
    ) -> impl Future<
        Output = Result<GcpWorkloadIdentityFederationToken, GcpWorkloadIdentityFederationError>,
    > + use<> {
        let client = self.client.clone();
        async move {
            match token_type.as_str() {
                gcp::TOKEN_TYPE_ID_TOKEN | gcp::TOKEN_TYPE_JWT => (),
                other => {
                    return Err(GcpWorkloadIdentityFederationError::new(format!(
                        "Unsupported token type `{other}`"
                    )));
                }
            }

            match client
                .issue_task_identity_token(IdentityTokenOptions {
                    audience,
                    requested_duration,
                    subject_template: vec1!["principal".to_owned()],
                })
                .await
            {
                Ok(token) => Ok(GcpWorkloadIdentityFederationToken::new(token, token_type)),
                Err(err) => Err(GcpWorkloadIdentityFederationError::new(err.to_string())),
            }
        }
    }
}

impl Entity for ManagedSecretManager {
    type Event = ();
}

impl SingletonEntity for ManagedSecretManager {}
