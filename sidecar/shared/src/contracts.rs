// FEATURE: Auth1
// FEATURE: B1
// FEATURE: B3
// FEATURE: B4
// FEATURE: C1
// FEATURE: C14
// FEATURE: C15
// FEATURE: L8
// FEATURE: R7
// FEATURE: RT1
// FEATURE: RT2
// FEATURE: RT3
// FEATURE: RT4
// FEATURE: Search8
// FEATURE: Sto1
// FEATURE: Sto3
// FEATURE: Sto4
// FEATURE: WH3

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SidecarRuntimeContracts {
    pub cdc: CdcStreamContract,
    pub realtime: RealtimeContract,
    pub auth: AuthIssuerContract,
    pub storage: StorageContract,
    pub backup_restore: BackupRestoreContract,
    pub repack: RepackContract,
    pub analytical_mirror: AnalyticalMirrorContract,
}

impl SidecarRuntimeContracts {
    pub fn validate(&self) -> Result<(), SidecarContractError> {
        self.cdc.validate()?;
        self.realtime.validate()?;
        self.auth.validate()?;
        self.storage.validate()?;
        self.backup_restore.validate()?;
        self.repack.validate()?;
        self.analytical_mirror.validate()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcStreamContract {
    pub slot_name: String,
    pub publication_name: String,
    pub sinks: Vec<CdcSink>,
    pub retry_policy: DeliveryRetryPolicy,
}

impl CdcStreamContract {
    pub fn validate(&self) -> Result<(), SidecarContractError> {
        validate_required("cdc.slot_name", &self.slot_name)?;
        validate_required("cdc.publication_name", &self.publication_name)?;
        if self.sinks.is_empty() {
            return Err(SidecarContractError::MissingRequiredField("cdc.sinks"));
        }
        for sink in &self.sinks {
            sink.validate()?;
        }
        self.retry_policy.validate()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CdcSink {
    Webhook { url: String },
    Realtime { topic_prefix: String },
    AnalyticalMirror { stream_name: String },
    Kafka { topic: String },
    Nats { subject: String },
    PubSub { project_id: String, topic: String },
}

impl CdcSink {
    fn validate(&self) -> Result<(), SidecarContractError> {
        match self {
            Self::Webhook { url } => {
                validate_required("cdc.sinks.webhook.url", url)?;
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    return Err(SidecarContractError::InvalidUrl);
                }
                Ok(())
            }
            Self::Realtime { topic_prefix } => {
                validate_required("cdc.sinks.realtime.topic_prefix", topic_prefix)
            }
            Self::AnalyticalMirror { stream_name } => {
                validate_required("cdc.sinks.analytical_mirror.stream_name", stream_name)
            }
            Self::Kafka { topic } => validate_required("cdc.sinks.kafka.topic", topic),
            Self::Nats { subject } => validate_nats_subject("cdc.sinks.nats.subject", subject),
            Self::PubSub { project_id, topic } => {
                validate_pubsub_project_id("cdc.sinks.pubsub.project_id", project_id)?;
                validate_pubsub_topic("cdc.sinks.pubsub.topic", topic)
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DeliveryRetryPolicy {
    pub max_attempts: u32,
    pub dead_letter_queue: String,
}

impl DeliveryRetryPolicy {
    pub fn validate(&self) -> Result<(), SidecarContractError> {
        if self.max_attempts == 0 {
            return Err(SidecarContractError::InvalidRetryAttempts);
        }
        validate_required("retry_policy.dead_letter_queue", &self.dead_letter_queue)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeContract {
    pub topic: String,
    pub tenant_id: String,
    pub filters: Vec<String>,
    pub presence_enabled: bool,
}

impl RealtimeContract {
    pub fn validate(&self) -> Result<(), SidecarContractError> {
        validate_required("realtime.topic", &self.topic)?;
        validate_required("realtime.tenant_id", &self.tenant_id)?;
        validate_optional_list("realtime.filters", &self.filters)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AuthIssuerContract {
    pub issuer: String,
    pub signing_key_ref: String,
    pub token_ttl_seconds: u32,
    pub tenant_claim: String,
}

impl AuthIssuerContract {
    pub fn validate(&self) -> Result<(), SidecarContractError> {
        validate_required("auth.issuer", &self.issuer)?;
        validate_required("auth.signing_key_ref", &self.signing_key_ref)?;
        validate_required("auth.tenant_claim", &self.tenant_claim)?;
        if self.token_ttl_seconds == 0 {
            return Err(SidecarContractError::InvalidTokenTtl);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StorageContract {
    pub bucket: String,
    pub metadata_table: String,
    pub presigned_url_ttl_seconds: u32,
    pub acl_tenant_column: String,
}

impl StorageContract {
    pub fn validate(&self) -> Result<(), SidecarContractError> {
        validate_required("storage.bucket", &self.bucket)?;
        validate_required("storage.metadata_table", &self.metadata_table)?;
        validate_required("storage.acl_tenant_column", &self.acl_tenant_column)?;
        if self.presigned_url_ttl_seconds == 0 {
            return Err(SidecarContractError::InvalidPresignedUrlTtl);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupRestoreContract {
    pub schedule: String,
    pub archive_uri: String,
    pub pitr_target: Option<String>,
    pub queryable_branch_name: Option<String>,
}

impl BackupRestoreContract {
    pub fn validate(&self) -> Result<(), SidecarContractError> {
        validate_required("backup.schedule", &self.schedule)?;
        validate_required("backup.archive_uri", &self.archive_uri)?;
        validate_optional("backup.pitr_target", &self.pitr_target)?;
        validate_optional("backup.queryable_branch_name", &self.queryable_branch_name)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RepackContract {
    pub target: String,
    pub strategy: RepackExecutionStrategy,
    pub max_concurrency: u32,
}

impl RepackContract {
    pub fn validate(&self) -> Result<(), SidecarContractError> {
        validate_required("repack.target", &self.target)?;
        if self.max_concurrency == 0 {
            return Err(SidecarContractError::InvalidConcurrency);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RepackExecutionStrategy {
    PgRepack,
    RepackConcurrentlyPg19,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AnalyticalMirrorContract {
    pub source_slot: String,
    pub mirror_name: String,
    pub storage_uri: String,
    pub search_index_enabled: bool,
}

impl AnalyticalMirrorContract {
    pub fn validate(&self) -> Result<(), SidecarContractError> {
        validate_required("analytical_mirror.source_slot", &self.source_slot)?;
        validate_required("analytical_mirror.mirror_name", &self.mirror_name)?;
        validate_required("analytical_mirror.storage_uri", &self.storage_uri)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SidecarContractError {
    InvalidConcurrency,
    InvalidPresignedUrlTtl,
    InvalidRetryAttempts,
    InvalidTokenTtl,
    InvalidUrl,
    InvalidCdcSinkConfig(&'static str),
    MissingRequiredField(&'static str),
}

impl fmt::Display for SidecarContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConcurrency => {
                write!(formatter, "max_concurrency must be greater than zero")
            }
            Self::InvalidPresignedUrlTtl => {
                write!(
                    formatter,
                    "presigned_url_ttl_seconds must be greater than zero"
                )
            }
            Self::InvalidRetryAttempts => {
                write!(formatter, "max_attempts must be greater than zero")
            }
            Self::InvalidTokenTtl => {
                write!(formatter, "token_ttl_seconds must be greater than zero")
            }
            Self::InvalidUrl => write!(formatter, "url must start with http:// or https://"),
            Self::InvalidCdcSinkConfig(field) => {
                write!(
                    formatter,
                    "{field} contains invalid CDC sink routing syntax"
                )
            }
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for SidecarContractError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), SidecarContractError> {
    if value.trim().is_empty() {
        return Err(SidecarContractError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_nats_subject(field: &'static str, value: &str) -> Result<(), SidecarContractError> {
    validate_required(field, value)?;
    let tokens = value.split('.').collect::<Vec<_>>();
    if tokens.iter().any(|token| token.is_empty()) {
        return Err(SidecarContractError::InvalidCdcSinkConfig(field));
    }
    if value.chars().all(is_nats_subject_char)
        && !tokens.iter().any(|token| *token == "*" || *token == ">")
    {
        Ok(())
    } else {
        Err(SidecarContractError::InvalidCdcSinkConfig(field))
    }
}

fn is_nats_subject_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':')
}

fn validate_pubsub_project_id(
    field: &'static str,
    value: &str,
) -> Result<(), SidecarContractError> {
    validate_required(field, value)?;
    let bytes = value.as_bytes();
    if !(6..=30).contains(&bytes.len()) {
        return Err(SidecarContractError::InvalidCdcSinkConfig(field));
    }
    let last = bytes[bytes.len() - 1];
    if !bytes[0].is_ascii_lowercase() || !(last.is_ascii_lowercase() || last.is_ascii_digit()) {
        return Err(SidecarContractError::InvalidCdcSinkConfig(field));
    }
    if bytes
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
    {
        Ok(())
    } else {
        Err(SidecarContractError::InvalidCdcSinkConfig(field))
    }
}

fn validate_pubsub_topic(field: &'static str, value: &str) -> Result<(), SidecarContractError> {
    validate_required(field, value)?;
    let bytes = value.as_bytes();
    if !(3..=255).contains(&bytes.len()) || value.starts_with("goog") {
        return Err(SidecarContractError::InvalidCdcSinkConfig(field));
    }
    if bytes[0].is_ascii_alphabetic()
        && bytes.iter().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~' | b'+' | b'%')
        })
    {
        Ok(())
    } else {
        Err(SidecarContractError::InvalidCdcSinkConfig(field))
    }
}

fn validate_optional(
    field: &'static str,
    value: &Option<String>,
) -> Result<(), SidecarContractError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(SidecarContractError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional_list(
    field: &'static str,
    values: &[String],
) -> Result<(), SidecarContractError> {
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(SidecarContractError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_sidecar_runtime_contracts_pass() {
        assert_eq!(valid_contracts().validate(), Ok(()));
    }

    #[test]
    fn cdc_rejects_invalid_webhook_url() {
        let mut contracts = valid_contracts();
        contracts.cdc.sinks = vec![CdcSink::Webhook {
            url: "ftp://example.com".to_string(),
        }];

        assert_eq!(contracts.validate(), Err(SidecarContractError::InvalidUrl));
    }

    #[test]
    fn cdc_rejects_protocol_breaking_nats_subjects() {
        for subject in [
            "tenant.orders\r\nPING",
            "tenant orders",
            "tenant.*",
            "tenant.>",
            ".tenant.orders",
            "tenant..orders",
        ] {
            let mut contracts = valid_contracts();
            contracts.cdc.sinks = vec![CdcSink::Nats {
                subject: subject.to_string(),
            }];

            assert_eq!(
                contracts.validate(),
                Err(SidecarContractError::InvalidCdcSinkConfig(
                    "cdc.sinks.nats.subject"
                )),
                "subject {subject:?} should fail closed"
            );
        }
    }

    #[test]
    fn cdc_rejects_invalid_pubsub_routes() {
        let mut contracts = valid_contracts();
        contracts.cdc.sinks = vec![CdcSink::PubSub {
            project_id: "AnalyticsProd".to_string(),
            topic: "orders".to_string(),
        }];
        assert_eq!(
            contracts.validate(),
            Err(SidecarContractError::InvalidCdcSinkConfig(
                "cdc.sinks.pubsub.project_id"
            ))
        );

        let mut contracts = valid_contracts();
        contracts.cdc.sinks = vec![CdcSink::PubSub {
            project_id: "analytics-prod".to_string(),
            topic: "googManaged".to_string(),
        }];
        assert_eq!(
            contracts.validate(),
            Err(SidecarContractError::InvalidCdcSinkConfig(
                "cdc.sinks.pubsub.topic"
            ))
        );
    }

    #[test]
    fn storage_requires_presigned_url_ttl() {
        let mut contracts = valid_contracts();
        contracts.storage.presigned_url_ttl_seconds = 0;

        assert_eq!(
            contracts.validate(),
            Err(SidecarContractError::InvalidPresignedUrlTtl)
        );
    }

    fn valid_contracts() -> SidecarRuntimeContracts {
        SidecarRuntimeContracts {
            cdc: CdcStreamContract {
                slot_name: "ai_blaise_cdc".to_string(),
                publication_name: "ai_blaise_publication".to_string(),
                sinks: vec![
                    CdcSink::Realtime {
                        topic_prefix: "tenant".to_string(),
                    },
                    CdcSink::Webhook {
                        url: "https://example.com/webhooks".to_string(),
                    },
                    CdcSink::AnalyticalMirror {
                        stream_name: "metrics_mirror".to_string(),
                    },
                ],
                retry_policy: DeliveryRetryPolicy {
                    max_attempts: 5,
                    dead_letter_queue: "cdc_dead_letters".to_string(),
                },
            },
            realtime: RealtimeContract {
                topic: "tenant-a:public.orders".to_string(),
                tenant_id: "tenant-a".to_string(),
                filters: vec!["status = 'open'".to_string()],
                presence_enabled: true,
            },
            auth: AuthIssuerContract {
                issuer: "https://auth.example.com".to_string(),
                signing_key_ref: "jwt-signing-key".to_string(),
                token_ttl_seconds: 3_600,
                tenant_claim: "tenant_id".to_string(),
            },
            storage: StorageContract {
                bucket: "tenant-files".to_string(),
                metadata_table: "storage.objects".to_string(),
                presigned_url_ttl_seconds: 900,
                acl_tenant_column: "tenant_id".to_string(),
            },
            backup_restore: BackupRestoreContract {
                schedule: "0 */6 * * *".to_string(),
                archive_uri: "s3://ai-blaise-citus-backups/prod".to_string(),
                pitr_target: Some("2026-05-19T12:00:00Z".to_string()),
                queryable_branch_name: Some("prod-at-noon".to_string()),
            },
            repack: RepackContract {
                target: "public.orders".to_string(),
                strategy: RepackExecutionStrategy::PgRepack,
                max_concurrency: 2,
            },
            analytical_mirror: AnalyticalMirrorContract {
                source_slot: "ai_blaise_cdc".to_string(),
                mirror_name: "metrics_mirror".to_string(),
                storage_uri: "s3://ai-blaise-cold/metrics".to_string(),
                search_index_enabled: true,
            },
        }
    }
}
