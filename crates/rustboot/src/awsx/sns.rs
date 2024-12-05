use aws_config::SdkConfig;
use outbox_pattern_processor::aws::SnsClient;

pub async fn sns_client(aws_sdk_config: &SdkConfig) -> SnsClient {
    SnsClient::new(aws_sdk_config).await
}