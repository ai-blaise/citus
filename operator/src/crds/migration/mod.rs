// FEATURE: C9
// FEATURE: M3

pub mod state_machine;

use serde_yaml::{Mapping, Value};
use std::error::Error;
use std::fmt;

pub use state_machine::{transition, PhaseEvidence, StateMachineError};

pub const MIGRATION_2VI_PRECHECK_SQL: &str = "companion_internal.verify_two_version_invariant()";
pub const MIGRATION_ROLLBACK_SQL: &str = "companion_internal.schema_job_rollback_to";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationSpec {
    pub migration_type: MigrationType,
    pub yaml: String,
    pub on_conflict: MigrationConflictAction,
}

impl MigrationSpec {
    pub fn validate(&self) -> Result<(), MigrationSpecError> {
        validate_required("yaml", &self.yaml)?;
        validate_yaml_contract(&self.yaml)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MigrationType {
    Pgroll,
    GhOst,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MigrationConflictAction {
    Fail,
    Skip,
    Replace,
    ManualReview,
}

/// gh-ost-style life-cycle phases driven by [`state_machine::transition`].
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MigrationPhase {
    DeleteOnly,
    WriteOnly,
    Backfill,
    Public,
    Complete,
}

impl MigrationPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeleteOnly => "DeleteOnly",
            Self::WriteOnly => "WriteOnly",
            Self::Backfill => "Backfill",
            Self::Public => "Public",
            Self::Complete => "Complete",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MigrationSpecError {
    MissingRequiredField(&'static str),
    InvalidYaml(String),
    DocumentMustBeMap,
    UnsupportedOperation(String),
    UnsafeOperation(String),
}

impl fmt::Display for MigrationSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
            Self::InvalidYaml(message) => write!(formatter, "invalid migration yaml: {message}"),
            Self::DocumentMustBeMap => write!(formatter, "migration yaml must be a mapping"),
            Self::UnsupportedOperation(operation) => {
                write!(formatter, "unsupported migration operation: {operation}")
            }
            Self::UnsafeOperation(message) => {
                write!(formatter, "unsafe migration operation: {message}")
            }
        }
    }
}

impl Error for MigrationSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), MigrationSpecError> {
    if value.trim().is_empty() {
        return Err(MigrationSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_yaml_contract(yaml: &str) -> Result<(), MigrationSpecError> {
    let document: Value = serde_yaml::from_str(yaml)
        .map_err(|error| MigrationSpecError::InvalidYaml(error.to_string()))?;
    let mapping = document
        .as_mapping()
        .ok_or(MigrationSpecError::DocumentMustBeMap)?;

    let precheck = required_string(
        mapping,
        &[
            "twoVersionInvariantPrecheck",
            "two_version_invariant_precheck",
        ],
        "twoVersionInvariantPrecheck",
    )?;
    if normalize_sql_reference(&precheck) != normalize_sql_reference(MIGRATION_2VI_PRECHECK_SQL) {
        return Err(MigrationSpecError::UnsafeOperation(format!(
            "twoVersionInvariantPrecheck must reference {MIGRATION_2VI_PRECHECK_SQL}"
        )));
    }

    validate_rollback_reference(mapping)?;

    let operations = mapping_value(mapping, &["operations"])
        .and_then(Value::as_sequence)
        .ok_or(MigrationSpecError::MissingRequiredField("operations"))?;
    if operations.is_empty() {
        return Err(MigrationSpecError::MissingRequiredField("operations"));
    }

    for (index, operation) in operations.iter().enumerate() {
        validate_operation(index + 1, operation)?;
    }

    Ok(())
}

fn validate_rollback_reference(mapping: &Mapping) -> Result<(), MigrationSpecError> {
    let rollback = mapping_value(mapping, &["rollback"])
        .and_then(Value::as_mapping)
        .ok_or(MigrationSpecError::MissingRequiredField("rollback"))?;
    let operation = required_string(rollback, &["operation"], "rollback.operation")?;
    if !normalize_sql_reference(&operation).contains(MIGRATION_ROLLBACK_SQL) {
        return Err(MigrationSpecError::UnsafeOperation(format!(
            "rollback.operation must reference {MIGRATION_ROLLBACK_SQL}"
        )));
    }

    let target_phase = required_string(
        rollback,
        &["targetPhase", "target_phase"],
        "rollback.targetPhase",
    )?;
    match normalize_phase(&target_phase).as_str() {
        "delete_only" | "write_only" | "backfill" => Ok(()),
        other => Err(MigrationSpecError::UnsafeOperation(format!(
            "rollback.targetPhase must be delete_only, write_only, or backfill, got {other}"
        ))),
    }
}

fn validate_operation(index: usize, operation: &Value) -> Result<(), MigrationSpecError> {
    let mapping = operation.as_mapping().ok_or_else(|| {
        MigrationSpecError::UnsupportedOperation(format!("operations[{index}] must be a mapping"))
    })?;
    if mapping.len() != 1 {
        return Err(MigrationSpecError::UnsupportedOperation(format!(
            "operations[{index}] must contain exactly one operation"
        )));
    }

    let (operation_name, body) = mapping.iter().next().expect("len checked");
    let operation_name = normalize_key(operation_name)?;
    let body = body.as_mapping().ok_or_else(|| {
        MigrationSpecError::UnsupportedOperation(format!(
            "operations[{index}].{operation_name} must be a mapping"
        ))
    })?;

    match operation_name.as_str() {
        "addcolumn" => {
            validate_identifier_like(&required_string(body, &["table"], "operations.table")?)?;
            validate_identifier_like(&required_string(body, &["column"], "operations.column")?)?;
            let sql_type = required_string(body, &["sqlType", "sql_type"], "operations.sqlType")?;
            reject_unsafe_sql_fragment("operations.sqlType", &sql_type)
        }
        "backfill" => {
            let statement = required_string(body, &["statement"], "operations.statement")?;
            validate_backfill_statement(&statement)
        }
        "swapcolumn" => {
            validate_identifier_like(&required_string(
                body,
                &["oldColumn", "old_column"],
                "operations.oldColumn",
            )?)?;
            validate_identifier_like(&required_string(
                body,
                &["newColumn", "new_column"],
                "operations.newColumn",
            )?)
        }
        "dropcolumn" => {
            validate_identifier_like(&required_string(body, &["column"], "operations.column")?)
        }
        other => Err(MigrationSpecError::UnsupportedOperation(other.to_string())),
    }
}

fn mapping_value<'a>(mapping: &'a Mapping, keys: &[&str]) -> Option<&'a Value> {
    keys.iter()
        .find_map(|key| mapping.get(Value::String((*key).to_string())))
}

fn required_string(
    mapping: &Mapping,
    keys: &[&str],
    field: &'static str,
) -> Result<String, MigrationSpecError> {
    mapping_value(mapping, keys)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or(MigrationSpecError::MissingRequiredField(field))
}

fn normalize_key(value: &Value) -> Result<String, MigrationSpecError> {
    value
        .as_str()
        .map(|text| {
            text.chars()
                .filter(|character| !matches!(character, '-' | '_'))
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .ok_or_else(|| {
            MigrationSpecError::UnsupportedOperation("operation key must be a string".to_string())
        })
}

fn normalize_sql_reference(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches(';').trim();
    trimmed
        .strip_prefix("SELECT ")
        .unwrap_or(trimmed)
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn normalize_phase(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|character| if character == '-' { '_' } else { character })
        .collect::<String>()
        .to_ascii_lowercase()
}

fn validate_identifier_like(value: &str) -> Result<(), MigrationSpecError> {
    reject_unsafe_sql_fragment("identifier", value)?;
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '.'))
    {
        Ok(())
    } else {
        Err(MigrationSpecError::UnsafeOperation(format!(
            "identifier contains unsupported characters: {value}"
        )))
    }
}

fn validate_backfill_statement(statement: &str) -> Result<(), MigrationSpecError> {
    reject_unsafe_sql_fragment("operations.statement", statement)?;
    let lowered = statement.trim_start().to_ascii_lowercase();
    if !lowered.starts_with("update ") {
        return Err(MigrationSpecError::UnsafeOperation(
            "backfill statement must be a single UPDATE".to_string(),
        ));
    }
    for forbidden in [" drop ", " truncate ", " delete ", " alter "] {
        if lowered.contains(forbidden) {
            return Err(MigrationSpecError::UnsafeOperation(format!(
                "backfill statement contains forbidden token {forbidden:?}"
            )));
        }
    }
    Ok(())
}

fn reject_unsafe_sql_fragment(field: &str, value: &str) -> Result<(), MigrationSpecError> {
    if value.contains(';') || value.contains("--") || value.contains("/*") || value.contains("*/") {
        return Err(MigrationSpecError::UnsafeOperation(format!(
            "{field} must be a single SQL fragment without comments or statement separators"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_pgroll_migration_passes() {
        let spec = MigrationSpec {
            migration_type: MigrationType::Pgroll,
            yaml: valid_yaml(),
            on_conflict: MigrationConflictAction::ManualReview,
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn migration_rejects_empty_yaml() {
        let spec = MigrationSpec {
            migration_type: MigrationType::GhOst,
            yaml: String::new(),
            on_conflict: MigrationConflictAction::Fail,
        };

        assert_eq!(
            spec.validate(),
            Err(MigrationSpecError::MissingRequiredField("yaml"))
        );
    }

    #[test]
    fn migration_phase_as_str_round_trip() {
        for phase in [
            MigrationPhase::DeleteOnly,
            MigrationPhase::WriteOnly,
            MigrationPhase::Backfill,
            MigrationPhase::Public,
            MigrationPhase::Complete,
        ] {
            assert!(!phase.as_str().is_empty());
        }
    }

    #[test]
    fn migration_rejects_yaml_without_2vi_precheck() {
        let spec = MigrationSpec {
            migration_type: MigrationType::Pgroll,
            yaml: "operations:\n  - addColumn:\n      table: public.users\n      column: display_name\n      sqlType: text\nrollback:\n  operation: companion_internal.schema_job_rollback_to\n  targetPhase: write_only".to_string(),
            on_conflict: MigrationConflictAction::ManualReview,
        };

        assert_eq!(
            spec.validate(),
            Err(MigrationSpecError::MissingRequiredField(
                "twoVersionInvariantPrecheck"
            ))
        );
    }

    #[test]
    fn migration_rejects_yaml_without_rollback_reference() {
        let spec = MigrationSpec {
            migration_type: MigrationType::Pgroll,
            yaml: "twoVersionInvariantPrecheck: companion_internal.verify_two_version_invariant()\noperations:\n  - addColumn:\n      table: public.users\n      column: display_name\n      sqlType: text".to_string(),
            on_conflict: MigrationConflictAction::ManualReview,
        };

        assert_eq!(
            spec.validate(),
            Err(MigrationSpecError::MissingRequiredField("rollback"))
        );
    }

    #[test]
    fn migration_rejects_unsafe_backfill_statement() {
        let spec = MigrationSpec {
            migration_type: MigrationType::Pgroll,
            yaml: valid_yaml().replace(
                "UPDATE public.users SET display_name = email",
                "DELETE FROM public.users",
            ),
            on_conflict: MigrationConflictAction::ManualReview,
        };

        assert!(matches!(
            spec.validate(),
            Err(MigrationSpecError::UnsafeOperation(_))
        ));
    }

    fn valid_yaml() -> String {
        "twoVersionInvariantPrecheck: companion_internal.verify_two_version_invariant()\nrollback:\n  operation: companion_internal.schema_job_rollback_to\n  targetPhase: write_only\noperations:\n  - addColumn:\n      table: public.users\n      column: display_name\n      sqlType: text\n  - backfill:\n      statement: UPDATE public.users SET display_name = email"
            .to_string()
    }
}
