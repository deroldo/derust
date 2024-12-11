use crate::awsx::SnsClient;
use aws_config::SdkConfig;
use aws_sdk_sns::Client;

pub async fn sns_client(aws_sdk_config: &SdkConfig) -> SnsClient {
    SnsClient {
        client: Client::new(aws_sdk_config),
    }
}
