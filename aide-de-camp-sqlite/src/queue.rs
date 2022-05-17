use crate::job_handle::SqliteJobHandle;
use crate::types::JobRow;
use aide_de_camp_core::anyhow::Context;
use aide_de_camp_core::async_trait::async_trait;
use aide_de_camp_core::chrono::{DateTime, Utc};
use aide_de_camp_core::error::{JobError, QueueError};
use aide_de_camp_core::job::JobHandler;
use aide_de_camp_core::queue::Queue;
use aide_de_camp_core::{xid, Xid};
use bincode::{Decode, Encode};
use sqlx::{FromRow, QueryBuilder, SqlitePool};

#[derive(Clone)]
pub struct SqliteQueue {
    pool: SqlitePool,
    bincode_config: bincode::config::Configuration,
}

impl SqliteQueue {
    pub fn with_pool(pool: SqlitePool) -> Self {
        Self {
            pool,
            bincode_config: bincode::config::standard(),
        }
    }

    pub async fn print_job_queue(&self) -> Result<(), sqlx::Error> {
        let rows = sqlx::query_scalar!("SELECT jid FROM adc_queue")
            .fetch_all(&self.pool)
            .await?;
        println!("{:?}", rows);
        Ok(())
    }
}

#[async_trait]
impl Queue for SqliteQueue {
    type JobHandle = SqliteJobHandle;
    async fn schedule_at<J>(
        &self,
        payload: J::Payload,
        scheduled_at: DateTime<Utc>,
    ) -> Result<Xid, QueueError>
    where
        J: JobHandler + 'static,
        J::Payload: Decode + Encode,
        J::Error: Into<JobError>,
    {
        let payload = bincode::encode_to_vec(payload, self.bincode_config)?;
        let jid = xid::new();
        let jid_string = jid.to_string();
        let job_type = J::name();

        sqlx::query!(
            "INSERT INTO adc_queue (jid,job_type,payload,scheduled_at) VALUES (?1,?2,?3,?4)",
            jid_string,
            job_type,
            payload,
            scheduled_at
        )
        .execute(&self.pool)
        .await
        .context("Failed to add job to the queue")?;
        Ok(jid)
    }
    async fn poll_next_with_instant(
        &self,
        job_types: &[&str],
        now: DateTime<Utc>,
    ) -> Result<Option<SqliteJobHandle>, QueueError> {
        let mut builder = QueryBuilder::new(
            "UPDATE adc_queue SET started_at=(strftime('%s', 'now')),retries=retries+1 ",
        );
        let query = {
            builder.push(
                "WHERE jid IN (SELECT jid FROM adc_queue WHERE started_at IS NULL AND queue='default' AND scheduled_at <="
            );
            builder.push_bind(now);
            builder.push(" AND job_type IN (");
            {
                let mut separated = builder.separated(",");
                for job_type in job_types {
                    separated.push_bind(job_type);
                }
            }
            builder.push(") limit 1) RETURNING *");
            builder.build().bind(now)
        };
        let row = query
            .try_map(|row| JobRow::from_row(&row))
            .fetch_optional(&self.pool)
            .await
            .context("Failed to checkout out job from the queue")?;

        if let Some(row) = row {
            Ok(Some(SqliteJobHandle::new(row, self.pool.clone())))
        } else {
            Ok(None)
        }
    }
}
