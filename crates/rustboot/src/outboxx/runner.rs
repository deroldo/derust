use crate::shutdown_signal;
use outbox_pattern_processor::outbox_processor::OutboxProcessor;
use outbox_pattern_processor::outbox_resources::OutboxProcessorResources;
use tracing::info;
use wg::WaitGroup;

pub async fn run(
    wg: WaitGroup,
    outbox_processor_resources: OutboxProcessorResources,
) {
    info!("Starting embedded outbox-pattern-processor");

    let _ = OutboxProcessor::new(outbox_processor_resources.clone())
        .with_graceful_shutdown(shutdown_signal())
        .init_process()
        .await;

    if outbox_processor_resources.scheduled_clear_locked_partition.unwrap_or(false) {
        let _ = OutboxProcessor::new(outbox_processor_resources)
            .with_graceful_shutdown(shutdown_signal())
            .init_processed_locked_cleaner()
            .await;
    }

    wg.done();

    info!("Embedded outbox-pattern-processor stopped");
}