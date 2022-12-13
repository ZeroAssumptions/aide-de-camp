# aide-de-camp-sqlite

A SQLite backed implementation of the job Queue.

**NOTE**: It is possible that a single job gets sent to two runners. This is due to SQLite lacking
row locking and `BEGIN EXCLUSIVE TRANSACTION` not working well (it was very slow) for this use
case. This is only an issue at high concurrency, in which case you probably don't want to use
SQLite in the first place. In other words, this isn't “Exactly Once” kind of queue.

## Schema

```sql
CREATE TABLE IF NOT EXISTS adc_queue (
 jid TEXT PRIMARY KEY,
 queue TEXT NOT NULL default 'default',
 job_type TEXT not null,
 payload blob not null,
 retries int not null default 0,
 scheduled_at INTEGER not null,
 started_at INTEGER,
 enqueued_at INTEGER not null default (strftime('%s', 'now')),
 priority TINYINT not null default 0,
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
 died_at INTEGER not null default (strftime('%s', 'now')),
 priority TINYINT not null default 0,
);

CREATE INDEX IF NOT EXISTS adc_queue_jobs ON adc_queue (
    scheduled_at asc,
    started_at asc,
    queue,
    job_type
);
```

Crate includes [SQLx `MIGRATOR`](https://docs.rs/sqlx/0.4.0-beta.1/sqlx/macro.migrate.html) that could be used to manage schema.

**NOTE:** [SQLx doesn't support](https://github.com/launchbadge/sqlx/issues/1698) multiple migrators in the same database. That means ADC should either have dedicated schema/database or it's your responsibility to apply migrations.

## Example

```rust

use aide_de_camp_sqlite::{SqliteQueue, MIGRATOR};
use aide_de_camp::prelude::{Queue, JobProcessor, JobRunner, RunnerOptions, RunnerRouter, Duration, Xid, CancellationToken};
use async_trait::async_trait;
use sqlx::SqlitePool;

struct MyJob;

#[async_trait::async_trait]
impl JobProcessor for MyJob {
    type Payload = Vec<u32>;
    type Error = anyhow::Error;

    async fn handle(&self, jid: Xid, payload: Self::Payload, cancellation_token: CancellationToken) -> Result<(), Self::Error> {
        // Do work here
        Ok(())
    }

    fn name() -> &'static str {
        "my_job"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let pool = SqlitePool::connect(":memory:").await?;
    // Setup schema, alternatively you can add schema to your migrations.
    MIGRATOR.run(&pool).await?;
    let queue = SqliteQueue::with_pool(pool);

    // Add job the queue to run next
    let _jid = queue.schedule::<MyJob>(vec![1,2,3], 0).await?;

    // First create a job processor and router
    let router = {
        let mut r = RunnerRouter::default();
        r.add_job_handler(MyJob);
        r
    };
    // Setup runner to at most 10 jobs concurrently
    let mut runner = JobRunner::new(queue, router, 10, RunnerOptions::default());
    // Poll the queue every second, this will block unless something went really wrong.
    // The future supplied as the second parameter will tell the server to shut down when it completes.
    runner.run_with_shutdown(Duration::seconds(1), async move {
        // To avoid blocking this doctest, run for 10 milliseconds, then initiate shutdown.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // In a real application, you may want to wait for a CTRL+C event or something similar.
        // You could do this with tokio using the signal module: tokio::signal::ctrl_c().await.expect("failed to install CTRL+C signal handler");
    }).await?;
    Ok(())
}
```