// FEATURE: M1
// FEATURE: M11

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationPlan {
    pub name: String,
    pub table: String,
    pub operations: Vec<MigrationOperation>,
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MigrationError {
    InvalidBackfillBatch,
    InvalidLockTimeout,
    MissingRequiredField(&'static str),
    NoTypeChange,
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
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::NoTypeChange => write!(formatter, "from_type and to_type must differ"),
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
}
