table "queue" {
  schema = schema.main
  column "queue" {
    null    = false
    type    = text
    default = "default"
  }
  column "job_type" {
    null = false
    type = text
  }
  column "payload" {
    null = false
    type = blob
  }
  column "retries" {
    null    = false
    type    = int
    default = 0
  }
  column "scheduled_at" {
    null = false
    type = integer
  }
  column "started_at" {
    null = true
    type = integer
  }
  column "enqueued_at" {
    null    = false
    type    = integer
    default = sql("strftime('%s', 'now')")
  }
  column "jid" {
    null = true
    type = text
  }
  primary_key {
    columns = [column.jid]
  }
}
table "dead_queue" {
  schema = schema.main
  column "queue" {
    null    = false
    type    = text
    default = "default"
  }
  column "job_type" {
    null = false
    type = text
  }
  column "payload" {
    null = false
    type = blob
  }
  column "retries" {
    null    = false
    type    = int
    default = 0
  }
  column "error" {
    null = true
    type = text
  }
  column "scheduled_at" {
    null = false
    type = integer
  }
  column "started_at" {
    null = true
    type = integer
  }
  column "enqueued_at" {
    null = false
    type = integer
  }
  column "died_at" {
    null    = false
    type    = integer
    default = sql("strftime('%s', 'now')")
  }
  column "jid" {
    null = true
    type = text
  }
  primary_key {
    columns = [column.jid]
  }
}
schema "main" {
}
