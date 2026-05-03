use std::{collections::HashMap, future::Future, sync::Arc, time::Duration};

use vec1::vec1;

use warp_graphql::managed_secrets::ManagedSecret;
use warpui::{Entity, SingletonEntity};

use crate::{
    ManagedSecretValue,
    client::{
        IdentityTokenOptions, ManagedSecretConfigs, ManagedSecretsClient, SecretOwner,
        TaskIdentityToken,
    },
    envelope::UploadKey,
    gcp::{self, GcpWorkloadIdentityFederationError, GcpWorkloadIdentityFederationToken},
};
use warp_graphql::queries::task_secrets::ManagedSecretValue as GqlManagedSecretValue;

/// Singleton model for working with Warp-managed secrets.
pub struct ManagedSecretManager {
    client: Arc<dyn ManagedSecretsClient>,
    actor_provider: Arc<dyn ActorProvider>,
}

pub trait ActorProvider: Send + Sync + 'static {
    fn actor_uid(&self) -> Option<String>;
}

impl ManagedSecretManager {
    pub fn new(
        client: Arc<dyn ManagedSecretsClient>,
        actor_provider: Arc<dyn ActorProvider>,
    ) -> Self {
        crate::envelope::init();
        Self {
            client,
            actor_provider,
        }
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
            let gql_secrets = client
                .get_task_secrets(task_id, workload_token.token)
                .await?;

            // Convert GQL ManagedSecretValue to our ManagedSecretValue
            let mut secrets = HashMap::new();
            for (name, gql_value) in gql_secrets {
                let value = match gql_value {
                    GqlManagedSecretValue::ManagedSecretRawValue(raw) => {
                        ManagedSecretValue::raw_value(raw.value)
                    }
                    GqlManagedSecretValue::ManagedSecretAnthropicApiKeyValue(v) => {
                        ManagedSecretValue::anthropic_api_key(v.api_key)
                    }
                    GqlManagedSecretValue::ManagedSecretAnthropicBedrockAccessKeyValue(v) => {
                        ManagedSecretValue::anthropic_bedrock_access_key(
                            v.aws_access_key_id,
                            v.aws_secret_access_key,
                            // aws_session_token is now optional on the server.
                            v.aws_session_token,
                            v.aws_region,
                        )
                    }
                    GqlManagedSecretValue::ManagedSecretAnthropicBedrockApiKeyValue(v) => {
                        ManagedSecretValue::anthropic_bedrock_api_key(
                            v.aws_bearer_token_bedrock,
                            v.aws_region,
                        )
                    }
                    GqlManagedSecretValue::Unknown => {
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

/// Find the public upload key corresponding to `owner`.
/// Returns an error if there's no such key in `configs`.
fn owner_public_key<'a>(
    configs: &'a ManagedSecretConfigs,
    owner: &SecretOwner,
) -> Result<&'a str, anyhow::Error> {
    match owner {
        SecretOwner::CurrentUser => configs
            .user_secrets
            .as_ref()
            .and_then(|config| config.public_key.as_deref())
            .ok_or_else(|| anyhow::anyhow!("No public key for user")),
        SecretOwner::Team { team_uid } => configs
            .team_secrets
            .get(team_uid)
            .and_then(|config| config.public_key.as_deref())
            .ok_or_else(|| anyhow::anyhow!("No public key for team {team_uid}")),
    }
}

impl Entity for ManagedSecretManager {
    type Event = ();
}

impl SingletonEntity for ManagedSecretManager {}
