create table {table}
(
    queue_url       text        not null,
    idempotency_key text        not null,
    status          text        not null check (status in ('in_progress', 'completed')),
    created_at      timestamptz not null default now(),
    expires_at      timestamptz not null,
    primary key (queue_url, idempotency_key)
);

create index idx_{index}_by_expires_at on {table} (expires_at);
