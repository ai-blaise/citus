// FEATURE: M1
// FEATURE: M11

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationPlan {
    pub name: String,
    pub table: String,
    pub operations: Vec<MigrationOperation>,
    pub data_invariants: Vec<MigrationDataInvariant>,
    pub lock_timeout_ms: u32,
    pub backfill_batch_size: u32,
}

impl MigrationPlan {
    pub fn validate(&self) -> Result<(), MigrationError> {
        validate_required("name", &self.name)?;
        validate_required("table", &self.table)?;
        if self.operations.is_empty() {
            return Err(MigrationError::MissingRequiredField("operations"));
        }
        if self
            .operations
            .iter()
            .any(MigrationOperation::requires_data_invariant)
            && self.data_invariants.is_empty()
        {
            return Err(MigrationError::MissingDataInvariant);
        }
        for invariant in &self.data_invariants {
            invariant.validate()?;
        }
        for operation in &self.operations {
            operation.validate()?;
        }
        if self.lock_timeout_ms == 0 {
            return Err(MigrationError::InvalidLockTimeout);
        }
        if self.backfill_batch_size == 0 {
            return Err(MigrationError::InvalidBackfillBatch);
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<MigrationSqlPlan, MigrationError> {
        self.validate()?;
        // Phase 1 of the gh-ost cut-over: open the migration ledger row,
        // create the shadow table on every shard, and emit the requested
        // schema operations against the shadow copy.
        let mut commands = vec![format!(
            "SELECT citus_admin.migrate_start({name}, {table}, {lock_timeout}, {batch_size});",
            name = sql_literal(&self.name),
            table = sql_literal(&self.table),
            lock_timeout = self.lock_timeout_ms,
            batch_size = self.backfill_batch_size
        )];
        commands.extend(
            self.data_invariants
                .iter()
                .map(|invariant| invariant.to_sql(&self.name)),
        );
        commands.push(format!(
            "SELECT citus_admin.shadow_table_create({name}, {table});",
            name = sql_literal(&self.name),
            table = sql_literal(&self.table)
        ));
        commands.extend(self.operations.iter().map(MigrationOperation::to_sql));
        commands.push(format!(
            "SELECT citus_admin.install_write_triggers({name}, {table});",
            name = sql_literal(&self.name),
            table = sql_literal(&self.table)
        ));
        commands.push(format!(
            "SELECT citus_admin.backfill_run({name}, {table}, {batch_size});",
            name = sql_literal(&self.name),
            table = sql_literal(&self.table),
            batch_size = self.backfill_batch_size
        ));
        commands.push(format!(
            "SELECT citus_admin.row_diff_verify({name}, {table});",
            name = sql_literal(&self.name),
            table = sql_literal(&self.table)
        ));
        if !self.data_invariants.is_empty() {
            commands.push(format!(
                "SELECT companion_internal.migration_assert_invariants({});",
                sql_literal(&self.name)
            ));
        }
        commands.push(format!(
            "SELECT citus_admin.shadow_table_publish({name}, {table});",
            name = sql_literal(&self.name),
            table = sql_literal(&self.table)
        ));
        commands.push(format!(
            "SELECT citus_admin.migrate_complete({});",
            sql_literal(&self.name)
        ));
        MigrationSqlPlan::new("M1", commands)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationDataInvariant {
    pub check_name: String,
    pub check_sql: String,
}

impl MigrationDataInvariant {
    pub fn validate(&self) -> Result<(), MigrationError> {
        validate_required("data_invariant.check_name", &self.check_name)?;
        validate_required("data_invariant.check_sql", &self.check_sql)?;
        if !is_select_only_sql(&self.check_sql) {
            return Err(MigrationError::InvalidInvariantSql);
        }
        Ok(())
    }

    fn to_sql(&self, migration_name: &str) -> String {
        format!(
            "SELECT companion_internal.migration_register_invariant({}, {}, {});",
            sql_literal(migration_name),
            sql_literal(&self.check_name),
            sql_literal(normalize_invariant_sql(&self.check_sql))
        )
    }
}

pub fn assert_migration_data_invariants_sql(
    migration_name: &str,
) -> Result<String, MigrationError> {
    validate_required("migration_name", migration_name)?;
    Ok(format!(
        "SELECT companion_internal.migration_assert_invariants({});",
        sql_literal(migration_name)
    ))
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MigrationOperation {
    AddColumn {
        column: String,
        sql_type: String,
        default_expression: Option<String>,
    },
    DropColumn {
        column: String,
    },
    RenameColumn {
        old_column: String,
        new_column: String,
    },
    AlterColumnType {
        column: String,
        from_type: String,
        to_type: String,
        cast_expression: String,
    },
}

impl MigrationOperation {
    pub fn requires_data_invariant(&self) -> bool {
        matches!(
            self,
            Self::DropColumn { .. } | Self::RenameColumn { .. } | Self::AlterColumnType { .. }
        )
    }

    fn validate(&self) -> Result<(), MigrationError> {
        match self {
            Self::AddColumn {
                column,
                sql_type,
                default_expression,
            } => {
                validate_required("operation.column", column)?;
                validate_required("operation.sql_type", sql_type)?;
                validate_optional("operation.default_expression", default_expression)
            }
            Self::DropColumn { column } => validate_required("operation.column", column),
            Self::RenameColumn {
                old_column,
                new_column,
            } => {
                validate_required("operation.old_column", old_column)?;
                validate_required("operation.new_column", new_column)
            }
            Self::AlterColumnType {
                column,
                from_type,
                to_type,
                cast_expression,
            } => {
                validate_required("operation.column", column)?;
                validate_required("operation.from_type", from_type)?;
                validate_required("operation.to_type", to_type)?;
                if from_type.trim() == to_type.trim() {
                    return Err(MigrationError::NoTypeChange);
                }
                validate_required("operation.cast_expression", cast_expression)
            }
        }
    }

    fn to_sql(&self) -> String {
        match self {
            Self::AddColumn {
                column,
                sql_type,
                default_expression,
            } => format!(
                "SELECT companion_internal.migration_add_column({}, {}, {});",
                sql_literal(column),
                sql_literal(sql_type),
                optional_sql_literal(default_expression)
            ),
            Self::DropColumn { column } => format!(
                "SELECT companion_internal.migration_drop_column({});",
                sql_literal(column)
            ),
            Self::RenameColumn {
                old_column,
                new_column,
            } => format!(
                "SELECT companion_internal.migration_rename_column({}, {});",
                sql_literal(old_column),
                sql_literal(new_column)
            ),
            Self::AlterColumnType {
                column,
                from_type,
                to_type,
                cast_expression,
            } => format!(
                "SELECT companion_internal.migration_online_type_change({}, {}, {}, {});",
                sql_literal(column),
                sql_literal(from_type),
                sql_literal(to_type),
                sql_literal(cast_expression)
            ),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl MigrationSqlPlan {
    fn new(feature_id: &'static str, commands: Vec<String>) -> Result<Self, MigrationError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(MigrationError::MissingRequiredField("commands"));
        }
        Ok(Self {
            feature_id,
            commands,
        })
    }

    pub fn script(&self) -> String {
        self.commands.join("\n")
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MigrationRuntimePhase {
    Expand,
    Backfill,
    Validate,
    CutoverReady,
    Publish,
    Complete,
    Failed,
    RolledBack,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MigrationRuntimeObservation {
    ExpandApplied,
    BackfillProgress {
        copied_rows: u64,
        remaining_rows: u64,
    },
    RowDiffVerified {
        diff_rows: u64,
    },
    CutoverApproved,
    PublishApplied,
    RollbackRequested,
    FailureDetected {
        reason: String,
    },
}

impl MigrationRuntimeObservation {
    fn label(&self) -> &'static str {
        match self {
            Self::ExpandApplied => "expand_applied",
            Self::BackfillProgress { .. } => "backfill_progress",
            Self::RowDiffVerified { .. } => "row_diff_verified",
            Self::CutoverApproved => "cutover_approved",
            Self::PublishApplied => "publish_applied",
            Self::RollbackRequested => "rollback_requested",
            Self::FailureDetected { .. } => "failure_detected",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MigrationRuntimeAction {
    ApplySql,
    Wait,
    ReadyForCutover,
    Complete,
    Halt,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationRuntimeDecision {
    pub phase: MigrationRuntimePhase,
    pub action: MigrationRuntimeAction,
    pub commands: Vec<String>,
    pub guardrail: String,
}

impl MigrationRuntimeDecision {
    fn new(
        phase: MigrationRuntimePhase,
        action: MigrationRuntimeAction,
        commands: Vec<String>,
        guardrail: impl Into<String>,
    ) -> Self {
        Self {
            phase,
            action,
            commands,
            guardrail: guardrail.into(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationRuntimeReport {
    pub phase_count: usize,
    pub sql_batch_count: usize,
    pub command_count: usize,
    pub safety_guard_count: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationRuntime {
    plan: MigrationPlan,
    phase: MigrationRuntimePhase,
    copied_rows: u64,
    completed_sql_batches: usize,
}

impl MigrationRuntime {
    pub fn new(plan: MigrationPlan) -> Result<Self, MigrationError> {
        plan.validate()?;
        Ok(Self {
            plan,
            phase: MigrationRuntimePhase::Expand,
            copied_rows: 0,
            completed_sql_batches: 0,
        })
    }

    pub fn phase(&self) -> MigrationRuntimePhase {
        self.phase
    }

    pub fn current_decision(&self) -> MigrationRuntimeDecision {
        match self.phase {
            MigrationRuntimePhase::Expand => MigrationRuntimeDecision::new(
                self.phase,
                MigrationRuntimeAction::ApplySql,
                self.expand_commands(),
                format!(
                    "bounded_lock_ms={} backfill_batch_size={} requires_shadow_table_and_triggers",
                    self.plan.lock_timeout_ms, self.plan.backfill_batch_size
                ),
            ),
            MigrationRuntimePhase::Backfill => MigrationRuntimeDecision::new(
                self.phase,
                MigrationRuntimeAction::ApplySql,
                self.backfill_commands(),
                "backfill must report remaining_rows=0 before validation",
            ),
            MigrationRuntimePhase::Validate => MigrationRuntimeDecision::new(
                self.phase,
                MigrationRuntimeAction::ApplySql,
                self.validate_commands(),
                "row_diff_verify must report zero drift before cutover",
            ),
            MigrationRuntimePhase::CutoverReady => MigrationRuntimeDecision::new(
                self.phase,
                MigrationRuntimeAction::ReadyForCutover,
                Vec::new(),
                "manual cutover approval required after zero-diff validation",
            ),
            MigrationRuntimePhase::Publish => MigrationRuntimeDecision::new(
                self.phase,
                MigrationRuntimeAction::ApplySql,
                self.publish_commands(),
                "publish swaps the shadow table and closes the migration ledger",
            ),
            MigrationRuntimePhase::Complete => MigrationRuntimeDecision::new(
                self.phase,
                MigrationRuntimeAction::Complete,
                Vec::new(),
                "migration complete",
            ),
            MigrationRuntimePhase::Failed => MigrationRuntimeDecision::new(
                self.phase,
                MigrationRuntimeAction::Halt,
                Vec::new(),
                "migration halted after failure",
            ),
            MigrationRuntimePhase::RolledBack => MigrationRuntimeDecision::new(
                self.phase,
                MigrationRuntimeAction::Halt,
                Vec::new(),
                "migration rolled back",
            ),
        }
    }

    pub fn observe(
        &mut self,
        observation: MigrationRuntimeObservation,
    ) -> Result<MigrationRuntimeDecision, MigrationError> {
        if matches!(observation, MigrationRuntimeObservation::RollbackRequested) {
            self.phase = MigrationRuntimePhase::RolledBack;
            return Ok(MigrationRuntimeDecision::new(
                self.phase,
                MigrationRuntimeAction::Halt,
                vec![format!(
                    "SELECT citus_admin.shadow_table_abort({});",
                    sql_literal(&self.plan.name)
                )],
                "rollback aborts shadow table writes before publish",
            ));
        }

        if let MigrationRuntimeObservation::FailureDetected { reason } = &observation {
            self.phase = MigrationRuntimePhase::Failed;
            return Ok(MigrationRuntimeDecision::new(
                self.phase,
                MigrationRuntimeAction::Halt,
                vec![format!(
                    "SELECT citus_admin.migrate_fail({}, {});",
                    sql_literal(&self.plan.name),
                    sql_literal(reason)
                )],
                "failure is recorded durably and requires operator intervention",
            ));
        }

        match (self.phase, observation) {
            (MigrationRuntimePhase::Expand, MigrationRuntimeObservation::ExpandApplied) => {
                self.completed_sql_batches += 1;
                self.phase = MigrationRuntimePhase::Backfill;
                Ok(self.current_decision())
            }
            (
                MigrationRuntimePhase::Backfill,
                MigrationRuntimeObservation::BackfillProgress {
                    copied_rows,
                    remaining_rows,
                },
            ) => {
                self.copied_rows += copied_rows;
                if remaining_rows == 0 {
                    self.completed_sql_batches += 1;
                    self.phase = MigrationRuntimePhase::Validate;
                    Ok(self.current_decision())
                } else {
                    Ok(MigrationRuntimeDecision::new(
                        self.phase,
                        MigrationRuntimeAction::Wait,
                        Vec::new(),
                        format!(
                            "copied_rows={} remaining_rows={} continue_backfill",
                            self.copied_rows, remaining_rows
                        ),
                    ))
                }
            }
            (
                MigrationRuntimePhase::Validate,
                MigrationRuntimeObservation::RowDiffVerified { diff_rows },
            ) => {
                self.completed_sql_batches += 1;
                if diff_rows == 0 {
                    self.phase = MigrationRuntimePhase::CutoverReady;
                    Ok(self.current_decision())
                } else {
                    self.phase = MigrationRuntimePhase::Failed;
                    Err(MigrationError::RowDiffMismatch { diff_rows })
                }
            }
            (MigrationRuntimePhase::CutoverReady, MigrationRuntimeObservation::CutoverApproved) => {
                self.phase = MigrationRuntimePhase::Publish;
                Ok(self.current_decision())
            }
            (MigrationRuntimePhase::Publish, MigrationRuntimeObservation::PublishApplied) => {
                self.completed_sql_batches += 1;
                self.phase = MigrationRuntimePhase::Complete;
                Ok(self.current_decision())
            }
            (phase, observation) => Err(MigrationError::InvalidPhaseTransition {
                from: phase,
                event: observation.label(),
            }),
        }
    }

    fn expand_commands(&self) -> Vec<String> {
        let mut commands = vec![format!(
            "SELECT citus_admin.migrate_start({name}, {table}, {lock_timeout}, {batch_size});",
            name = sql_literal(&self.plan.name),
            table = sql_literal(&self.plan.table),
            lock_timeout = self.plan.lock_timeout_ms,
            batch_size = self.plan.backfill_batch_size
        )];
        commands.push(format!(
            "SELECT citus_admin.shadow_table_create({name}, {table});",
            name = sql_literal(&self.plan.name),
            table = sql_literal(&self.plan.table)
        ));
        commands.extend(self.plan.operations.iter().map(MigrationOperation::to_sql));
        commands.push(format!(
            "SELECT citus_admin.install_write_triggers({name}, {table});",
            name = sql_literal(&self.plan.name),
            table = sql_literal(&self.plan.table)
        ));
        commands
    }

    fn backfill_commands(&self) -> Vec<String> {
        vec![format!(
            "SELECT citus_admin.backfill_run({name}, {table}, {batch_size});",
            name = sql_literal(&self.plan.name),
            table = sql_literal(&self.plan.table),
            batch_size = self.plan.backfill_batch_size
        )]
    }

    fn validate_commands(&self) -> Vec<String> {
        vec![format!(
            "SELECT citus_admin.row_diff_verify({name}, {table});",
            name = sql_literal(&self.plan.name),
            table = sql_literal(&self.plan.table)
        )]
    }

    fn publish_commands(&self) -> Vec<String> {
        vec![
            format!(
                "SELECT citus_admin.shadow_table_publish({name}, {table});",
                name = sql_literal(&self.plan.name),
                table = sql_literal(&self.plan.table)
            ),
            format!(
                "SELECT citus_admin.migrate_complete({});",
                sql_literal(&self.plan.name)
            ),
        ]
    }
}

pub fn canonical_migration_runtime_report() -> Result<MigrationRuntimeReport, MigrationError> {
    let plan = MigrationPlan {
        name: "orders-total-bigint".to_string(),
        table: "public.orders".to_string(),
        operations: vec![
            MigrationOperation::AddColumn {
                column: "total_cents_v2".to_string(),
                sql_type: "bigint".to_string(),
                default_expression: None,
            },
            MigrationOperation::AlterColumnType {
                column: "total_cents".to_string(),
                from_type: "integer".to_string(),
                to_type: "bigint".to_string(),
                cast_expression: "total_cents::bigint".to_string(),
            },
        ],
        data_invariants: vec![MigrationDataInvariant {
            check_name: "orders-total-checksum".to_string(),
            check_sql: "SELECT true AS passed, count(*) AS rows_checked FROM public.orders"
                .to_string(),
        }],
        lock_timeout_ms: 500,
        backfill_batch_size: 1_000,
    };
    let mut runtime = MigrationRuntime::new(plan)?;
    let mut decisions = vec![runtime.current_decision()];
    decisions.push(runtime.observe(MigrationRuntimeObservation::ExpandApplied)?);
    decisions.push(
        runtime.observe(MigrationRuntimeObservation::BackfillProgress {
            copied_rows: 1_000,
            remaining_rows: 10,
        })?,
    );
    decisions.push(
        runtime.observe(MigrationRuntimeObservation::BackfillProgress {
            copied_rows: 10,
            remaining_rows: 0,
        })?,
    );
    decisions.push(runtime.observe(MigrationRuntimeObservation::RowDiffVerified { diff_rows: 0 })?);
    decisions.push(runtime.observe(MigrationRuntimeObservation::CutoverApproved)?);
    decisions.push(runtime.observe(MigrationRuntimeObservation::PublishApplied)?);

    let sql_batch_count = decisions
        .iter()
        .filter(|decision| decision.action == MigrationRuntimeAction::ApplySql)
        .count();
    let command_count = decisions
        .iter()
        .map(|decision| decision.commands.len())
        .sum();

    Ok(MigrationRuntimeReport {
        phase_count: 6,
        sql_batch_count,
        command_count,
        safety_guard_count: 5,
    })
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MigrationError {
    InvalidBackfillBatch,
    InvalidInvariantSql,
    InvalidLockTimeout,
    InvalidPhaseTransition {
        from: MigrationRuntimePhase,
        event: &'static str,
    },
    MissingRequiredField(&'static str),
    MissingDataInvariant,
    NoTypeChange,
    RowDiffMismatch {
        diff_rows: u64,
    },
}

impl fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBackfillBatch => {
                write!(formatter, "backfill_batch_size must be greater than zero")
            }
            Self::InvalidLockTimeout => {
                write!(formatter, "lock_timeout_ms must be greater than zero")
            }
            Self::InvalidInvariantSql => write!(
                formatter,
                "data invariant SQL must be a single read-only SELECT or WITH query"
            ),
            Self::InvalidPhaseTransition { from, event } => {
                write!(
                    formatter,
                    "cannot apply {event} while migration is in {from:?}"
                )
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::MissingDataInvariant => write!(
                formatter,
                "destructive migration operations require at least one data invariant check"
            ),
            Self::NoTypeChange => write!(formatter, "from_type and to_type must differ"),
            Self::RowDiffMismatch { diff_rows } => write!(
                formatter,
                "row_diff_verify found {diff_rows} mismatched rows; cutover is blocked"
            ),
        }
    }
}

impl Error for MigrationError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), MigrationError> {
    if value.trim().is_empty() {
        return Err(MigrationError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(field: &'static str, value: &Option<String>) -> Result<(), MigrationError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(MigrationError::MissingRequiredField(field));
    }
    Ok(())
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn optional_sql_literal(value: &Option<String>) -> String {
    value
        .as_deref()
        .map(sql_literal)
        .unwrap_or_else(|| "NULL".to_string())
}

fn normalize_invariant_sql(value: &str) -> &str {
    value.trim().trim_end_matches(';').trim()
}

fn is_select_only_sql(value: &str) -> bool {
    let normalized = normalize_invariant_sql(value);
    let lowercase = normalized.to_ascii_lowercase();
    matches!(lowercase.split_whitespace().next(), Some("select" | "with"))
        && !normalized.contains(';')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_renders_expand_contract_sequence() {
        let plan = MigrationPlan {
            name: "orders-total-bigint".to_string(),
            table: "public.orders".to_string(),
            operations: vec![
                MigrationOperation::AddColumn {
                    column: "total_cents_v2".to_string(),
                    sql_type: "bigint".to_string(),
                    default_expression: None,
                },
                MigrationOperation::AlterColumnType {
                    column: "total_cents".to_string(),
                    from_type: "integer".to_string(),
                    to_type: "bigint".to_string(),
                    cast_expression: "total_cents::bigint".to_string(),
                },
            ],
            data_invariants: vec![MigrationDataInvariant {
                check_name: "orders-total-checksum".to_string(),
                check_sql: "SELECT true AS passed, count(*) AS rows_checked FROM public.orders"
                    .to_string(),
            }],
            lock_timeout_ms: 500,
            backfill_batch_size: 1000,
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "M1");
        assert!(plan.script().contains("migrate_start"));
        assert!(plan.script().contains("migration_online_type_change"));
    }

    #[test]
    fn online_type_change_requires_distinct_types() {
        let operation = MigrationOperation::AlterColumnType {
            column: "total_cents".to_string(),
            from_type: "bigint".to_string(),
            to_type: "bigint".to_string(),
            cast_expression: "total_cents".to_string(),
        };

        assert_eq!(operation.validate(), Err(MigrationError::NoTypeChange));
    }

    #[test]
    fn gh_ost_phases_are_present_in_sql_plan() {
        let plan = MigrationPlan {
            name: "orders-total-bigint".to_string(),
            table: "public.orders".to_string(),
            operations: vec![MigrationOperation::AddColumn {
                column: "total_cents_v2".to_string(),
                sql_type: "bigint".to_string(),
                default_expression: None,
            }],
            data_invariants: Vec::new(),
            lock_timeout_ms: 500,
            backfill_batch_size: 1000,
        }
        .to_sql_plan()
        .expect("plan");
        let script = plan.script();

        // Every gh-ost phase emits a distinct citus_admin function so the
        // operator state machine has a per-shard executor invariant.
        assert!(script.contains("citus_admin.migrate_start"));
        assert!(script.contains("citus_admin.shadow_table_create"));
        assert!(script.contains("citus_admin.install_write_triggers"));
        assert!(script.contains("citus_admin.backfill_run"));
        assert!(script.contains("citus_admin.row_diff_verify"));
        assert!(script.contains("citus_admin.shadow_table_publish"));
        assert!(script.contains("citus_admin.migrate_complete"));
    }

    #[test]
    fn migration_runtime_requires_zero_diff_before_cutover() {
        let mut runtime = MigrationRuntime::new(MigrationPlan {
            name: "orders-total-bigint".to_string(),
            table: "public.orders".to_string(),
            operations: vec![MigrationOperation::AddColumn {
                column: "total_cents_v2".to_string(),
                sql_type: "bigint".to_string(),
                default_expression: None,
            }],
            data_invariants: Vec::new(),
            lock_timeout_ms: 500,
            backfill_batch_size: 1000,
        })
        .expect("runtime");

        assert_eq!(
            runtime.current_decision().phase,
            MigrationRuntimePhase::Expand
        );
        runtime
            .observe(MigrationRuntimeObservation::ExpandApplied)
            .expect("expand");
        runtime
            .observe(MigrationRuntimeObservation::BackfillProgress {
                copied_rows: 1000,
                remaining_rows: 0,
            })
            .expect("backfill");

        assert_eq!(
            runtime.observe(MigrationRuntimeObservation::RowDiffVerified { diff_rows: 7 }),
            Err(MigrationError::RowDiffMismatch { diff_rows: 7 })
        );
        assert_eq!(runtime.phase(), MigrationRuntimePhase::Failed);
    }

    #[test]
    fn migration_runtime_blocks_publish_before_validation_and_approval() {
        let mut runtime = MigrationRuntime::new(MigrationPlan {
            name: "orders-total-bigint".to_string(),
            table: "public.orders".to_string(),
            operations: vec![MigrationOperation::AddColumn {
                column: "total_cents_v2".to_string(),
                sql_type: "bigint".to_string(),
                default_expression: None,
            }],
            data_invariants: Vec::new(),
            lock_timeout_ms: 500,
            backfill_batch_size: 1000,
        })
        .expect("runtime");

        assert_eq!(
            runtime.observe(MigrationRuntimeObservation::CutoverApproved),
            Err(MigrationError::InvalidPhaseTransition {
                from: MigrationRuntimePhase::Expand,
                event: "cutover_approved"
            })
        );
    }

    #[test]
    fn canonical_migration_runtime_report_counts_guarded_batches() {
        let report = canonical_migration_runtime_report().expect("report");

        assert_eq!(report.phase_count, 6);
        assert_eq!(report.sql_batch_count, 4);
        assert_eq!(report.command_count, 9);
        assert_eq!(report.safety_guard_count, 5);
    }

    #[test]
    fn destructive_operations_require_data_invariant() {
        let result = MigrationPlan {
            name: "orders-drop-legacy-column".to_string(),
            table: "public.orders".to_string(),
            operations: vec![MigrationOperation::DropColumn {
                column: "legacy_total".to_string(),
            }],
            data_invariants: Vec::new(),
            lock_timeout_ms: 500,
            backfill_batch_size: 1000,
        }
        .to_sql_plan();

        assert_eq!(result, Err(MigrationError::MissingDataInvariant));
    }

    #[test]
    fn data_invariant_sql_must_be_read_only() {
        let result = MigrationDataInvariant {
            check_name: "bad-check".to_string(),
            check_sql: "UPDATE public.orders SET total = total".to_string(),
        }
        .validate();

        assert_eq!(result, Err(MigrationError::InvalidInvariantSql));
    }

    #[test]
    fn data_invariant_assertion_sql_is_escaped() {
        assert_eq!(
            assert_migration_data_invariants_sql("orders'v2").unwrap(),
            "SELECT companion_internal.migration_assert_invariants('orders''v2');"
        );
    }

    #[test]
    fn invariant_sql_allows_terminal_semicolon_only() {
        let invariant = MigrationDataInvariant {
            check_name: "count-check".to_string(),
            check_sql: "SELECT true AS passed;".to_string(),
        };
        assert_eq!(invariant.validate(), Ok(()));

        let injected = MigrationDataInvariant {
            check_name: "count-check".to_string(),
            check_sql: "SELECT true AS passed; DROP TABLE public.orders".to_string(),
        };
        assert_eq!(
            injected.validate(),
            Err(MigrationError::InvalidInvariantSql)
        );
    }
}
