use crate::shutdown_signal;
use outbox_pattern_processor::outbox_processor::OutboxProcessor;
use outbox_pattern_processor::outbox_resources::OutboxProcessorResources;
use tracing::info;
use wg::WaitGroup;

pub async fn run(wg: WaitGroup, outbox_processor_resources: OutboxProcessorResources) {
    if outbox_processor_resources
        .scheduled_clear_locked_partition
        .unwrap_or(false)
    {
        tokio::spawn(run_clear_locked_partition(
            wg.add(1),
            outbox_processor_resources.clone(),
        ));
    }

    info!("Started embedded outbox-pattern-processor");

    let _ = OutboxProcessor::new(outbox_processor_resources.clone())
        .with_graceful_shutdown(shutdown_signal())
        .init_process()
        .await;

    wg.done();

    info!("Embedded outbox-pattern-processor stopped");
}

async fn run_clear_locked_partition(
    wg: WaitGroup,
    outbox_processor_resources: OutboxProcessorResources,
) {
    info!("Started embedded outbox-pattern-processor clear locked partition");

    let _ = OutboxProcessor::new(outbox_processor_resources)
        .with_graceful_shutdown(shutdown_signal())
        .init_processed_locked_cleaner()
        .await;

    wg.done();

    info!("Embedded outbox-pattern-processor clear locked partition stopped");
}
