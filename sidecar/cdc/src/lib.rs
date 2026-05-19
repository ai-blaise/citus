//! CDC sidecar contracts.

// FEATURE: C1
// FEATURE: C2
// FEATURE: C3
// FEATURE: C14
// FEATURE: C15
// FEATURE: L8
// FEATURE: WH3

use ai_blaise_citus_sidecar_shared::{
    CdcSink, CdcStreamContract, DeliveryRetryPolicy, SidecarContractError,
};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcSidecarPlan {
    pub stream: CdcStreamContract,
    pub slot: LogicalSlotPlan,
    pub sinks: Vec<CdcSinkPlan>,
    pub schema_capture: Option<SchemaCapturePlan>,
    pub anonymization: Vec<AnonymizationRule>,
}

impl CdcSidecarPlan {
    pub fn validate(&self) -> Result<(), CdcSidecarError> {
        self.stream.validate()?;
        self.slot.validate()?;
        if self.sinks.is_empty() {
            return Err(CdcSidecarError::MissingRequiredField("sinks"));
        }
        for sink in &self.sinks {
            sink.validate()?;
        }
        if let Some(schema_capture) = &self.schema_capture {
            schema_capture.validate()?;
        }
        for rule in &self.anonymization {
            rule.validate()?;
        }
        Ok(())
    }

    pub fn delivery_plan(
        &self,
        event: &CdcEventEnvelope,
    ) -> Result<CdcDeliveryPlan, CdcSidecarError> {
        self.validate()?;
        event.validate()?;

        let mut routed_sinks = Vec::with_capacity(self.sinks.len());
        for sink in &self.sinks {
            routed_sinks.push(SinkDeliveryPlan {
                sink: sink.name().to_string(),
                target: sink.target().to_string(),
                retry_policy: sink.retry_policy().clone(),
            });
        }

        Ok(CdcDeliveryPlan {
            event_lsn: event.lsn.clone(),
            table: format!("{}.{}", event.schema, event.table),
            operation: event.operation,
            routed_sinks,
            anonymized_columns: self
                .anonymization
                .iter()
                .filter(|rule| rule.matches_event(event))
                .map(|rule| rule.column.clone())
                .collect(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogicalSlotPlan {
    pub slot_name: String,
    pub publication_name: String,
    pub plugin: WalOutputPlugin,
    pub confirmed_flush_lsn: Option<String>,
}

impl LogicalSlotPlan {
    fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_identifier("slot.slot_name", &self.slot_name)?;
        validate_identifier("slot.publication_name", &self.publication_name)?;
        if let Some(lsn) = &self.confirmed_flush_lsn {
            validate_lsn(lsn)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum WalOutputPlugin {
    Wal2Json,
    PgOutput,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CdcSinkPlan {
    Webhook {
        name: String,
        url: String,
        retry_policy: DeliveryRetryPolicy,
    },
    Realtime {
        name: String,
        topic_prefix: String,
        retry_policy: DeliveryRetryPolicy,
    },
    AnalyticalMirror {
        name: String,
        mirror_name: String,
        storage_uri: String,
        retry_policy: DeliveryRetryPolicy,
    },
    Kafka {
        name: String,
        topic: String,
        bootstrap_servers: String,
        retry_policy: DeliveryRetryPolicy,
    },
    Nats {
        name: String,
        subject: String,
        server_url: String,
        retry_policy: DeliveryRetryPolicy,
    },
    PubSub {
        name: String,
        project_id: String,
        topic: String,
        retry_policy: DeliveryRetryPolicy,
    },
}

impl CdcSinkPlan {
    fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_required("sink.name", self.name())?;
        self.retry_policy().validate()?;

        match self {
            Self::Webhook { url, .. } => validate_http_url("sink.webhook.url", url),
            Self::Realtime { topic_prefix, .. } => {
                validate_required("sink.realtime.topic_prefix", topic_prefix)
            }
            Self::AnalyticalMirror {
                mirror_name,
                storage_uri,
                ..
            } => {
                validate_required("sink.analytical_mirror.mirror_name", mirror_name)?;
                validate_object_uri("sink.analytical_mirror.storage_uri", storage_uri)
            }
            Self::Kafka {
                topic,
                bootstrap_servers,
                ..
            } => {
                validate_required("sink.kafka.topic", topic)?;
                validate_required("sink.kafka.bootstrap_servers", bootstrap_servers)
            }
            Self::Nats {
                subject,
                server_url,
                ..
            } => {
                validate_required("sink.nats.subject", subject)?;
                validate_nats_url(server_url)
            }
            Self::PubSub {
                project_id, topic, ..
            } => {
                validate_required("sink.pubsub.project_id", project_id)?;
                validate_required("sink.pubsub.topic", topic)
            }
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Webhook { name, .. }
            | Self::Realtime { name, .. }
            | Self::AnalyticalMirror { name, .. }
            | Self::Kafka { name, .. }
            | Self::Nats { name, .. }
            | Self::PubSub { name, .. } => name,
        }
    }

    fn target(&self) -> &str {
        match self {
            Self::Webhook { url, .. } => url,
            Self::Realtime { topic_prefix, .. } => topic_prefix,
            Self::AnalyticalMirror { mirror_name, .. } => mirror_name,
            Self::Kafka { topic, .. } => topic,
            Self::Nats { subject, .. } => subject,
            Self::PubSub { topic, .. } => topic,
        }
    }

    fn retry_policy(&self) -> &DeliveryRetryPolicy {
        match self {
            Self::Webhook { retry_policy, .. }
            | Self::Realtime { retry_policy, .. }
            | Self::AnalyticalMirror { retry_policy, .. }
            | Self::Kafka { retry_policy, .. }
            | Self::Nats { retry_policy, .. }
            | Self::PubSub { retry_policy, .. } => retry_policy,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SchemaCapturePlan {
    pub ddl_stream_table: String,
    pub include_schemas: Vec<String>,
    pub fail_on_parse_error: bool,
}

impl SchemaCapturePlan {
    fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_qualified_name("schema_capture.ddl_stream_table", &self.ddl_stream_table)?;
        validate_required_list("schema_capture.include_schemas", &self.include_schemas)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AnonymizationRule {
    pub schema: String,
    pub table: String,
    pub column: String,
    pub strategy: AnonymizationStrategy,
}

impl AnonymizationRule {
    fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_identifier("anonymization.schema", &self.schema)?;
        validate_identifier("anonymization.table", &self.table)?;
        validate_identifier("anonymization.column", &self.column)
    }

    fn matches_event(&self, event: &CdcEventEnvelope) -> bool {
        self.schema == event.schema
            && self.table == event.table
            && event
                .columns
                .iter()
                .any(|column| column.name == self.column)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AnonymizationStrategy {
    Redact,
    Hash,
    Null,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcEventEnvelope {
    pub lsn: String,
    pub schema: String,
    pub table: String,
    pub tenant_id: String,
    pub operation: CdcOperation,
    pub columns: Vec<CdcColumnValue>,
}

impl CdcEventEnvelope {
    pub fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_lsn(&self.lsn)?;
        validate_identifier("event.schema", &self.schema)?;
        validate_identifier("event.table", &self.table)?;
        validate_required("event.tenant_id", &self.tenant_id)?;
        if self.columns.is_empty() {
            return Err(CdcSidecarError::MissingRequiredField("event.columns"));
        }
        for column in &self.columns {
            column.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CdcOperation {
    Insert,
    Update,
    Delete,
    Truncate,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcColumnValue {
    pub name: String,
    pub value: Option<String>,
}

impl CdcColumnValue {
    fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_identifier("event.column.name", &self.name)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcDeliveryPlan {
    pub event_lsn: String,
    pub table: String,
    pub operation: CdcOperation,
    pub routed_sinks: Vec<SinkDeliveryPlan>,
    pub anonymized_columns: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SinkDeliveryPlan {
    pub sink: String,
    pub target: String,
    pub retry_policy: DeliveryRetryPolicy,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CdcSidecarError {
    InvalidHttpUrl(&'static str),
    InvalidIdentifier(&'static str),
    InvalidLsn,
    InvalidNatsUrl,
    InvalidObjectUri(&'static str),
    MissingRequiredField(&'static str),
    SharedContract(String),
}

impl fmt::Display for CdcSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHttpUrl(field) => {
                write!(formatter, "{field} must start with http:// or https://")
            }
            Self::InvalidIdentifier(field) => {
                write!(formatter, "{field} must be a non-empty SQL identifier")
            }
            Self::InvalidLsn => write!(formatter, "LSN must use the PostgreSQL HEX/HEX form"),
            Self::InvalidNatsUrl => write!(formatter, "NATS server URL must start with nats://"),
            Self::InvalidObjectUri(field) => write!(formatter, "{field} must be an object URI"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::SharedContract(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for CdcSidecarError {}

impl From<SidecarContractError> for CdcSidecarError {
    fn from(error: SidecarContractError) -> Self {
        Self::SharedContract(error.to_string())
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    if value.trim().is_empty() {
        return Err(CdcSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(field: &'static str, values: &[String]) -> Result<(), CdcSidecarError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(CdcSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    validate_required(field, value)?;
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidIdentifier(field))
    }
}

fn validate_qualified_name(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    validate_required(field, value)?;
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() == 2
        && parts
            .iter()
            .all(|part| validate_identifier(field, part).is_ok())
    {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidIdentifier(field))
    }
}

fn validate_lsn(value: &str) -> Result<(), CdcSidecarError> {
    validate_required("lsn", value)?;
    let parts: Vec<_> = value.split('/').collect();
    if parts.len() == 2
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_hexdigit()))
    {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidLsn)
    }
}

fn validate_http_url(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    validate_required(field, value)?;
    if value.starts_with("http://") || value.starts_with("https://") {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidHttpUrl(field))
    }
}

fn validate_object_uri(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    validate_required(field, value)?;
    if value.starts_with("s3://") || value.starts_with("gs://") || value.starts_with("az://") {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidObjectUri(field))
    }
}

fn validate_nats_url(value: &str) -> Result<(), CdcSidecarError> {
    validate_required("sink.nats.server_url", value)?;
    if value.starts_with("nats://") {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidNatsUrl)
    }
}

pub fn sink_plan_from_shared(
    sink: CdcSink,
    retry_policy: DeliveryRetryPolicy,
) -> Result<CdcSinkPlan, CdcSidecarError> {
    let plan = match sink {
        CdcSink::Webhook { url } => CdcSinkPlan::Webhook {
            name: "webhook".to_string(),
            url,
            retry_policy,
        },
        CdcSink::Realtime { topic_prefix } => CdcSinkPlan::Realtime {
            name: "realtime".to_string(),
            topic_prefix,
            retry_policy,
        },
        CdcSink::AnalyticalMirror { stream_name } => CdcSinkPlan::AnalyticalMirror {
            name: "analytical-mirror".to_string(),
            mirror_name: stream_name,
            storage_uri: "s3://analytical-mirror/default".to_string(),
            retry_policy,
        },
        CdcSink::Kafka { topic } => CdcSinkPlan::Kafka {
            name: "kafka".to_string(),
            topic,
            bootstrap_servers: "localhost:9092".to_string(),
            retry_policy,
        },
        CdcSink::Nats { subject } => CdcSinkPlan::Nats {
            name: "nats".to_string(),
            subject,
            server_url: "nats://localhost:4222".to_string(),
            retry_policy,
        },
        CdcSink::PubSub { project_id, topic } => CdcSinkPlan::PubSub {
            name: "pubsub".to_string(),
            project_id,
            topic,
            retry_policy,
        },
    };
    plan.validate()?;
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdc_sidecar_plan_routes_event_to_all_sinks() {
        let plan = valid_plan();
        let delivery = plan
            .delivery_plan(&valid_event())
            .expect("delivery plan should route");

        assert_eq!(delivery.event_lsn, "16/B374D848");
        assert_eq!(delivery.table, "public.orders");
        assert_eq!(delivery.routed_sinks.len(), 3);
        assert_eq!(delivery.anonymized_columns, vec!["email".to_string()]);
    }

    #[test]
    fn logical_slot_rejects_invalid_lsn() {
        let mut plan = valid_slot();
        plan.confirmed_flush_lsn = Some("not-an-lsn".to_string());

        assert_eq!(plan.validate(), Err(CdcSidecarError::InvalidLsn));
    }

    #[test]
    fn nats_sink_requires_nats_url() {
        let sink = CdcSinkPlan::Nats {
            name: "nats".to_string(),
            subject: "tenant.orders".to_string(),
            server_url: "http://nats".to_string(),
            retry_policy: valid_retry_policy(),
        };

        assert_eq!(sink.validate(), Err(CdcSidecarError::InvalidNatsUrl));
    }

    #[test]
    fn pubsub_sink_requires_project_id() {
        let sink = CdcSinkPlan::PubSub {
            name: "pubsub".to_string(),
            project_id: " ".to_string(),
            topic: "orders".to_string(),
            retry_policy: valid_retry_policy(),
        };

        assert_eq!(
            sink.validate(),
            Err(CdcSidecarError::MissingRequiredField(
                "sink.pubsub.project_id"
            ))
        );
    }

    #[test]
    fn schema_capture_requires_qualified_table() {
        let schema_capture = SchemaCapturePlan {
            ddl_stream_table: "ddl_events".to_string(),
            include_schemas: vec!["public".to_string()],
            fail_on_parse_error: true,
        };

        assert_eq!(
            schema_capture.validate(),
            Err(CdcSidecarError::InvalidIdentifier(
                "schema_capture.ddl_stream_table"
            ))
        );
    }

    fn valid_plan() -> CdcSidecarPlan {
        CdcSidecarPlan {
            stream: CdcStreamContract {
                slot_name: "ai_blaise_cdc".to_string(),
                publication_name: "ai_blaise_publication".to_string(),
                sinks: vec![
                    CdcSink::Nats {
                        subject: "tenant.orders".to_string(),
                    },
                    CdcSink::PubSub {
                        project_id: "analytics-prod".to_string(),
                        topic: "orders".to_string(),
                    },
                ],
                retry_policy: valid_retry_policy(),
            },
            slot: valid_slot(),
            sinks: vec![
                CdcSinkPlan::Webhook {
                    name: "webhook".to_string(),
                    url: "https://hooks.example.com/orders".to_string(),
                    retry_policy: valid_retry_policy(),
                },
                CdcSinkPlan::Nats {
                    name: "nats".to_string(),
                    subject: "tenant.orders".to_string(),
                    server_url: "nats://nats.cdc.svc:4222".to_string(),
                    retry_policy: valid_retry_policy(),
                },
                CdcSinkPlan::PubSub {
                    name: "pubsub".to_string(),
                    project_id: "analytics-prod".to_string(),
                    topic: "orders".to_string(),
                    retry_policy: valid_retry_policy(),
                },
            ],
            schema_capture: Some(SchemaCapturePlan {
                ddl_stream_table: "cdc.ddl_events".to_string(),
                include_schemas: vec!["public".to_string()],
                fail_on_parse_error: true,
            }),
            anonymization: vec![AnonymizationRule {
                schema: "public".to_string(),
                table: "orders".to_string(),
                column: "email".to_string(),
                strategy: AnonymizationStrategy::Hash,
            }],
        }
    }

    fn valid_slot() -> LogicalSlotPlan {
        LogicalSlotPlan {
            slot_name: "ai_blaise_cdc".to_string(),
            publication_name: "ai_blaise_publication".to_string(),
            plugin: WalOutputPlugin::Wal2Json,
            confirmed_flush_lsn: Some("16/B374D848".to_string()),
        }
    }

    fn valid_event() -> CdcEventEnvelope {
        CdcEventEnvelope {
            lsn: "16/B374D848".to_string(),
            schema: "public".to_string(),
            table: "orders".to_string(),
            tenant_id: "tenant-a".to_string(),
            operation: CdcOperation::Insert,
            columns: vec![
                CdcColumnValue {
                    name: "id".to_string(),
                    value: Some("1".to_string()),
                },
                CdcColumnValue {
                    name: "email".to_string(),
                    value: Some("person@example.com".to_string()),
                },
            ],
        }
    }

    fn valid_retry_policy() -> DeliveryRetryPolicy {
        DeliveryRetryPolicy {
            max_attempts: 5,
            dead_letter_queue: "cdc.dead_letters".to_string(),
        }
    }
}
