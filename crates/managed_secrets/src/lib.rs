pub mod client;
mod envelope;
mod gcp;
mod manager;
mod secret_value;

pub use client::TaskIdentityToken;
pub use client::{
    ManagedSecret, ManagedSecretConfig, ManagedSecretOwner, ManagedSecretType,
    TaskManagedSecretValue,
};
pub use envelope::{init as init_envelope, UploadKey};
pub use gcp::{
    GcpCredentials, GcpFederationConfig, GcpWorkloadIdentityFederationError,
    GcpWorkloadIdentityFederationToken, PrepareGcpCredentialsError,
};
pub use manager::{ActorProvider, ManagedSecretManager};
pub use secret_value::ManagedSecretValue;
