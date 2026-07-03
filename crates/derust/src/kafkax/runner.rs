use std::time::Duration;

use rdkafka::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message as KafkaMessage;
use rdkafka::{Offset, TopicPartitionList};
use tracing::{error, info};
use wg::WaitGroup;

use super::consumer::{KafkaTopicConsumer, Message};
use crate::httpx::AppContext;
use crate::shutdown_signal;

#[cfg(any(feature = "statsd", feature = "prometheus"))]
use crate::metricx::{MetricTags, start_stopwatch};

pub async fn run<T>(
    wg: WaitGroup,
    context: AppContext<T>,
    consumers: Vec<KafkaTopicConsumer<T>>,
) where
    T: Clone + Send + Sync + 'static,
{
    let mut handles = Vec::new();
    for consumer in consumers {
        info!(
            "Started Kafka consumer for topic {} with concurrency {}",
            consumer.topic, consumer.concurrency
        );
        for _ in 0..consumer.concurrency {
            let ctx = context.clone();
            let c = consumer.clone();
            handles.push(tokio::spawn(poll_topic(ctx, c)));
        }
    }

    for handle in handles {
        let _ = handle.await;
    }

    wg.done();

    info!("Kafka consumers stopped");
}

async fn poll_topic<T>(context: AppContext<T>, consumer: KafkaTopicConsumer<T>)
where
    T: Clone + Send + Sync + 'static,
{
    let mut config = ClientConfig::new();
    config
        .set("bootstrap.servers", &consumer.cluster_config.brokers)
        .set("group.id", &consumer.cluster_config.group_id)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest");

    for (key, value) in &consumer.cluster_config.extra_config {
        config.set(key.as_str(), value.as_str());
    }

    let stream_consumer: StreamConsumer = match config.create() {
        Ok(c) => c,
        Err(e) => {
            error!(
                "Failed to create Kafka consumer for topic {}: {:?}",
                consumer.topic, e
            );
            return;
        }
    };

    if let Err(e) = stream_consumer.subscribe(&[consumer.topic.as_str()]) {
        error!("Failed to subscribe to Kafka topic {}: {:?}", consumer.topic, e);
        return;
    }

    let mut shutdown = Box::into_pin(Box::new(shutdown_signal()));

    loop {
        tokio::select! {
            msg_result = stream_consumer.recv() => {
                match msg_result {
                    Ok(borrowed_msg) => {
                        let payload = borrowed_msg.payload().map(|b| b.to_vec());
                        let key = borrowed_msg.key().map(|b| b.to_vec());
                        let topic = borrowed_msg.topic().to_string();
                        let partition = borrowed_msg.partition();
                        let offset = borrowed_msg.offset();
                        drop(borrowed_msg);

                        let msg = Message::new(payload, key, topic.clone(), partition, offset);
                        let success = process_message(&context, &consumer, msg).await;

                        if success {
                            let mut tpl = TopicPartitionList::new();
                            if tpl.add_partition_offset(&topic, partition, Offset::Offset(offset + 1)).is_ok() {
                                let _ = stream_consumer.commit(&tpl, CommitMode::Async);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Kafka receive error on topic {}: {:?}", consumer.topic, e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
            _ = &mut shutdown => {
                info!("Kafka consumer for topic {} stopped", consumer.topic);
                break;
            }
        }
    }
}

async fn process_message<T>(
    context: &AppContext<T>,
    consumer: &KafkaTopicConsumer<T>,
    msg: Message,
) -> bool
where
    T: Clone + Send + Sync + 'static,
{
    let topic = msg.topic().to_string();
    let partition = msg.partition();
    let offset = msg.offset();

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    let metric_tags = MetricTags::default().push("topic".to_string(), topic.clone());

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    let stopwatch = start_stopwatch(context, "kafka_consumer_seconds", metric_tags);

    let result = (consumer.handler)(context.clone(), msg).await;

    let success = match &result {
        Ok(_) => {
            info!(
                "Kafka message consumed :: topic={} :: partition={} :: offset={}",
                topic, partition, offset
            );
            true
        }
        Err(err) => {
            error!(
                "Kafka message failed :: topic={} :: partition={} :: offset={} :: error={:?}",
                topic, partition, offset, err
            );
            false
        }
    };

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    {
        let success_str = if success { "true" } else { "false" };
        stopwatch.record(MetricTags::default().push("success".to_string(), success_str.to_string()));
    }

    success
}
