mod secrets_manager;
mod sns;
mod sqs;

#[cfg(feature = "sqs")]
pub mod sqsx;

pub use secrets_manager::*;
pub use sns::*;
pub use sqs::*;

use crate::envx::Environment;
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_sdk_secretsmanager::config::Credentials;
use lazy_static::lazy_static;
use tracing::info;

const LOCALSTACK_ENDPOINT: &str = "http://localhost:4566";
const LOCALSTACK_ENDPOINT_ENV: &str = "LOCALSTACK_ENDPOINT";
const AWS_ENDPOINT_URL_ENV: &str = "AWS_ENDPOINT_URL";

lazy_static! {
    static ref LOCALSTACK_REGION: Region = Region::from_static("us-east-1");
    static ref LOCALSTACK_CREDENTIALS: Credentials =
        Credentials::new("test", "test", None, None, "test");
}

#[derive(Clone)]
pub struct SnsClient {
    pub client: aws_sdk_sns::Client,
}

#[derive(Clone)]
pub struct SqsClient {
    pub client: aws_sdk_sqs::Client,
}

#[derive(Clone)]
pub struct SecretsManagerClient {
    pub client: aws_sdk_secretsmanager::Client,
}

fn resolve_localstack_endpoint(localstack: Option<String>, aws: Option<String>) -> String {
    [localstack, aws]
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_string())
        .find(|value| !value.is_empty())
        .unwrap_or_else(|| LOCALSTACK_ENDPOINT.to_string())
}

/// Loads the AWS config. In `local`/`test` it targets localstack, at `http://localhost:4566` by default;
/// override it with the `LOCALSTACK_ENDPOINT` (or, as a fallback, `AWS_ENDPOINT_URL`) env var.
pub async fn load_aws_config(env: Environment) -> SdkConfig {
    if env.is_local() || env.is_test() {
        let endpoint = resolve_localstack_endpoint(
            std::env::var(LOCALSTACK_ENDPOINT_ENV).ok(),
            std::env::var(AWS_ENDPOINT_URL_ENV).ok(),
        );
        info!("Loading AWS config for localstack at {endpoint}");
        aws_config::from_env()
            .region(LOCALSTACK_REGION.clone())
            .credentials_provider(LOCALSTACK_CREDENTIALS.clone())
            .endpoint_url(endpoint)
            .load()
            .await
    } else {
        aws_config::load_defaults(BehaviorVersion::latest()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_localhost() {
        assert_eq!(resolve_localstack_endpoint(None, None), LOCALSTACK_ENDPOINT);
    }

    #[test]
    fn localstack_var_wins_over_aws_var() {
        assert_eq!(
            resolve_localstack_endpoint(Some("http://a:1".into()), Some("http://b:2".into())),
            "http://a:1"
        );
    }

    #[test]
    fn falls_back_to_aws_var_and_ignores_blank() {
        assert_eq!(
            resolve_localstack_endpoint(Some("  ".into()), Some("http://b:2".into())),
            "http://b:2"
        );
    }
}
