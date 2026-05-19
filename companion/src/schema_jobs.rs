// FEATURE: C10
// FEATURE: M2

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SchemaJobPlan {
    pub name: String,
    pub table: String,
    pub state: SchemaJobState,
    pub operations: Vec<SchemaJobOperation>,
    pub lease_seconds: u32,
}

impl SchemaJobPlan {
    pub fn validate(&self) -> Result<(), SchemaJobError> {
        validate_required("name", &self.name)?;
        validate_required("table", &self.table)?;
        if self.operations.is_empty() {
            return Err(SchemaJobError::MissingRequiredField("operations"));
        }
        for operation in &self.operations {
            operation.validate()?;
        }
        if self.lease_seconds == 0 {
            return Err(SchemaJobError::InvalidLease);
        }
        Ok(())
    }

    pub fn can_advance_to(&self, next: SchemaJobState) -> bool {
        matches!(
            (self.state, next),
            (SchemaJobState::DeleteOnly, SchemaJobState::WriteOnly)
                | (SchemaJobState::WriteOnly, SchemaJobState::Backfill)
                | (SchemaJobState::Backfill, SchemaJobState::Public)
        )
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SchemaJobState {
    DeleteOnly,
    WriteOnly,
    Backfill,
    Public,
    Paused,
    Canceled,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SchemaJobOperation {
    AddColumn {
        column: String,
        sql_type: String,
    },
    Backfill {
        statement: String,
    },
    SwapColumn {
        old_column: String,
        new_column: String,
    },
    DropColumn {
        column: String,
    },
}

impl SchemaJobOperation {
    fn validate(&self) -> Result<(), SchemaJobError> {
        match self {
            Self::AddColumn { column, sql_type } => {
                validate_required("operations.column", column)?;
                validate_required("operations.sql_type", sql_type)
            }
            Self::Backfill { statement } => validate_required("operations.statement", statement),
            Self::SwapColumn {
                old_column,
                new_column,
            } => {
                validate_required("operations.old_column", old_column)?;
                validate_required("operations.new_column", new_column)
            }
            Self::DropColumn { column } => validate_required("operations.column", column),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SchemaJobError {
    InvalidLease,
    MissingRequiredField(&'static str),
}

impl fmt::Display for SchemaJobError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLease => write!(formatter, "lease_seconds must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for SchemaJobError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), SchemaJobError> {
    if value.trim().is_empty() {
        return Err(SchemaJobError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_schema_job_passes() {
        let plan = SchemaJobPlan {
            name: "users-add-display-name".to_string(),
            table: "public.users".to_string(),
            state: SchemaJobState::DeleteOnly,
            operations: vec![SchemaJobOperation::AddColumn {
                column: "display_name".to_string(),
                sql_type: "text".to_string(),
            }],
            lease_seconds: 30,
        };

        assert_eq!(plan.validate(), Ok(()));
        assert!(plan.can_advance_to(SchemaJobState::WriteOnly));
        assert!(!plan.can_advance_to(SchemaJobState::Public));
    }

    #[test]
    fn schema_job_requires_operation_list() {
        let plan = SchemaJobPlan {
            name: "empty".to_string(),
            table: "public.users".to_string(),
            state: SchemaJobState::DeleteOnly,
            operations: Vec::new(),
            lease_seconds: 30,
        };

        assert_eq!(
            plan.validate(),
            Err(SchemaJobError::MissingRequiredField("operations"))
        );
    }

    #[test]
    fn swap_column_requires_new_column() {
        let plan = SchemaJobPlan {
            name: "users-swap-name".to_string(),
            table: "public.users".to_string(),
            state: SchemaJobState::WriteOnly,
            operations: vec![SchemaJobOperation::SwapColumn {
                old_column: "name".to_string(),
                new_column: String::new(),
            }],
            lease_seconds: 30,
        };

        assert_eq!(
            plan.validate(),
            Err(SchemaJobError::MissingRequiredField(
                "operations.new_column"
            ))
        );
    }
}
