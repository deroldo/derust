use aws_config::SdkConfig;
use outbox_pattern_processor::aws::SqsClient;

pub async fn sqs_client(aws_sdk_config: &SdkConfig) -> SqsClient {
    SqsClient::new(aws_sdk_config).await
}