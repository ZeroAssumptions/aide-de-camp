use aide_de_camp_core::bytes::Bytes;
use aide_de_camp_core::{DateTime, Xid};
use sqlx::sqlite::SqliteRow;
use sqlx::{Error, FromRow, Row};
use std::str::FromStr;

#[derive(Debug)]
pub(crate) struct JobRow {
    pub jid: Xid,
    pub job_type: String,
    pub payload: Bytes,
    pub retries: u32,
    pub scheduled_at: DateTime,
    pub enqueued_at: DateTime,
}

impl<'r> FromRow<'r, SqliteRow> for JobRow {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        let jid = row
            .try_get("jid")
            .map(Xid::from_str)?
            .map_err(|xid_err| Error::Decode(Box::new(xid_err)))?;
        let job_type = row.try_get("job_type")?;
        let payload = row.try_get::<Vec<u8>, _>("payload").map(Bytes::from)?;
        // Retry count is incremented every time job is checked out from the queue.
        // We decrement it by one  to "real" retry count.
        let retries: u32 = row.try_get("retries").map(|r: u32| r - 1)?;
        let scheduled_at = row.try_get("scheduled_at")?;
        let enqueued_at = row.try_get("enqueued_at")?;
        Ok(Self {
            jid,
            job_type,
            payload,
            retries,
            scheduled_at,
            enqueued_at,
        })
    }
}
