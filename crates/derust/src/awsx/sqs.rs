use crate::awsx::SqsClient;
use aws_config::SdkConfig;
use aws_sdk_sqs::Client;

pub async fn sqs_client(aws_sdk_config: &SdkConfig) -> SqsClient {
    SqsClient {
        client: Client::new(aws_sdk_config),
    }
}
