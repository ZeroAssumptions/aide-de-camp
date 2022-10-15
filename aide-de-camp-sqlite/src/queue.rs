use crate::job_handle::SqliteJobHandle;
use crate::types::JobRow;
use aide_de_camp::core::job_processor::JobProcessor;
use aide_de_camp::core::queue::{Queue, QueueError};
use aide_de_camp::core::{bincode::Encode, new_xid, DateTime, Xid};
use anyhow::Context;
use async_trait::async_trait;
use bincode::Decode;
use sqlx::sqlite::SqliteQueryResult;
use sqlx::{FromRow, QueryBuilder, SqlitePool};
use tracing::instrument;

/// An implementation of the Queue backed by SQlite
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
}

#[async_trait]
impl Queue for SqliteQueue {
    type JobHandle = SqliteJobHandle;

    #[instrument(skip_all, err, ret, fields(job_type = J::name(), payload_size))]
    async fn schedule_at<J>(
        &self,
        payload: J::Payload,
        scheduled_at: DateTime,
        priority: i8,
    ) -> Result<Xid, QueueError>
    where
        J: JobProcessor + 'static,
        J::Payload: Encode,
    {
        let payload = bincode::encode_to_vec(&payload, self.bincode_config)?;
        let jid = new_xid();
        let jid_string = jid.to_string();
        let job_type = J::name();

        tracing::Span::current().record("payload_size", payload.len());

        sqlx::query!(
            "INSERT INTO adc_queue (jid,job_type,payload,scheduled_at,priority) VALUES (?1,?2,?3,?4,?5)",
            jid_string,
            job_type,
            payload,
            scheduled_at,
            priority
        )
        .execute(&self.pool)
        .await
        .context("Failed to add job to the queue")?;
        Ok(jid)
    }

    #[instrument(skip_all, err)]
    async fn poll_next_with_instant(
        &self,
        job_types: &[&str],
        now: DateTime,
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
            builder.push(") ORDER BY priority DESC LIMIT 1) RETURNING *");
            builder.build().bind(now)
        };
        let row = query
            .try_map(|row| JobRow::from_row(&row))
            .fetch_optional(&self.pool)
            .await
            .context("Failed to check out a job from the queue")?;

        if let Some(row) = row {
            Ok(Some(SqliteJobHandle::new(row, self.pool.clone())))
        } else {
            Ok(None)
        }
    }

    #[instrument(skip_all, err)]
    async fn cancel_job(&self, job_id: Xid) -> Result<(), QueueError> {
        let jid = job_id.to_string();
        let result: SqliteQueryResult = sqlx::query!(
            "DELETE FROM adc_queue WHERE started_at IS NULL and jid = ?",
            jid
        )
        .execute(&self.pool)
        .await
        .context("Failed to remove job from the queue")?;
        if result.rows_affected() == 0 {
            Err(QueueError::JobNotFound(job_id))
        } else {
            Ok(())
        }
    }

    #[allow(clippy::or_fun_call)]
    #[instrument(skip_all, err)]
    async fn unschedule_job<J>(&self, job_id: Xid) -> Result<J::Payload, QueueError>
    where
        J: JobProcessor + 'static,
        J::Payload: Decode,
    {
        let jid = job_id.to_string();
        let job_type = J::name();
        let payload = sqlx::query!(
            "DELETE FROM adc_queue WHERE started_at IS NULL and jid = ? AND job_type = ? RETURNING *",
            jid,
            job_type)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to remove job from the queue")?
        .map(|row| row.payload.unwrap_or_default())
        .ok_or(QueueError::JobNotFound(job_id))?;
        let (decoded, _) = bincode::decode_from_slice(&payload, self.bincode_config)?;
        Ok(decoded)
    }
}
