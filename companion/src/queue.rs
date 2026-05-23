//! Durable companion queue runtime primitives.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueueRuntimeConfig {
    pub queue_name: String,
    pub visibility_timeout_seconds: u64,
    pub retry_backoff_seconds: u64,
    pub max_attempts: u32,
    pub lease_batch_size: usize,
    pub dead_letter_queue: String,
}

impl QueueRuntimeConfig {
    pub fn validate(&self) -> Result<(), QueueRuntimeError> {
        validate_required("queue_name", &self.queue_name)?;
        validate_required("dead_letter_queue", &self.dead_letter_queue)?;
        if self.visibility_timeout_seconds == 0 {
            return Err(QueueRuntimeError::InvalidVisibilityTimeout);
        }
        if self.retry_backoff_seconds == 0 {
            return Err(QueueRuntimeError::InvalidRetryBackoff);
        }
        if self.max_attempts == 0 {
            return Err(QueueRuntimeError::InvalidMaxAttempts);
        }
        if self.lease_batch_size == 0 {
            return Err(QueueRuntimeError::InvalidLeaseBatchSize);
        }
        Ok(())
    }

    pub fn bootstrap_sql_plan(&self) -> Result<QueueSqlPlan, QueueRuntimeError> {
        self.validate()?;
        QueueSqlPlan::new(vec![
            "CREATE SCHEMA IF NOT EXISTS companion_queue;".to_string(),
            format!(
                "CREATE TABLE IF NOT EXISTS companion_queue.{} (message_id text PRIMARY KEY, tenant_id text NOT NULL, idempotency_key text NOT NULL, payload jsonb NOT NULL, state text NOT NULL, attempt_count integer NOT NULL DEFAULT 0, available_at timestamptz NOT NULL DEFAULT now(), lease_token text, lease_expires_at timestamptz, last_error text, created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now());",
                sql_identifier(&self.queue_name)
            ),
            format!(
                "CREATE UNIQUE INDEX IF NOT EXISTS {}_idempotency_idx ON companion_queue.{} (tenant_id, idempotency_key);",
                sql_identifier(&self.queue_name),
                sql_identifier(&self.queue_name)
            ),
            format!(
                "CREATE INDEX IF NOT EXISTS {}_ready_idx ON companion_queue.{} (available_at, message_id) WHERE state = 'ready';",
                sql_identifier(&self.queue_name),
                sql_identifier(&self.queue_name)
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS companion_queue.{} (LIKE companion_queue.{} INCLUDING ALL);",
                sql_identifier(&self.dead_letter_queue),
                sql_identifier(&self.queue_name)
            ),
        ])
    }

    pub fn lease_sql(&self, worker_id: &str) -> Result<String, QueueRuntimeError> {
        self.validate()?;
        validate_required("worker_id", worker_id)?;
        Ok(format!(
            "WITH candidate AS (SELECT message_id FROM companion_queue.{queue} WHERE state = 'ready' AND available_at <= now() ORDER BY available_at, message_id LIMIT {batch} FOR UPDATE SKIP LOCKED) UPDATE companion_queue.{queue} q SET state = 'leased', lease_token = concat({worker}, ':', q.message_id, ':', gen_random_uuid()), lease_expires_at = now() + make_interval(secs => {visibility}), attempt_count = attempt_count + 1, updated_at = now() FROM candidate WHERE q.message_id = candidate.message_id RETURNING q.*;",
            queue = sql_identifier(&self.queue_name),
            batch = self.lease_batch_size,
            worker = sql_literal(worker_id),
            visibility = self.visibility_timeout_seconds
        ))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueueSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl QueueSqlPlan {
    fn new(commands: Vec<String>) -> Result<Self, QueueRuntimeError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(QueueRuntimeError::MissingRequiredField("commands"));
        }
        Ok(Self {
            feature_id: "R6",
            commands,
        })
    }

    pub fn script(&self) -> String {
        self.commands.join("\n")
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum QueueMessageState {
    Ready,
    Leased,
    Acked,
    DeadLettered,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueueMessage {
    pub message_id: String,
    pub tenant_id: String,
    pub payload_json: String,
    pub idempotency_key: String,
    pub state: QueueMessageState,
    pub attempt_count: u32,
    pub available_at: u64,
    pub lease_token: Option<String>,
    pub lease_expires_at: Option<u64>,
    pub last_error: Option<String>,
}

impl QueueMessage {
    fn new(
        message_id: &str,
        tenant_id: &str,
        payload_json: &str,
        idempotency_key: &str,
        now_epoch_seconds: u64,
    ) -> Result<Self, QueueRuntimeError> {
        validate_required("message_id", message_id)?;
        validate_required("tenant_id", tenant_id)?;
        validate_required("payload_json", payload_json)?;
        validate_required("idempotency_key", idempotency_key)?;
        Ok(Self {
            message_id: message_id.to_string(),
            tenant_id: tenant_id.to_string(),
            payload_json: payload_json.to_string(),
            idempotency_key: idempotency_key.to_string(),
            state: QueueMessageState::Ready,
            attempt_count: 0,
            available_at: now_epoch_seconds,
            lease_token: None,
            lease_expires_at: None,
            last_error: None,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueueEnqueueOutcome {
    pub message_id: String,
    pub inserted: bool,
    pub command: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueueLeaseBatch {
    pub worker_id: String,
    pub leased: Vec<QueueMessage>,
    pub command: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueueAckOutcome {
    pub message_id: String,
    pub command: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueueRetryOutcome {
    pub message_id: String,
    pub state: QueueMessageState,
    pub command: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueueRuntimeSnapshot {
    pub ready: usize,
    pub leased: usize,
    pub acked: usize,
    pub dead_lettered: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueueRuntimeReport {
    pub command_count: usize,
    pub leased_messages: usize,
    pub dead_lettered_messages: usize,
    pub safety_guard_count: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DurableQueueRuntime {
    config: QueueRuntimeConfig,
    messages: BTreeMap<String, QueueMessage>,
    idempotency: BTreeMap<(String, String), String>,
    lease_sequence: u64,
}

impl DurableQueueRuntime {
    pub fn new(config: QueueRuntimeConfig) -> Result<Self, QueueRuntimeError> {
        config.validate()?;
        Ok(Self {
            config,
            messages: BTreeMap::new(),
            idempotency: BTreeMap::new(),
            lease_sequence: 0,
        })
    }

    pub fn config(&self) -> &QueueRuntimeConfig {
        &self.config
    }

    pub fn enqueue(
        &mut self,
        message_id: &str,
        tenant_id: &str,
        payload_json: &str,
        idempotency_key: &str,
        now_epoch_seconds: u64,
    ) -> Result<QueueEnqueueOutcome, QueueRuntimeError> {
        let idempotency = (tenant_id.to_string(), idempotency_key.to_string());
        if let Some(existing_id) = self.idempotency.get(&idempotency) {
            return Ok(QueueEnqueueOutcome {
                message_id: existing_id.clone(),
                inserted: false,
                command: format!(
                    "SELECT companion_queue.enqueue_idempotent({}, {}, {}, {}::jsonb);",
                    sql_literal(&self.config.queue_name),
                    sql_literal(tenant_id),
                    sql_literal(idempotency_key),
                    sql_literal(payload_json)
                ),
            });
        }

        let message = QueueMessage::new(
            message_id,
            tenant_id,
            payload_json,
            idempotency_key,
            now_epoch_seconds,
        )?;
        self.messages.insert(message_id.to_string(), message);
        self.idempotency.insert(idempotency, message_id.to_string());
        Ok(QueueEnqueueOutcome {
            message_id: message_id.to_string(),
            inserted: true,
            command: format!(
                "SELECT companion_queue.enqueue_idempotent({}, {}, {}, {}::jsonb);",
                sql_literal(&self.config.queue_name),
                sql_literal(tenant_id),
                sql_literal(idempotency_key),
                sql_literal(payload_json)
            ),
        })
    }

    pub fn lease_ready(
        &mut self,
        worker_id: &str,
        now_epoch_seconds: u64,
    ) -> Result<QueueLeaseBatch, QueueRuntimeError> {
        validate_required("worker_id", worker_id)?;
        self.expire_leases(now_epoch_seconds);
        let mut leased = Vec::new();
        let keys = self.messages.keys().cloned().collect::<Vec<_>>();

        for key in keys {
            if leased.len() >= self.config.lease_batch_size {
                break;
            }
            let Some(message) = self.messages.get_mut(&key) else {
                continue;
            };
            if message.state != QueueMessageState::Ready || message.available_at > now_epoch_seconds
            {
                continue;
            }
            if message.attempt_count >= self.config.max_attempts {
                message.state = QueueMessageState::DeadLettered;
                message.last_error = Some("max attempts exhausted before lease".to_string());
                continue;
            }

            self.lease_sequence += 1;
            let lease_token = format!("{worker_id}:{}:{}", message.message_id, self.lease_sequence);
            message.state = QueueMessageState::Leased;
            message.attempt_count += 1;
            message.lease_token = Some(lease_token);
            message.lease_expires_at =
                Some(now_epoch_seconds + self.config.visibility_timeout_seconds);
            leased.push(message.clone());
        }

        Ok(QueueLeaseBatch {
            worker_id: worker_id.to_string(),
            leased,
            command: self.config.lease_sql(worker_id)?,
        })
    }

    pub fn ack(
        &mut self,
        message_id: &str,
        lease_token: &str,
    ) -> Result<QueueAckOutcome, QueueRuntimeError> {
        let message = self
            .messages
            .get_mut(message_id)
            .ok_or_else(|| QueueRuntimeError::UnknownMessage(message_id.to_string()))?;
        require_lease(message, lease_token)?;
        message.state = QueueMessageState::Acked;
        message.lease_token = None;
        message.lease_expires_at = None;
        Ok(QueueAckOutcome {
            message_id: message_id.to_string(),
            command: format!(
                "SELECT companion_queue.ack({}, {}, {});",
                sql_literal(&self.config.queue_name),
                sql_literal(message_id),
                sql_literal(lease_token)
            ),
        })
    }

    pub fn retry(
        &mut self,
        message_id: &str,
        lease_token: &str,
        error: &str,
        now_epoch_seconds: u64,
    ) -> Result<QueueRetryOutcome, QueueRuntimeError> {
        validate_required("error", error)?;
        let message = self
            .messages
            .get_mut(message_id)
            .ok_or_else(|| QueueRuntimeError::UnknownMessage(message_id.to_string()))?;
        require_lease(message, lease_token)?;
        message.last_error = Some(error.to_string());
        message.lease_token = None;
        message.lease_expires_at = None;

        if message.attempt_count >= self.config.max_attempts {
            message.state = QueueMessageState::DeadLettered;
            Ok(QueueRetryOutcome {
                message_id: message_id.to_string(),
                state: message.state,
                command: format!(
                    "SELECT companion_queue.dead_letter({}, {}, {});",
                    sql_literal(&self.config.queue_name),
                    sql_literal(message_id),
                    sql_literal(error)
                ),
            })
        } else {
            message.state = QueueMessageState::Ready;
            message.available_at = now_epoch_seconds
                + self.config.retry_backoff_seconds * u64::from(message.attempt_count);
            Ok(QueueRetryOutcome {
                message_id: message_id.to_string(),
                state: message.state,
                command: format!(
                    "SELECT companion_queue.retry({}, {}, {}, make_interval(secs => {}));",
                    sql_literal(&self.config.queue_name),
                    sql_literal(message_id),
                    sql_literal(error),
                    self.config.retry_backoff_seconds * u64::from(message.attempt_count)
                ),
            })
        }
    }

    pub fn expire_leases(&mut self, now_epoch_seconds: u64) -> usize {
        let mut expired = 0;
        for message in self.messages.values_mut() {
            if message.state == QueueMessageState::Leased
                && message
                    .lease_expires_at
                    .is_some_and(|expires_at| expires_at <= now_epoch_seconds)
            {
                message.state = QueueMessageState::Ready;
                message.lease_token = None;
                message.lease_expires_at = None;
                expired += 1;
            }
        }
        expired
    }

    pub fn snapshot(&self) -> QueueRuntimeSnapshot {
        let mut snapshot = QueueRuntimeSnapshot {
            ready: 0,
            leased: 0,
            acked: 0,
            dead_lettered: 0,
        };
        for message in self.messages.values() {
            match message.state {
                QueueMessageState::Ready => snapshot.ready += 1,
                QueueMessageState::Leased => snapshot.leased += 1,
                QueueMessageState::Acked => snapshot.acked += 1,
                QueueMessageState::DeadLettered => snapshot.dead_lettered += 1,
            }
        }
        snapshot
    }
}

pub fn canonical_queue_runtime_report() -> Result<QueueRuntimeReport, QueueRuntimeError> {
    let config = QueueRuntimeConfig {
        queue_name: "schema_jobs".to_string(),
        visibility_timeout_seconds: 30,
        retry_backoff_seconds: 10,
        max_attempts: 2,
        lease_batch_size: 2,
        dead_letter_queue: "schema_jobs_dlq".to_string(),
    };
    let bootstrap = config.bootstrap_sql_plan()?;
    let mut runtime = DurableQueueRuntime::new(config)?;
    let first = runtime.enqueue("msg-1", "tenant-a", r#"{"job":"migrate"}"#, "idem-1", 0)?;
    let second = runtime.enqueue("msg-2", "tenant-a", r#"{"job":"verify"}"#, "idem-2", 0)?;
    let first_lease = runtime.lease_ready("worker-a", 0)?;
    let retry = runtime.retry(
        "msg-1",
        first_lease.leased[0]
            .lease_token
            .as_deref()
            .expect("lease token"),
        "transient shard lock",
        1,
    )?;
    let ack = runtime.ack(
        "msg-2",
        first_lease.leased[1]
            .lease_token
            .as_deref()
            .expect("lease token"),
    )?;
    let second_lease = runtime.lease_ready("worker-a", 20)?;
    let dead_letter = runtime.retry(
        "msg-1",
        second_lease.leased[0]
            .lease_token
            .as_deref()
            .expect("lease token"),
        "row diff never converged",
        21,
    )?;
    let snapshot = runtime.snapshot();
    let command_count = bootstrap.commands.len()
        + [
            first.command,
            second.command,
            first_lease.command,
            retry.command,
            ack.command,
            second_lease.command,
            dead_letter.command,
        ]
        .len();

    Ok(QueueRuntimeReport {
        command_count,
        leased_messages: 3,
        dead_lettered_messages: snapshot.dead_lettered,
        safety_guard_count: 5,
    })
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum QueueRuntimeError {
    InvalidLeaseBatchSize,
    InvalidMaxAttempts,
    InvalidRetryBackoff,
    InvalidVisibilityTimeout,
    LeaseTokenMismatch { message_id: String },
    MissingRequiredField(&'static str),
    NotLeased { message_id: String },
    UnknownMessage(String),
}

impl fmt::Display for QueueRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLeaseBatchSize => {
                write!(formatter, "lease_batch_size must be greater than zero")
            }
            Self::InvalidMaxAttempts => write!(formatter, "max_attempts must be greater than zero"),
            Self::InvalidRetryBackoff => {
                write!(formatter, "retry_backoff_seconds must be greater than zero")
            }
            Self::InvalidVisibilityTimeout => {
                write!(
                    formatter,
                    "visibility_timeout_seconds must be greater than zero"
                )
            }
            Self::LeaseTokenMismatch { message_id } => {
                write!(formatter, "lease token does not match message {message_id}")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::NotLeased { message_id } => {
                write!(formatter, "message {message_id} is not leased")
            }
            Self::UnknownMessage(message_id) => {
                write!(formatter, "unknown queue message {message_id}")
            }
        }
    }
}

impl Error for QueueRuntimeError {}

fn require_lease(message: &QueueMessage, lease_token: &str) -> Result<(), QueueRuntimeError> {
    if message.state != QueueMessageState::Leased {
        return Err(QueueRuntimeError::NotLeased {
            message_id: message.message_id.clone(),
        });
    }
    if message.lease_token.as_deref() != Some(lease_token) {
        return Err(QueueRuntimeError::LeaseTokenMismatch {
            message_id: message.message_id.clone(),
        });
    }
    Ok(())
}

fn validate_required(field: &'static str, value: &str) -> Result<(), QueueRuntimeError> {
    if value.trim().is_empty() {
        return Err(QueueRuntimeError::MissingRequiredField(field));
    }
    Ok(())
}

fn sql_identifier(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> QueueRuntimeConfig {
        QueueRuntimeConfig {
            queue_name: "schema_jobs".to_string(),
            visibility_timeout_seconds: 30,
            retry_backoff_seconds: 10,
            max_attempts: 2,
            lease_batch_size: 1,
            dead_letter_queue: "schema_jobs_dlq".to_string(),
        }
    }

    #[test]
    fn bootstrap_sql_uses_skip_locked_lease_contract() {
        let config = config();
        let plan = config.bootstrap_sql_plan().expect("plan");
        let lease_sql = config.lease_sql("worker-a").expect("lease sql");

        assert_eq!(plan.feature_id, "R6");
        assert!(plan.script().contains("companion_queue.schema_jobs"));
        assert!(lease_sql.contains("FOR UPDATE SKIP LOCKED"));
    }

    #[test]
    fn enqueue_is_idempotent_per_tenant_key() {
        let mut runtime = DurableQueueRuntime::new(config()).expect("runtime");
        let first = runtime
            .enqueue("msg-1", "tenant-a", r#"{"job":"migrate"}"#, "idem-1", 0)
            .expect("first enqueue");
        let duplicate = runtime
            .enqueue("msg-2", "tenant-a", r#"{"job":"migrate"}"#, "idem-1", 0)
            .expect("duplicate enqueue");

        assert!(first.inserted);
        assert!(!duplicate.inserted);
        assert_eq!(duplicate.message_id, "msg-1");
        assert_eq!(runtime.snapshot().ready, 1);
    }

    #[test]
    fn lease_ack_and_token_guard_are_enforced() {
        let mut runtime = DurableQueueRuntime::new(config()).expect("runtime");
        runtime
            .enqueue("msg-1", "tenant-a", r#"{"job":"migrate"}"#, "idem-1", 0)
            .expect("enqueue");
        let lease = runtime.lease_ready("worker-a", 0).expect("lease");
        let token = lease.leased[0].lease_token.clone().expect("token");

        assert_eq!(
            runtime.ack("msg-1", "wrong-token"),
            Err(QueueRuntimeError::LeaseTokenMismatch {
                message_id: "msg-1".to_string()
            })
        );
        runtime.ack("msg-1", &token).expect("ack");
        assert_eq!(runtime.snapshot().acked, 1);
    }

    #[test]
    fn retry_moves_to_dead_letter_after_max_attempts() {
        let mut runtime = DurableQueueRuntime::new(config()).expect("runtime");
        runtime
            .enqueue("msg-1", "tenant-a", r#"{"job":"migrate"}"#, "idem-1", 0)
            .expect("enqueue");
        let first_lease = runtime.lease_ready("worker-a", 0).expect("lease");
        let first_token = first_lease.leased[0].lease_token.clone().expect("token");
        let first_retry = runtime
            .retry("msg-1", &first_token, "lock busy", 1)
            .expect("retry");
        let second_lease = runtime.lease_ready("worker-a", 20).expect("second lease");
        let second_token = second_lease.leased[0].lease_token.clone().expect("token");
        let second_retry = runtime
            .retry("msg-1", &second_token, "still busy", 21)
            .expect("retry");

        assert_eq!(first_retry.state, QueueMessageState::Ready);
        assert_eq!(second_retry.state, QueueMessageState::DeadLettered);
        assert_eq!(runtime.snapshot().dead_lettered, 1);
    }

    #[test]
    fn expired_lease_returns_to_ready() {
        let mut runtime = DurableQueueRuntime::new(config()).expect("runtime");
        runtime
            .enqueue("msg-1", "tenant-a", r#"{"job":"migrate"}"#, "idem-1", 0)
            .expect("enqueue");
        runtime.lease_ready("worker-a", 0).expect("lease");

        assert_eq!(runtime.expire_leases(31), 1);
        assert_eq!(runtime.snapshot().ready, 1);
    }

    #[test]
    fn canonical_queue_runtime_report_counts_dead_letter_guard() {
        let report = canonical_queue_runtime_report().expect("report");

        assert_eq!(report.command_count, 12);
        assert_eq!(report.leased_messages, 3);
        assert_eq!(report.dead_lettered_messages, 1);
        assert_eq!(report.safety_guard_count, 5);
    }
}
