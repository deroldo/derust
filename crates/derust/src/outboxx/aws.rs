//! Wiring between the derust AWS config (`awsx::load_aws_config`) and the outbox processor clients.
//!
//! The clients are built straight from the [`SdkConfig`], so in `local`/`test` they target localstack
//! and elsewhere they use the default AWS credentials chain. The processor's own `SnsClient::new` is
//! intentionally not used: it reads `LOCAL_ENDPOINT`/`LOCAL_REGION` and replaces the credentials
//! (`from_sdk_config`, available since `outbox-pattern-processor` 0.4.1, uses the config as is).

use aws_config::SdkConfig;
use outbox_pattern_processor::aws::{SnsClient, SqsClient};
use outbox_pattern_processor::outbox_resources::OutboxProcessorResources;
use sqlx::{Pool, Postgres};

/// Outbox SNS client (`outbox_pattern_processor::aws::SnsClient`). Not the same type as `awsx::SnsClient`.
pub type OutboxSnsClient = SnsClient;
/// Outbox SQS client (`outbox_pattern_processor::aws::SqsClient`). Not the same type as `awsx::SqsClient`.
pub type OutboxSqsClient = SqsClient;

/// Which AWS destinations the outbox processor is able to publish to.
///
/// A destination without a client makes every outbox row for it fail and be retried, so enable
/// exactly the ones the service writes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxAwsClients {
    /// HTTP destinations only.
    None,
    Sns,
    Sqs,
    SnsAndSqs,
}

impl OutboxAwsClients {
    pub fn sns_enabled(self) -> bool {
        matches!(self, Self::Sns | Self::SnsAndSqs)
    }

    pub fn sqs_enabled(self) -> bool {
        matches!(self, Self::Sqs | Self::SnsAndSqs)
    }
}

/// Builds the outbox SNS client from the derust AWS config.
pub fn outbox_sns_client(aws_config: &SdkConfig) -> OutboxSnsClient {
    SnsClient::from_sdk_config(aws_config)
}

/// Builds the outbox SQS client from the derust AWS config.
pub fn outbox_sqs_client(aws_config: &SdkConfig) -> OutboxSqsClient {
    SqsClient::from_sdk_config(aws_config)
}

/// Extension to build [`OutboxProcessorResources`] from the derust AWS config. Import it with
/// `use derust::outboxx::OutboxProcessorResourcesExt;`.
pub trait OutboxProcessorResourcesExt: Sized {
    fn from_aws_config(
        postgres_pool: Pool<Postgres>,
        aws_config: &SdkConfig,
        clients: OutboxAwsClients,
    ) -> Self;
}

impl OutboxProcessorResourcesExt for OutboxProcessorResources {
    fn from_aws_config(
        postgres_pool: Pool<Postgres>,
        aws_config: &SdkConfig,
        clients: OutboxAwsClients,
    ) -> Self {
        OutboxProcessorResources::new(
            postgres_pool,
            clients.sqs_enabled().then(|| outbox_sqs_client(aws_config)),
            clients.sns_enabled().then(|| outbox_sns_client(aws_config)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_config::Region;
    use sqlx::postgres::PgPoolOptions;

    fn config() -> SdkConfig {
        SdkConfig::builder()
            .region(Region::from_static("us-east-1"))
            .endpoint_url("http://localstack.test:4566")
            .behavior_version(aws_config::BehaviorVersion::latest())
            .build()
    }

    fn pool() -> Pool<Postgres> {
        PgPoolOptions::new()
            .connect_lazy("postgres://u:p@localhost/db")
            .unwrap()
    }

    fn build(clients: OutboxAwsClients) -> OutboxProcessorResources {
        OutboxProcessorResources::from_aws_config(pool(), &config(), clients)
    }

    #[tokio::test]
    async fn enables_clients_independently() {
        let none = build(OutboxAwsClients::None);
        assert!(none.sns_client.is_none() && none.sqs_client.is_none());

        let sns = build(OutboxAwsClients::Sns);
        assert!(sns.sns_client.is_some() && sns.sqs_client.is_none());

        let sqs = build(OutboxAwsClients::Sqs);
        assert!(sqs.sns_client.is_none() && sqs.sqs_client.is_some());

        let both = build(OutboxAwsClients::SnsAndSqs);
        assert!(both.sns_client.is_some() && both.sqs_client.is_some());
    }

    #[tokio::test]
    async fn clients_inherit_region_and_endpoint_from_aws_config() {
        let sns = outbox_sns_client(&config());
        let sqs = outbox_sqs_client(&config());
        assert_eq!(sns.client.config().region().unwrap().as_ref(), "us-east-1");
        assert_eq!(sqs.client.config().region().unwrap().as_ref(), "us-east-1");
    }
}
