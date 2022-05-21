use crate::types::JobRow;
use aide_de_camp::core::job_handle::JobHandle;
use aide_de_camp::core::queue::QueueError;
use aide_de_camp::core::{Bytes, Xid};
use anyhow::Context;
use async_trait::async_trait;
use sqlx::SqlitePool;

pub struct SqliteJobHandle {
    pool: SqlitePool,
    row: JobRow,
}

#[async_trait]
impl JobHandle for SqliteJobHandle {
    fn id(&self) -> Xid {
        self.row.jid
    }

    fn job_type(&self) -> &str {
        &self.row.job_type
    }

    fn payload(&self) -> Bytes {
        self.row.payload.clone()
    }

    fn retries(&self) -> u32 {
        self.row.retries
    }

    async fn complete(mut self) -> Result<(), QueueError> {
        let jid = self.row.jid.to_string();
        sqlx::query!("DELETE FROM adc_queue where jid = ?1", jid)
            .execute(&self.pool)
            .await
            .context("Failed to mark job as completed")?;
        Ok(())
    }

    async fn fail(mut self) -> Result<(), QueueError> {
        let jid = self.row.jid.to_string();
        sqlx::query!("UPDATE adc_queue SET started_at=null WHERE jid = ?1", jid)
            .execute(&self.pool)
            .await
            .context("Failed to mark job as failed")?;
        Ok(())
    }

    async fn dead_queue(mut self) -> Result<(), QueueError> {
        let jid = self.row.jid.to_string();
        let retries = self.row.retries;
        let job_type = self.row.job_type.clone();
        let payload = self.row.payload.as_ref();
        let scheduled_at = self.row.scheduled_at;
        let enqueued_at = self.row.enqueued_at;

        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start transaction")?;
        sqlx::query!("DELETE FROM adc_queue WHERE jid = ?1", jid)
            .execute(&mut tx)
            .await
            .context("Failed to delete job from the queue")?;

        sqlx::query!("INSERT INTO adc_dead_queue (jid, job_type, payload, retries, scheduled_at, enqueued_at) VALUES (?1, ?2, ?3, ?4,?5,?6)",
            jid,
            job_type,
            payload,
            retries,
            scheduled_at,
            enqueued_at)
            .execute(&mut tx).await
            .context("Failed to move job to dead queue")?;
        Ok(())
    }
}

impl SqliteJobHandle {
    pub(crate) fn new(row: JobRow, pool: SqlitePool) -> Self {
        Self { row, pool }
    }
}
