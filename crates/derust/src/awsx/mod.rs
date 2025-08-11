mod secrets_manager;
mod sns;
mod sqs;

pub use secrets_manager::*;
pub use sns::*;
pub use sqs::*;

use crate::envx::Environment;
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_sdk_secretsmanager::config::Credentials;
use lazy_static::lazy_static;
use tracing::info;

const LOCALSTACK_ENDPOINT: &str = "http://localhost:4566";

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

pub async fn load_aws_config(env: Environment) -> SdkConfig {
    if env.is_local() || env.is_test() {
        info!("Loading AWS config for localstack");
        aws_config::from_env()
            .region(LOCALSTACK_REGION.clone())
            .credentials_provider(LOCALSTACK_CREDENTIALS.clone())
            .endpoint_url(LOCALSTACK_ENDPOINT)
            .load()
            .await
    } else {
        aws_config::load_defaults(BehaviorVersion::latest()).await
    }
}
