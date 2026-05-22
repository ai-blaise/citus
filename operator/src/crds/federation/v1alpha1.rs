// FEATURE: F1

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FederationSpec {
    pub name: String,
    pub federation_type: FederationType,
    pub connection: FederationConnection,
    pub foreign_schema_prefix: String,
}

impl FederationSpec {
    pub fn validate(&self) -> Result<(), FederationSpecError> {
        validate_required("name", &self.name)?;
        validate_required("foreign_schema_prefix", &self.foreign_schema_prefix)?;
        self.connection.validate()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FederationType {
    Snowflake,
    BigQuery,
    Databricks,
    MySql,
    Mongo,
    Oracle,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FederationConnection {
    pub secret_ref: String,
}

impl FederationConnection {
    fn validate(&self) -> Result<(), FederationSpecError> {
        validate_required("connection.secret_ref", &self.secret_ref)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FederationSpecError {
    MissingRequiredField(&'static str),
}

impl fmt::Display for FederationSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for FederationSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), FederationSpecError> {
    if value.trim().is_empty() {
        return Err(FederationSpecError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_federation_passes() {
        let spec = FederationSpec {
            name: "warehouse".to_string(),
            federation_type: FederationType::Snowflake,
            connection: FederationConnection {
                secret_ref: "snowflake-warehouse".to_string(),
            },
            foreign_schema_prefix: "snowflake_".to_string(),
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn federation_rejects_empty_secret() {
        let mut spec = minimal_spec();
        spec.connection.secret_ref = String::new();

        assert_eq!(
            spec.validate(),
            Err(FederationSpecError::MissingRequiredField(
                "connection.secret_ref"
            ))
        );
    }

    fn minimal_spec() -> FederationSpec {
        FederationSpec {
            name: "analytics".to_string(),
            federation_type: FederationType::Databricks,
            connection: FederationConnection {
                secret_ref: "databricks-analytics".to_string(),
            },
            foreign_schema_prefix: "lakehouse_".to_string(),
        }
    }
}
