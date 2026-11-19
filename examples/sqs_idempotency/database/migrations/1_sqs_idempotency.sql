create table sqs_idempotency
(
    queue_url       text        not null,
    idempotency_key text        not null,
    status          text        not null check (status in ('in_progress', 'completed')),
    created_at      timestamptz not null default now(),
    expires_at      timestamptz not null,
    primary key (queue_url, idempotency_key)
);

create index idx_sqs_idempotency_by_expires_at on sqs_idempotency (expires_at);
