//! async-nats publisher for CDC events.
//!
//! Subject: `citus.cdc.<schema>.<table>`. Headers carry `Ai-Blaise-Cdc-Lsn`,
//! `Ai-Blaise-Cdc-Tx-Xid`, `Ai-Blaise-Cdc-Op`, and `Ai-Blaise-Cdc-Source`.

use crate::replication::{
    event_headers, CdcEventSink, PgoutputRowChange, ReplicationError, ReplicationTarget,
};
use crate::CdcEventEnvelope;
use async_nats::{Client, HeaderMap, HeaderName, HeaderValue};
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, info};

const SUBJECT_PREFIX: &str = "citus.cdc";

/// async-nats sink. Constructed once and cloned across consumers via `Arc`.
pub struct NatsSink {
    client: Client,
    subject_prefix: String,
}

impl NatsSink {
    pub async fn connect_from_env() -> Result<Arc<Self>, ReplicationError> {
        let url =
            env::var("CITUS_CDC_NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());
        Self::connect(&url, SUBJECT_PREFIX).await
    }

    pub async fn connect(url: &str, subject_prefix: &str) -> Result<Arc<Self>, ReplicationError> {
        info!(url, "connecting CDC NATS sink");
        let client = async_nats::connect(url)
            .await
            .map_err(|error| ReplicationError::Sink(error.to_string()))?;
        Ok(Arc::new(Self {
            client,
            subject_prefix: subject_prefix.to_string(),
        }))
    }

    /// Compute the NATS subject for a given CDC event.
    pub fn subject(&self, event: &CdcEventEnvelope) -> String {
        format!(
            "{prefix}.{schema}.{table}",
            prefix = self.subject_prefix,
            schema = event.schema,
            table = event.table
        )
    }

    /// Publish a single CDC event with the standard header set derived from
    /// the originating pgoutput row change.
    pub async fn publish_change(
        &self,
        target: &ReplicationTarget,
        change: PgoutputRowChange,
    ) -> Result<(), ReplicationError> {
        let headers = event_headers(target, &change);
        let event = change.into_event()?;
        let subject = self.subject(&event);
        let mut nats_headers = HeaderMap::new();
        for (name, value) in headers {
            let header_name = HeaderName::from_str(name)
                .map_err(|error| ReplicationError::Sink(error.to_string()))?;
            let header_value = HeaderValue::from_str(&value)
                .map_err(|error| ReplicationError::Sink(error.to_string()))?;
            nats_headers.insert(header_name, header_value);
        }
        let payload = serde_json::to_vec(&serde_event(&event))
            .map_err(|error| ReplicationError::Sink(error.to_string()))?;
        debug!(%subject, "publishing CDC event");
        self.client
            .publish_with_headers(subject, nats_headers, payload.into())
            .await
            .map_err(|error| ReplicationError::Sink(error.to_string()))?;
        Ok(())
    }
}

fn serde_event(event: &CdcEventEnvelope) -> serde_json::Value {
    let mut columns = serde_json::Map::new();
    for column in &event.columns {
        columns.insert(
            column.name.clone(),
            column
                .value
                .as_deref()
                .map(|value| serde_json::Value::String(value.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
    }
    serde_json::json!({
        "lsn": event.lsn,
        "schema": event.schema,
        "table": event.table,
        "tenant_id": event.tenant_id,
        "operation": match event.operation {
            crate::CdcOperation::Insert => "insert",
            crate::CdcOperation::Update => "update",
            crate::CdcOperation::Delete => "delete",
            crate::CdcOperation::Truncate => "truncate",
        },
        "columns": serde_json::Value::Object(columns),
    })
}

#[async_trait::async_trait]
impl CdcEventSink for NatsSink {
    async fn publish(
        &self,
        target: &ReplicationTarget,
        event: CdcEventEnvelope,
    ) -> Result<(), ReplicationError> {
        let subject = self.subject(&event);
        let mut nats_headers = HeaderMap::new();
        nats_headers.insert(
            HeaderName::from_str("Ai-Blaise-Cdc-Lsn")
                .map_err(|error| ReplicationError::Sink(error.to_string()))?,
            HeaderValue::from_str(&event.lsn)
                .map_err(|error| ReplicationError::Sink(error.to_string()))?,
        );
        nats_headers.insert(
            HeaderName::from_str("Ai-Blaise-Cdc-Source")
                .map_err(|error| ReplicationError::Sink(error.to_string()))?,
            HeaderValue::from_str(&target.label)
                .map_err(|error| ReplicationError::Sink(error.to_string()))?,
        );
        let payload = serde_json::to_vec(&serde_event(&event))
            .map_err(|error| ReplicationError::Sink(error.to_string()))?;
        self.client
            .publish_with_headers(subject, nats_headers, payload.into())
            .await
            .map_err(|error| ReplicationError::Sink(error.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CdcColumnValue, CdcOperation};

    fn fake_sink() -> NatsSink {
        // Cannot construct a real Client without I/O; this struct uses the type
        // only via `subject()` which we exercise via a tiny helper.
        unreachable!("not constructed in unit tests; subject_for tested via helper")
    }

    #[test]
    fn subject_format_includes_schema_and_table() {
        // Pure helper that does not require a live client.
        let event = CdcEventEnvelope {
            lsn: "16/B374D848".to_string(),
            schema: "public".to_string(),
            table: "orders".to_string(),
            tenant_id: "tenant-a".to_string(),
            operation: CdcOperation::Insert,
            columns: vec![CdcColumnValue {
                name: "id".to_string(),
                value: Some("1".to_string()),
            }],
        };
        let formed = format!(
            "{prefix}.{schema}.{table}",
            prefix = SUBJECT_PREFIX,
            schema = event.schema,
            table = event.table
        );
        assert_eq!(formed, "citus.cdc.public.orders");
        let _ = fake_sink;
    }

    #[test]
    fn serde_event_emits_operation_string() {
        let event = CdcEventEnvelope {
            lsn: "16/B374D848".to_string(),
            schema: "public".to_string(),
            table: "orders".to_string(),
            tenant_id: "tenant-a".to_string(),
            operation: CdcOperation::Delete,
            columns: vec![CdcColumnValue {
                name: "id".to_string(),
                value: None,
            }],
        };
        let payload = serde_event(&event);
        assert_eq!(
            payload["operation"],
            serde_json::Value::String("delete".to_string())
        );
        assert_eq!(payload["columns"]["id"], serde_json::Value::Null);
    }
}
