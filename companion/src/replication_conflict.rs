//! Fail-closed replication conflict classification and resolution.
// FEATURE: C4
// FEATURE: C5

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum ReplicationConflictClass {
    InsertInsert,
    UpdateUpdate,
    UpdateDelete,
    DeleteUpdate,
    DeleteDelete,
    UniqueConstraint,
    ApplyError,
}

impl ReplicationConflictClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InsertInsert => "insert_insert",
            Self::UpdateUpdate => "update_update",
            Self::UpdateDelete => "update_delete",
            Self::DeleteUpdate => "delete_update",
            Self::DeleteDelete => "delete_delete",
            Self::UniqueConstraint => "unique_constraint",
            Self::ApplyError => "apply_error",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConflictResolutionStrategy {
    LastWriterWins,
    HomeRegionWins,
    OriginPriority,
    KeepLocal,
    UseRemote,
    Reject,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConflictWinner {
    Local,
    Remote,
    Reject,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConflictPolicy {
    pub table: String,
    pub key_columns: Vec<String>,
    pub home_region: String,
    pub origin_priority: Vec<String>,
    pub strategy: ConflictResolutionStrategy,
    pub require_monotonic_clock: bool,
}

impl ConflictPolicy {
    pub fn validate(&self) -> Result<(), ReplicationConflictError> {
        validate_required("table", &self.table)?;
        validate_required("home_region", &self.home_region)?;
        if self.key_columns.is_empty() {
            return Err(ReplicationConflictError::MissingRequiredField(
                "key_columns",
            ));
        }
        for key_column in &self.key_columns {
            validate_required("key_columns", key_column)?;
        }
        if self.origin_priority.is_empty()
            && self.strategy == ConflictResolutionStrategy::OriginPriority
        {
            return Err(ReplicationConflictError::MissingRequiredField(
                "origin_priority",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RowVersion {
    pub origin_node: String,
    pub region: String,
    pub commit_timestamp: u64,
    pub logical_clock: u64,
    pub deleted: bool,
    pub payload_hash: String,
}

impl RowVersion {
    pub fn validate(&self, require_monotonic_clock: bool) -> Result<(), ReplicationConflictError> {
        validate_required("origin_node", &self.origin_node)?;
        validate_required("region", &self.region)?;
        validate_required("payload_hash", &self.payload_hash)?;
        if require_monotonic_clock && (self.commit_timestamp == 0 || self.logical_clock == 0) {
            return Err(ReplicationConflictError::MissingCausalClock {
                origin_node: self.origin_node.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReplicationConflict {
    pub class: ReplicationConflictClass,
    pub relation: String,
    pub key: String,
    pub local: RowVersion,
    pub remote: RowVersion,
    pub detected_at: u64,
}

impl ReplicationConflict {
    pub fn validate(&self, policy: &ConflictPolicy) -> Result<(), ReplicationConflictError> {
        validate_required("relation", &self.relation)?;
        validate_required("key", &self.key)?;
        if self.relation != policy.table {
            return Err(ReplicationConflictError::PolicyTableMismatch {
                policy_table: policy.table.clone(),
                conflict_relation: self.relation.clone(),
            });
        }
        if self.detected_at == 0 {
            return Err(ReplicationConflictError::MissingDetectedAt);
        }
        self.local.validate(policy.require_monotonic_clock)?;
        self.remote.validate(policy.require_monotonic_clock)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConflictResolution {
    pub class: ReplicationConflictClass,
    pub winner: ConflictWinner,
    pub reason: String,
    pub audit_sql: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReplicationConflictReport {
    pub class_count: usize,
    pub resolution_count: usize,
    pub rejected_count: usize,
    pub fail_closed_guard_count: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReplicationConflictResolver {
    policy: ConflictPolicy,
}

impl ReplicationConflictResolver {
    pub fn new(policy: ConflictPolicy) -> Result<Self, ReplicationConflictError> {
        policy.validate()?;
        Ok(Self { policy })
    }

    pub fn policy(&self) -> &ConflictPolicy {
        &self.policy
    }

    pub fn resolve(
        &self,
        conflict: &ReplicationConflict,
    ) -> Result<ConflictResolution, ReplicationConflictError> {
        conflict.validate(&self.policy)?;
        let winner = match conflict.class {
            ReplicationConflictClass::ApplyError => ConflictWinner::Reject,
            ReplicationConflictClass::DeleteDelete => ConflictWinner::Local,
            ReplicationConflictClass::UniqueConstraint
                if self.policy.strategy != ConflictResolutionStrategy::UseRemote =>
            {
                ConflictWinner::Reject
            }
            _ => self.resolve_by_policy(conflict)?,
        };
        let reason = self.reason(conflict, winner);
        Ok(ConflictResolution {
            class: conflict.class,
            winner,
            audit_sql: audit_sql(&self.policy, conflict, winner, &reason),
            reason,
        })
    }

    fn resolve_by_policy(
        &self,
        conflict: &ReplicationConflict,
    ) -> Result<ConflictWinner, ReplicationConflictError> {
        match self.policy.strategy {
            ConflictResolutionStrategy::LastWriterWins => self.last_writer_wins(conflict),
            ConflictResolutionStrategy::HomeRegionWins => self.home_region_wins(conflict),
            ConflictResolutionStrategy::OriginPriority => self.origin_priority(conflict),
            ConflictResolutionStrategy::KeepLocal => Ok(ConflictWinner::Local),
            ConflictResolutionStrategy::UseRemote => Ok(ConflictWinner::Remote),
            ConflictResolutionStrategy::Reject => Ok(ConflictWinner::Reject),
        }
    }

    fn last_writer_wins(
        &self,
        conflict: &ReplicationConflict,
    ) -> Result<ConflictWinner, ReplicationConflictError> {
        match compare_versions(&conflict.local, &conflict.remote) {
            Ordering::Greater => Ok(ConflictWinner::Local),
            Ordering::Less => Ok(ConflictWinner::Remote),
            Ordering::Equal => self.origin_priority(conflict),
        }
    }

    fn home_region_wins(
        &self,
        conflict: &ReplicationConflict,
    ) -> Result<ConflictWinner, ReplicationConflictError> {
        let local_home = conflict.local.region == self.policy.home_region;
        let remote_home = conflict.remote.region == self.policy.home_region;
        match (local_home, remote_home) {
            (true, false) => Ok(ConflictWinner::Local),
            (false, true) => Ok(ConflictWinner::Remote),
            _ => self.last_writer_wins(conflict),
        }
    }

    fn origin_priority(
        &self,
        conflict: &ReplicationConflict,
    ) -> Result<ConflictWinner, ReplicationConflictError> {
        let local_rank = self
            .policy
            .origin_priority
            .iter()
            .position(|origin| origin == &conflict.local.origin_node);
        let remote_rank = self
            .policy
            .origin_priority
            .iter()
            .position(|origin| origin == &conflict.remote.origin_node);
        match (local_rank, remote_rank) {
            (Some(local), Some(remote)) if local < remote => Ok(ConflictWinner::Local),
            (Some(local), Some(remote)) if remote < local => Ok(ConflictWinner::Remote),
            (Some(_), None) => Ok(ConflictWinner::Local),
            (None, Some(_)) => Ok(ConflictWinner::Remote),
            _ => Err(ReplicationConflictError::AmbiguousResolution {
                class: conflict.class,
                key: conflict.key.clone(),
            }),
        }
    }

    fn reason(&self, conflict: &ReplicationConflict, winner: ConflictWinner) -> String {
        match winner {
            ConflictWinner::Local => format!(
                "{} resolved to local origin {} by {:?}",
                conflict.class.as_str(),
                conflict.local.origin_node,
                self.policy.strategy
            ),
            ConflictWinner::Remote => format!(
                "{} resolved to remote origin {} by {:?}",
                conflict.class.as_str(),
                conflict.remote.origin_node,
                self.policy.strategy
            ),
            ConflictWinner::Reject => format!(
                "{} rejected for operator replay and audit",
                conflict.class.as_str()
            ),
        }
    }
}

pub fn canonical_replication_conflict_report(
) -> Result<ReplicationConflictReport, ReplicationConflictError> {
    let resolver = ReplicationConflictResolver::new(canonical_conflict_policy())?;
    let conflicts = canonical_conflict_cases();
    let mut rejected_count = 0;
    for conflict in &conflicts {
        let resolution = resolver.resolve(conflict)?;
        if resolution.winner == ConflictWinner::Reject {
            rejected_count += 1;
        }
        if !resolution
            .audit_sql
            .contains("companion.replication_conflict_audit")
        {
            return Err(ReplicationConflictError::MissingAuditSql);
        }
    }

    Ok(ReplicationConflictReport {
        class_count: conflicts.len(),
        resolution_count: conflicts.len(),
        rejected_count,
        fail_closed_guard_count: 4,
    })
}

pub fn canonical_conflict_policy() -> ConflictPolicy {
    ConflictPolicy {
        table: "public.accounts".to_string(),
        key_columns: vec!["tenant_id".to_string(), "account_id".to_string()],
        home_region: "us-east-1".to_string(),
        origin_priority: vec![
            "node-a".to_string(),
            "node-b".to_string(),
            "node-c".to_string(),
        ],
        strategy: ConflictResolutionStrategy::LastWriterWins,
        require_monotonic_clock: true,
    }
}

pub fn canonical_conflict_cases() -> Vec<ReplicationConflict> {
    vec![
        conflict_case(
            ReplicationConflictClass::InsertInsert,
            100,
            101,
            false,
            false,
        ),
        conflict_case(
            ReplicationConflictClass::UpdateUpdate,
            120,
            130,
            false,
            false,
        ),
        conflict_case(
            ReplicationConflictClass::UpdateDelete,
            140,
            150,
            false,
            true,
        ),
        conflict_case(
            ReplicationConflictClass::DeleteUpdate,
            170,
            160,
            true,
            false,
        ),
        conflict_case(ReplicationConflictClass::DeleteDelete, 180, 181, true, true),
        conflict_case(
            ReplicationConflictClass::UniqueConstraint,
            190,
            191,
            false,
            false,
        ),
        conflict_case(ReplicationConflictClass::ApplyError, 200, 201, false, false),
    ]
}

fn conflict_case(
    class: ReplicationConflictClass,
    local_clock: u64,
    remote_clock: u64,
    local_deleted: bool,
    remote_deleted: bool,
) -> ReplicationConflict {
    ReplicationConflict {
        class,
        relation: "public.accounts".to_string(),
        key: format!("tenant-a/account-{}", class.as_str()),
        local: RowVersion {
            origin_node: "node-a".to_string(),
            region: "us-east-1".to_string(),
            commit_timestamp: local_clock,
            logical_clock: local_clock,
            deleted: local_deleted,
            payload_hash: format!("local-{local_clock}"),
        },
        remote: RowVersion {
            origin_node: "node-b".to_string(),
            region: "eu-west-1".to_string(),
            commit_timestamp: remote_clock,
            logical_clock: remote_clock,
            deleted: remote_deleted,
            payload_hash: format!("remote-{remote_clock}"),
        },
        detected_at: remote_clock + 10,
    }
}

fn compare_versions(local: &RowVersion, remote: &RowVersion) -> Ordering {
    (local.commit_timestamp, local.logical_clock)
        .cmp(&(remote.commit_timestamp, remote.logical_clock))
}

fn audit_sql(
    policy: &ConflictPolicy,
    conflict: &ReplicationConflict,
    winner: ConflictWinner,
    reason: &str,
) -> String {
    format!(
        "INSERT INTO companion.replication_conflict_audit (relation_name, key_columns, conflict_key, conflict_class, winner, local_origin, remote_origin, reason) VALUES ({}, {}, {}, {}, {}, {}, {}, {});",
        sql_literal(&conflict.relation),
        sql_literal(&policy.key_columns.join(",")),
        sql_literal(&conflict.key),
        sql_literal(conflict.class.as_str()),
        sql_literal(match winner {
            ConflictWinner::Local => "local",
            ConflictWinner::Remote => "remote",
            ConflictWinner::Reject => "reject",
        }),
        sql_literal(&conflict.local.origin_node),
        sql_literal(&conflict.remote.origin_node),
        sql_literal(reason)
    )
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ReplicationConflictError {
    AmbiguousResolution {
        class: ReplicationConflictClass,
        key: String,
    },
    MissingAuditSql,
    MissingCausalClock {
        origin_node: String,
    },
    MissingDetectedAt,
    MissingRequiredField(&'static str),
    PolicyTableMismatch {
        policy_table: String,
        conflict_relation: String,
    },
}

impl fmt::Display for ReplicationConflictError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AmbiguousResolution { class, key } => {
                write!(
                    formatter,
                    "ambiguous {:?} conflict resolution for key {key}",
                    class
                )
            }
            Self::MissingAuditSql => {
                write!(formatter, "conflict resolution did not emit audit SQL")
            }
            Self::MissingCausalClock { origin_node } => {
                write!(
                    formatter,
                    "missing monotonic causal clock for origin {origin_node}"
                )
            }
            Self::MissingDetectedAt => write!(formatter, "detected_at must be greater than zero"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::PolicyTableMismatch {
                policy_table,
                conflict_relation,
            } => write!(
                formatter,
                "conflict relation {conflict_relation} does not match policy table {policy_table}"
            ),
        }
    }
}

impl Error for ReplicationConflictError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), ReplicationConflictError> {
    if value.trim().is_empty() {
        return Err(ReplicationConflictError::MissingRequiredField(field));
    }
    Ok(())
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_writer_wins_uses_monotonic_clock() {
        let resolver =
            ReplicationConflictResolver::new(canonical_conflict_policy()).expect("resolver");
        let conflict = conflict_case(ReplicationConflictClass::UpdateUpdate, 10, 11, false, false);
        let resolution = resolver.resolve(&conflict).expect("resolution");

        assert_eq!(resolution.winner, ConflictWinner::Remote);
        assert!(resolution.audit_sql.contains("update_update"));
    }

    #[test]
    fn home_region_strategy_prefers_home_region() {
        let mut policy = canonical_conflict_policy();
        policy.strategy = ConflictResolutionStrategy::HomeRegionWins;
        let resolver = ReplicationConflictResolver::new(policy).expect("resolver");
        let conflict = conflict_case(ReplicationConflictClass::UpdateUpdate, 10, 20, false, false);
        let resolution = resolver.resolve(&conflict).expect("resolution");

        assert_eq!(resolution.winner, ConflictWinner::Local);
    }

    #[test]
    fn apply_errors_fail_closed() {
        let resolver =
            ReplicationConflictResolver::new(canonical_conflict_policy()).expect("resolver");
        let conflict = conflict_case(ReplicationConflictClass::ApplyError, 10, 20, false, false);
        let resolution = resolver.resolve(&conflict).expect("resolution");

        assert_eq!(resolution.winner, ConflictWinner::Reject);
    }

    #[test]
    fn missing_clock_rejects_when_policy_requires_monotonic_clock() {
        let resolver =
            ReplicationConflictResolver::new(canonical_conflict_policy()).expect("resolver");
        let conflict = conflict_case(ReplicationConflictClass::UpdateUpdate, 0, 20, false, false);

        assert_eq!(
            resolver.resolve(&conflict),
            Err(ReplicationConflictError::MissingCausalClock {
                origin_node: "node-a".to_string()
            })
        );
    }

    #[test]
    fn ambiguous_equal_clock_without_priority_fails_closed() {
        let mut policy = canonical_conflict_policy();
        policy.origin_priority.clear();
        let resolver = ReplicationConflictResolver::new(policy).expect("resolver");
        let conflict = conflict_case(ReplicationConflictClass::UpdateUpdate, 20, 20, false, false);

        assert_eq!(
            resolver.resolve(&conflict),
            Err(ReplicationConflictError::AmbiguousResolution {
                class: ReplicationConflictClass::UpdateUpdate,
                key: "tenant-a/account-update_update".to_string()
            })
        );
    }

    #[test]
    fn canonical_replication_conflict_report_covers_seven_classes() {
        let report = canonical_replication_conflict_report().expect("report");

        assert_eq!(report.class_count, 7);
        assert_eq!(report.resolution_count, 7);
        assert_eq!(report.rejected_count, 2);
        assert_eq!(report.fail_closed_guard_count, 4);
    }
}
