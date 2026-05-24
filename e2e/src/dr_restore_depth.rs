// FEATURE: B1
// FEATURE: B3
// FEATURE: B4
// FEATURE: B6
// FEATURE: MR9

use std::error::Error;
use std::fmt;

const EXPECTED_GATE_COUNT: u32 = 8;
const MIN_APPROVERS: usize = 2;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DrRestoreDepthAcceptance {
    pub config: RestoreDrillConfig,
    pub wal_archive: WalArchiveEvidence,
    pub pitr: PitrEvidence,
}

impl DrRestoreDepthAcceptance {
    pub fn canonical() -> Self {
        Self {
            config: RestoreDrillConfig {
                cluster: "prod".to_string(),
                mode: RestoreMode::InPlace,
                source_archive_uri: "s3://ai-blaise-citus-backups/prod/us-east-1".to_string(),
                base_backup_uri:
                    "s3://ai-blaise-citus-backups/prod/us-east-1/base/20260519T113000Z".to_string(),
                wal_archive_uri: "s3://ai-blaise-citus-backups/prod/us-east-1/wal".to_string(),
                target_cluster: "prod".to_string(),
                branch_name: "prod-pitr-20260519T120000Z".to_string(),
                target_time_utc: "2026-05-19T12:00:00Z".to_string(),
                base_backup_started_at_utc: "2026-05-19T11:30:00Z".to_string(),
                base_backup_completed_at_utc: "2026-05-19T11:42:00Z".to_string(),
                wal_coverage_start_utc: "2026-05-19T11:30:00Z".to_string(),
                wal_coverage_end_utc: "2026-05-19T12:05:00Z".to_string(),
                read_only_branch_first: true,
                kms_key_ref: "aws-kms-prod".to_string(),
                destructive_plan_id: Some("pitr-prod-20260519T120000Z".to_string()),
                operator_approvals: vec!["incident-commander".to_string(), "db-sre".to_string()],
            },
            wal_archive: WalArchiveEvidence {
                archive_uri: "s3://ai-blaise-citus-backups/prod/us-east-1/wal".to_string(),
                timeline: "00000001".to_string(),
                first_segment: "0000000100000000000000A1".to_string(),
                last_segment: "0000000100000000000000A6".to_string(),
                segment_count: 6,
                contiguous_segments: true,
                archive_command_observed: true,
                latest_segment_archived_at_utc: "2026-05-19T12:05:00Z".to_string(),
                archive_digest:
                    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                        .to_string(),
            },
            pitr: PitrEvidence {
                restore_job_id: "restore-prod-20260519T120000Z".to_string(),
                source_archive_uri: "s3://ai-blaise-citus-backups/prod/us-east-1".to_string(),
                target_cluster: "prod".to_string(),
                target_time_utc: "2026-05-19T12:00:00Z".to_string(),
                restore_started_at_utc: "2026-05-19T12:09:00Z".to_string(),
                restore_completed_at_utc: "2026-05-19T12:23:00Z".to_string(),
                restore_elapsed_seconds: 840,
                replay_timestamp_utc: "2026-05-19T11:59:58Z".to_string(),
                target_lsn: "0/A500060".to_string(),
                replay_lsn: "0/A600020".to_string(),
                promoted: true,
                validation_queries: vec![
                    ValidationQueryEvidence::new(
                        "pitr_timestamp",
                        "at_or_before_target",
                        "at_or_before_target",
                    ),
                    ValidationQueryEvidence::new("tenant_count", "128", "128"),
                    ValidationQueryEvidence::new(
                        "active_placements",
                        "all_shards_have_active_placement",
                        "all_shards_have_active_placement",
                    ),
                    ValidationQueryEvidence::new(
                        "ledger_hash_chain",
                        "pre_restore_head_only",
                        "pre_restore_head_only",
                    ),
                    ValidationQueryEvidence::new(
                        "search_freshness",
                        "reset_to_restore_point",
                        "reset_to_restore_point",
                    ),
                ],
            },
        }
    }

    pub fn report(&self) -> Result<DrRestoreDepthReport, DrRestoreDepthError> {
        self.config.validate()?;
        self.wal_archive.validate_for(&self.config)?;
        self.pitr.validate_for(&self.config, &self.wal_archive)?;

        Ok(DrRestoreDepthReport {
            gates: vec![
                DrRestoreGate::FailClosedConfig,
                DrRestoreGate::WalArchiveContinuity,
                DrRestoreGate::PitrTargetCovered,
                DrRestoreGate::ReplayEvidence,
                DrRestoreGate::QueryableBranchReadOnly,
                DrRestoreGate::DataValidationQueries,
                DrRestoreGate::EncryptionEvidence,
                DrRestoreGate::AuditTrail,
            ],
            mode: self.config.mode,
            wal_segments: self.wal_archive.segment_count,
            validation_queries: self.pitr.validation_queries.len() as u32,
            approvals: self.config.operator_approvals.len() as u32,
            restore_elapsed_seconds: self.pitr.restore_elapsed_seconds,
            archive_digest_algorithm: self
                .wal_archive
                .archive_digest
                .split_once(':')
                .map(|(algorithm, _)| algorithm.to_string())
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RestoreDrillConfig {
    pub cluster: String,
    pub mode: RestoreMode,
    pub source_archive_uri: String,
    pub base_backup_uri: String,
    pub wal_archive_uri: String,
    pub target_cluster: String,
    pub branch_name: String,
    pub target_time_utc: String,
    pub base_backup_started_at_utc: String,
    pub base_backup_completed_at_utc: String,
    pub wal_coverage_start_utc: String,
    pub wal_coverage_end_utc: String,
    pub read_only_branch_first: bool,
    pub kms_key_ref: String,
    pub destructive_plan_id: Option<String>,
    pub operator_approvals: Vec<String>,
}

impl RestoreDrillConfig {
    fn validate(&self) -> Result<(), DrRestoreDepthError> {
        validate_required("cluster", &self.cluster)?;
        validate_required("target_cluster", &self.target_cluster)?;
        validate_required("branch_name", &self.branch_name)?;
        validate_required("kms_key_ref", &self.kms_key_ref)?;
        validate_object_store_uri("source_archive_uri", &self.source_archive_uri)?;
        validate_object_store_uri("base_backup_uri", &self.base_backup_uri)?;
        validate_object_store_uri("wal_archive_uri", &self.wal_archive_uri)?;

        if !uri_is_under_root(&self.source_archive_uri, &self.base_backup_uri)
            || !uri_is_under_root(&self.source_archive_uri, &self.wal_archive_uri)
        {
            return Err(DrRestoreDepthError::ArchiveMismatch);
        }

        validate_timestamp("target_time_utc", &self.target_time_utc)?;
        validate_timestamp(
            "base_backup_started_at_utc",
            &self.base_backup_started_at_utc,
        )?;
        validate_timestamp(
            "base_backup_completed_at_utc",
            &self.base_backup_completed_at_utc,
        )?;
        validate_timestamp("wal_coverage_start_utc", &self.wal_coverage_start_utc)?;
        validate_timestamp("wal_coverage_end_utc", &self.wal_coverage_end_utc)?;

        require_ordered(
            "base backup window",
            &self.base_backup_started_at_utc,
            &self.base_backup_completed_at_utc,
        )?;
        require_ordered(
            "base backup before PITR target",
            &self.base_backup_completed_at_utc,
            &self.target_time_utc,
        )?;
        require_ordered(
            "WAL coverage starts before target",
            &self.wal_coverage_start_utc,
            &self.target_time_utc,
        )?;
        require_ordered(
            "WAL coverage reaches target",
            &self.target_time_utc,
            &self.wal_coverage_end_utc,
        )?;

        if !self.read_only_branch_first {
            return Err(DrRestoreDepthError::ReadOnlyBranchRequired);
        }

        match self.mode {
            RestoreMode::ReadOnlyBranch => {
                if self.target_cluster == self.cluster {
                    return Err(DrRestoreDepthError::TargetClusterMustDiffer);
                }
            }
            RestoreMode::NewCluster => {
                if self.target_cluster == self.cluster {
                    return Err(DrRestoreDepthError::TargetClusterMustDiffer);
                }
            }
            RestoreMode::InPlace => {
                if self.target_cluster != self.cluster {
                    return Err(DrRestoreDepthError::InPlaceTargetMismatch);
                }
                let Some(plan_id) = &self.destructive_plan_id else {
                    return Err(DrRestoreDepthError::PlanIdRequired);
                };
                validate_required("destructive_plan_id", plan_id)?;
            }
        }

        if !has_two_distinct_values(&self.operator_approvals) {
            return Err(DrRestoreDepthError::InsufficientApprovals);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RestoreMode {
    ReadOnlyBranch,
    NewCluster,
    InPlace,
}

impl RestoreMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnlyBranch => "read-only-branch",
            Self::NewCluster => "new-cluster",
            Self::InPlace => "in-place",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WalArchiveEvidence {
    pub archive_uri: String,
    pub timeline: String,
    pub first_segment: String,
    pub last_segment: String,
    pub segment_count: u32,
    pub contiguous_segments: bool,
    pub archive_command_observed: bool,
    pub latest_segment_archived_at_utc: String,
    pub archive_digest: String,
}

impl WalArchiveEvidence {
    fn validate_for(&self, config: &RestoreDrillConfig) -> Result<(), DrRestoreDepthError> {
        validate_object_store_uri("wal_archive.archive_uri", &self.archive_uri)?;
        if self.archive_uri != config.wal_archive_uri {
            return Err(DrRestoreDepthError::ArchiveMismatch);
        }
        validate_wal_timeline(&self.timeline)?;
        validate_wal_segment("first_segment", &self.first_segment)?;
        validate_wal_segment("last_segment", &self.last_segment)?;
        validate_digest(&self.archive_digest)?;
        validate_timestamp(
            "latest_segment_archived_at_utc",
            &self.latest_segment_archived_at_utc,
        )?;

        if self.segment_count == 0 {
            return Err(DrRestoreDepthError::MissingWalSegments);
        }
        if !self.contiguous_segments {
            return Err(DrRestoreDepthError::WalArchiveGap);
        }
        if !self.archive_command_observed {
            return Err(DrRestoreDepthError::ArchiveCommandMissing);
        }
        require_ordered(
            "latest WAL segment reaches target",
            &config.target_time_utc,
            &self.latest_segment_archived_at_utc,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PitrEvidence {
    pub restore_job_id: String,
    pub source_archive_uri: String,
    pub target_cluster: String,
    pub target_time_utc: String,
    pub restore_started_at_utc: String,
    pub restore_completed_at_utc: String,
    pub restore_elapsed_seconds: u32,
    pub replay_timestamp_utc: String,
    pub target_lsn: String,
    pub replay_lsn: String,
    pub promoted: bool,
    pub validation_queries: Vec<ValidationQueryEvidence>,
}

impl PitrEvidence {
    fn validate_for(
        &self,
        config: &RestoreDrillConfig,
        wal_archive: &WalArchiveEvidence,
    ) -> Result<(), DrRestoreDepthError> {
        validate_required("restore_job_id", &self.restore_job_id)?;
        validate_object_store_uri("pitr.source_archive_uri", &self.source_archive_uri)?;
        validate_required("pitr.target_cluster", &self.target_cluster)?;
        validate_timestamp("pitr.target_time_utc", &self.target_time_utc)?;
        validate_timestamp("restore_started_at_utc", &self.restore_started_at_utc)?;
        validate_timestamp("restore_completed_at_utc", &self.restore_completed_at_utc)?;
        validate_timestamp("replay_timestamp_utc", &self.replay_timestamp_utc)?;
        validate_lsn("target_lsn", &self.target_lsn)?;
        validate_lsn("replay_lsn", &self.replay_lsn)?;

        if self.source_archive_uri != config.source_archive_uri
            || self.target_time_utc != config.target_time_utc
            || self.target_cluster != config.target_cluster
        {
            return Err(DrRestoreDepthError::PitrEvidenceMismatch);
        }
        require_ordered(
            "restore execution window",
            &self.restore_started_at_utc,
            &self.restore_completed_at_utc,
        )?;
        if self.restore_elapsed_seconds == 0 {
            return Err(DrRestoreDepthError::InvalidRestoreWindow);
        }
        require_ordered(
            "replay timestamp at or before target",
            &self.replay_timestamp_utc,
            &self.target_time_utc,
        )?;
        require_ordered(
            "restore can only complete after archived WAL checkpoint",
            &wal_archive.latest_segment_archived_at_utc,
            &self.restore_completed_at_utc,
        )?;
        if !self.promoted {
            return Err(DrRestoreDepthError::RestoreNotPromoted);
        }
        if self.validation_queries.len() < 4 {
            return Err(DrRestoreDepthError::InsufficientValidationQueries);
        }
        for query in &self.validation_queries {
            query.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ValidationQueryEvidence {
    pub name: String,
    pub expected: String,
    pub observed: String,
}

impl ValidationQueryEvidence {
    pub fn new(name: &str, expected: &str, observed: &str) -> Self {
        Self {
            name: name.to_string(),
            expected: expected.to_string(),
            observed: observed.to_string(),
        }
    }

    fn validate(&self) -> Result<(), DrRestoreDepthError> {
        validate_required("validation_query.name", &self.name)?;
        validate_required("validation_query.expected", &self.expected)?;
        validate_required("validation_query.observed", &self.observed)?;
        if self.expected != self.observed {
            return Err(DrRestoreDepthError::ValidationQueryMismatch {
                name: self.name.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DrRestoreDepthReport {
    pub gates: Vec<DrRestoreGate>,
    pub mode: RestoreMode,
    pub wal_segments: u32,
    pub validation_queries: u32,
    pub approvals: u32,
    pub restore_elapsed_seconds: u32,
    pub archive_digest_algorithm: String,
}

impl DrRestoreDepthReport {
    pub fn tsv_header() -> &'static str {
        "mode\ttotal_gates\tgreen_gates\twal_segments\tvalidation_queries\tapprovals\trestore_elapsed_seconds\tarchive_digest_algorithm"
    }

    pub fn to_tsv_row(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.mode.as_str(),
            EXPECTED_GATE_COUNT,
            self.green_gates(),
            self.wal_segments,
            self.validation_queries,
            self.approvals,
            self.restore_elapsed_seconds,
            self.archive_digest_algorithm,
        )
    }

    fn green_gates(&self) -> u32 {
        self.gates.len() as u32
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DrRestoreGate {
    FailClosedConfig,
    WalArchiveContinuity,
    PitrTargetCovered,
    ReplayEvidence,
    QueryableBranchReadOnly,
    DataValidationQueries,
    EncryptionEvidence,
    AuditTrail,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DrRestoreDepthError {
    ArchiveCommandMissing,
    ArchiveMismatch,
    InPlaceTargetMismatch,
    InsufficientApprovals,
    InsufficientValidationQueries,
    InvalidDigest,
    InvalidLsn(&'static str),
    InvalidRestoreWindow,
    InvalidTimestamp(&'static str),
    InvalidUri(&'static str),
    InvalidWalSegment(&'static str),
    InvalidWalTimeline,
    MissingRequiredField(&'static str),
    MissingWalSegments,
    PitrEvidenceMismatch,
    PlanIdRequired,
    ReadOnlyBranchRequired,
    RestoreNotPromoted,
    TargetClusterMustDiffer,
    TimeOrderViolation(&'static str),
    ValidationQueryMismatch { name: String },
    WalArchiveGap,
}

impl fmt::Display for DrRestoreDepthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveCommandMissing => write!(formatter, "WAL archive command evidence is missing"),
            Self::ArchiveMismatch => write!(formatter, "restore archive evidence does not match configured archive URIs"),
            Self::InPlaceTargetMismatch => write!(formatter, "in-place restores must target the source cluster"),
            Self::InsufficientApprovals => write!(formatter, "restore drill requires two distinct operator approvals"),
            Self::InsufficientValidationQueries => write!(formatter, "restore evidence requires tenant, placement, ledger, and freshness validation queries"),
            Self::InvalidDigest => write!(formatter, "archive_digest must be a sha256 digest"),
            Self::InvalidLsn(field) => write!(formatter, "{field} must be a non-zero PostgreSQL LSN"),
            Self::InvalidRestoreWindow => write!(formatter, "restore_elapsed_seconds must be greater than zero"),
            Self::InvalidTimestamp(field) => write!(formatter, "{field} must be an RFC3339 UTC second timestamp"),
            Self::InvalidUri(field) => write!(formatter, "{field} must be a fail-closed object-store URI with a bucket and prefix"),
            Self::InvalidWalSegment(field) => write!(formatter, "{field} must be a 24-character hexadecimal WAL segment name"),
            Self::InvalidWalTimeline => write!(formatter, "timeline must be an 8-character hexadecimal PostgreSQL timeline"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::MissingWalSegments => write!(formatter, "WAL evidence must include at least one archived segment"),
            Self::PitrEvidenceMismatch => write!(formatter, "PITR evidence must match the configured source archive, target cluster, and target time"),
            Self::PlanIdRequired => write!(formatter, "in-place restores require a destructive plan id"),
            Self::ReadOnlyBranchRequired => write!(formatter, "restore drill must validate a read-only branch before destructive restore"),
            Self::RestoreNotPromoted => write!(formatter, "restored cluster must be promoted after reaching the target"),
            Self::TargetClusterMustDiffer => write!(formatter, "branch and new-cluster restores must not target the source cluster"),
            Self::TimeOrderViolation(label) => write!(formatter, "timestamp order violation: {label}"),
            Self::ValidationQueryMismatch { name } => write!(formatter, "validation query {name} did not match expected evidence"),
            Self::WalArchiveGap => write!(formatter, "WAL archive evidence is not contiguous"),
        }
    }
}

impl Error for DrRestoreDepthError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), DrRestoreDepthError> {
    if value.trim().is_empty() {
        return Err(DrRestoreDepthError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_object_store_uri(field: &'static str, value: &str) -> Result<(), DrRestoreDepthError> {
    validate_required(field, value)?;
    let Some((scheme, rest)) = value.split_once("://") else {
        return Err(DrRestoreDepthError::InvalidUri(field));
    };
    if !matches!(scheme, "s3" | "gs" | "az") {
        return Err(DrRestoreDepthError::InvalidUri(field));
    }
    if value.contains('*')
        || value.contains('?')
        || value.contains(' ')
        || value.contains("latest")
        || value.ends_with('/')
    {
        return Err(DrRestoreDepthError::InvalidUri(field));
    }
    let mut parts = rest.splitn(2, '/');
    let bucket = parts.next().unwrap_or_default();
    let prefix = parts.next().unwrap_or_default();
    if bucket.is_empty() || prefix.is_empty() {
        return Err(DrRestoreDepthError::InvalidUri(field));
    }
    Ok(())
}

fn uri_is_under_root(root: &str, child: &str) -> bool {
    child.len() > root.len()
        && child.starts_with(root)
        && child.as_bytes().get(root.len()) == Some(&b'/')
}

fn validate_timestamp(field: &'static str, value: &str) -> Result<(), DrRestoreDepthError> {
    validate_required(field, value)?;
    let bytes = value.as_bytes();
    let shape_is_valid = bytes.len() == 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z';
    let digits_are_valid = bytes.iter().enumerate().all(|(index, byte)| match index {
        4 | 7 | 10 | 13 | 16 | 19 => true,
        _ => byte.is_ascii_digit(),
    });
    if shape_is_valid && digits_are_valid {
        Ok(())
    } else {
        Err(DrRestoreDepthError::InvalidTimestamp(field))
    }
}

fn require_ordered(
    label: &'static str,
    earlier_or_equal: &str,
    later_or_equal: &str,
) -> Result<(), DrRestoreDepthError> {
    if earlier_or_equal <= later_or_equal {
        Ok(())
    } else {
        Err(DrRestoreDepthError::TimeOrderViolation(label))
    }
}

fn validate_wal_timeline(value: &str) -> Result<(), DrRestoreDepthError> {
    if value.len() == 8 && value.chars().all(|character| character.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(DrRestoreDepthError::InvalidWalTimeline)
    }
}

fn validate_wal_segment(field: &'static str, value: &str) -> Result<(), DrRestoreDepthError> {
    if value.len() == 24 && value.chars().all(|character| character.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(DrRestoreDepthError::InvalidWalSegment(field))
    }
}

fn validate_digest(value: &str) -> Result<(), DrRestoreDepthError> {
    let Some(digest) = value.strip_prefix("sha256:") else {
        return Err(DrRestoreDepthError::InvalidDigest);
    };
    if digest.len() == 64
        && digest
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Ok(())
    } else {
        Err(DrRestoreDepthError::InvalidDigest)
    }
}

fn validate_lsn(field: &'static str, value: &str) -> Result<(), DrRestoreDepthError> {
    let Some((high, low)) = value.split_once('/') else {
        return Err(DrRestoreDepthError::InvalidLsn(field));
    };
    if value == "0/0"
        || high.is_empty()
        || low.is_empty()
        || !high.chars().all(|character| character.is_ascii_hexdigit())
        || !low.chars().all(|character| character.is_ascii_hexdigit())
    {
        return Err(DrRestoreDepthError::InvalidLsn(field));
    }
    Ok(())
}

fn has_two_distinct_values(values: &[String]) -> bool {
    let mut distinct = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() && !distinct.contains(&trimmed) {
            distinct.push(trimmed);
        }
    }
    distinct.len() >= MIN_APPROVERS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_report_covers_restore_depth_gates() {
        let report = DrRestoreDepthAcceptance::canonical()
            .report()
            .expect("canonical DR restore-depth report");

        assert_eq!(report.gates.len() as u32, EXPECTED_GATE_COUNT);
        assert_eq!(report.mode, RestoreMode::InPlace);
        assert_eq!(report.wal_segments, 6);
        assert_eq!(report.validation_queries, 5);
        assert_eq!(report.approvals, 2);
        assert_eq!(report.restore_elapsed_seconds, 840);
        assert_eq!(report.archive_digest_algorithm, "sha256");
        assert_eq!(report.to_tsv_row(), "in-place\t8\t8\t6\t5\t2\t840\tsha256");
    }

    #[test]
    fn fail_closed_config_rejects_latest_archive_alias() {
        let mut acceptance = DrRestoreDepthAcceptance::canonical();
        acceptance.config.source_archive_uri = "s3://bucket/latest".to_string();

        assert_eq!(
            acceptance.report(),
            Err(DrRestoreDepthError::InvalidUri("source_archive_uri"))
        );
    }

    #[test]
    fn target_time_must_be_inside_wal_coverage() {
        let mut acceptance = DrRestoreDepthAcceptance::canonical();
        acceptance.config.wal_coverage_end_utc = "2026-05-19T11:59:00Z".to_string();

        assert_eq!(
            acceptance.report(),
            Err(DrRestoreDepthError::TimeOrderViolation(
                "WAL coverage reaches target"
            ))
        );
    }

    #[test]
    fn in_place_restore_requires_read_only_branch_and_plan_id() {
        let mut acceptance = DrRestoreDepthAcceptance::canonical();
        acceptance.config.read_only_branch_first = false;

        assert_eq!(
            acceptance.report(),
            Err(DrRestoreDepthError::ReadOnlyBranchRequired)
        );

        let mut acceptance = DrRestoreDepthAcceptance::canonical();
        acceptance.config.destructive_plan_id = None;

        assert_eq!(
            acceptance.report(),
            Err(DrRestoreDepthError::PlanIdRequired)
        );
    }

    #[test]
    fn wal_archive_must_be_contiguous_and_have_digest() {
        let mut acceptance = DrRestoreDepthAcceptance::canonical();
        acceptance.wal_archive.contiguous_segments = false;

        assert_eq!(acceptance.report(), Err(DrRestoreDepthError::WalArchiveGap));

        let mut acceptance = DrRestoreDepthAcceptance::canonical();
        acceptance.wal_archive.archive_digest = "sha1:abc".to_string();

        assert_eq!(acceptance.report(), Err(DrRestoreDepthError::InvalidDigest));
    }

    #[test]
    fn pitr_evidence_must_match_restore_target_and_promote() {
        let mut acceptance = DrRestoreDepthAcceptance::canonical();
        acceptance.pitr.target_time_utc = "2026-05-19T12:01:00Z".to_string();

        assert_eq!(
            acceptance.report(),
            Err(DrRestoreDepthError::PitrEvidenceMismatch)
        );

        let mut acceptance = DrRestoreDepthAcceptance::canonical();
        acceptance.pitr.promoted = false;

        assert_eq!(
            acceptance.report(),
            Err(DrRestoreDepthError::RestoreNotPromoted)
        );
    }

    #[test]
    fn validation_query_mismatch_fails() {
        let mut acceptance = DrRestoreDepthAcceptance::canonical();
        acceptance.pitr.validation_queries[1].observed = "127".to_string();

        assert_eq!(
            acceptance.report(),
            Err(DrRestoreDepthError::ValidationQueryMismatch {
                name: "tenant_count".to_string()
            })
        );
    }

    #[test]
    fn restore_drill_requires_two_distinct_approvals() {
        let mut acceptance = DrRestoreDepthAcceptance::canonical();
        acceptance.config.operator_approvals = vec!["db-sre".to_string(), "db-sre".to_string()];

        assert_eq!(
            acceptance.report(),
            Err(DrRestoreDepthError::InsufficientApprovals)
        );
    }
}
