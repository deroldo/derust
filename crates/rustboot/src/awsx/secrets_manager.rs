use crate::awsx::SecretsManagerClient;
use aws_config::SdkConfig;
use aws_sdk_secretsmanager::Client;

pub fn secrets_manager(aws_sdk_config: &SdkConfig) -> SecretsManagerClient {
    SecretsManagerClient {
        client: Client::new(aws_sdk_config),
    }
}
