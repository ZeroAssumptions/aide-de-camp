CREATE TABLE IF NOT EXISTS adc_queue (
 jid TEXT PRIMARY KEY,
 queue TEXT NOT NULL default 'default',
 job_type TEXT not null,
 payload blob not null,
 retries int not null default 0,
 scheduled_at INTEGER not null,
 started_at INTEGER,
 enqueued_at INTEGER not null default (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS adc_dead_queue (
 jid TEXT PRIMARY KEY,
 queue TEXT NOT NULL,
 job_type TEXT not null,
 payload blob not null,
 retries int not null,
 scheduled_at INTEGER not null,
 started_at INTEGER not null,
 enqueued_at INTEGER not null,
 died_at INTEGER not null default (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS adc_queue_jobs ON adc_queue (
    scheduled_at asc,
    started_at asc,
    queue,
    job_type
);