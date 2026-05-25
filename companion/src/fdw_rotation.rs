// FEATURE: F4

use std::error::Error;
use std::fmt;

const FEATURE_ID: &str = "F4";
const FDW_PASSWORD_VARIABLE: &str = "fdw_new_password";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FdwCredentialRotationPlan {
    pub server_name: String,
    pub mapping_user: FdwUserMapping,
    pub old_secret_ref: String,
    pub new_secret_ref: String,
    pub validation_table: String,
    pub validation_min_rows: u64,
}

impl FdwCredentialRotationPlan {
    pub fn validate(&self) -> Result<(), FdwRotationError> {
        validate_identifier("server_name", &self.server_name)?;
        self.mapping_user.validate()?;
        validate_secret_ref("old_secret_ref", &self.old_secret_ref)?;
        validate_secret_ref("new_secret_ref", &self.new_secret_ref)?;
        if self.old_secret_ref.trim() == self.new_secret_ref.trim() {
            return Err(FdwRotationError::SecretRefsMustDiffer);
        }
        quote_qualified_identifier("validation_table", &self.validation_table)?;
        if self.validation_min_rows == 0 {
            return Err(FdwRotationError::InvalidPositive("validation_min_rows"));
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<FdwCredentialRotationSqlPlan, FdwRotationError> {
        self.validate()?;

        let server_sql = quote_identifier("server_name", &self.server_name)?;
        let mapping_user_sql = self.mapping_user.to_sql()?;
        let validation_table_sql =
            quote_qualified_identifier("validation_table", &self.validation_table)?;
        let statements = vec![
            "BEGIN".to_string(),
            "SELECT postgres_fdw_disconnect_all()".to_string(),
            format!(
                "ALTER USER MAPPING FOR {mapping_user_sql} SERVER {server_sql} OPTIONS (SET password :'{FDW_PASSWORD_VARIABLE}')"
            ),
            "SELECT postgres_fdw_disconnect_all()".to_string(),
            format!(
                "SELECT count(*) >= {} AS fdw_rotation_valid FROM {validation_table_sql}",
                self.validation_min_rows
            ),
            "COMMIT".to_string(),
        ];

        Ok(FdwCredentialRotationSqlPlan {
            feature_id: FEATURE_ID,
            password_variable: FDW_PASSWORD_VARIABLE.to_string(),
            statements,
            old_secret_ref: self.old_secret_ref.clone(),
            new_secret_ref: self.new_secret_ref.clone(),
        })
    }

    pub fn report(&self) -> Result<FdwCredentialRotationReport, FdwRotationError> {
        let sql_plan = self.to_sql_plan()?;
        let script = sql_plan.render_psql_script();
        Ok(FdwCredentialRotationReport {
            feature_id: FEATURE_ID,
            server_name: self.server_name.clone(),
            mapping_user: self.mapping_user.as_report_value(),
            validation_table: self.validation_table.clone(),
            statement_count: sql_plan.statements.len(),
            disconnect_calls: sql_plan.disconnect_calls(),
            uses_secret_variable: script.contains(&format!(":'{FDW_PASSWORD_VARIABLE}'")),
            plan_secret_literals: sql_plan.contains_literal_secret_reference(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FdwUserMapping {
    CurrentUser,
    Public,
    User(String),
}

impl FdwUserMapping {
    fn validate(&self) -> Result<(), FdwRotationError> {
        if let Self::User(user_name) = self {
            validate_identifier("mapping_user", user_name)?;
        }
        Ok(())
    }

    fn to_sql(&self) -> Result<String, FdwRotationError> {
        match self {
            Self::CurrentUser => Ok("CURRENT_USER".to_string()),
            Self::Public => Ok("PUBLIC".to_string()),
            Self::User(user_name) => quote_identifier("mapping_user", user_name),
        }
    }

    fn as_report_value(&self) -> String {
        match self {
            Self::CurrentUser => "CURRENT_USER".to_string(),
            Self::Public => "PUBLIC".to_string(),
            Self::User(user_name) => user_name.clone(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FdwCredentialRotationSqlPlan {
    pub feature_id: &'static str,
    pub password_variable: String,
    pub statements: Vec<String>,
    old_secret_ref: String,
    new_secret_ref: String,
}

impl FdwCredentialRotationSqlPlan {
    pub fn render_psql_script(&self) -> String {
        self.statements
            .iter()
            .map(|statement| {
                if statement.ends_with(';') {
                    statement.clone()
                } else {
                    format!("{statement};")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn disconnect_calls(&self) -> usize {
        self.statements
            .iter()
            .filter(|statement| statement.contains("postgres_fdw_disconnect_all()"))
            .count()
    }

    pub fn contains_literal_secret_reference(&self) -> bool {
        let script = self.render_psql_script();
        script.contains(&self.old_secret_ref) || script.contains(&self.new_secret_ref)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FdwCredentialRotationReport {
    pub feature_id: &'static str,
    pub server_name: String,
    pub mapping_user: String,
    pub validation_table: String,
    pub statement_count: usize,
    pub disconnect_calls: usize,
    pub uses_secret_variable: bool,
    pub plan_secret_literals: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FdwRotationError {
    MissingRequiredField(&'static str),
    InvalidIdentifier { field: &'static str, value: String },
    UnsafeSecretRef(&'static str),
    SecretRefsMustDiffer,
    InvalidPositive(&'static str),
}

impl fmt::Display for FdwRotationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(formatter, "{field} is required"),
            Self::InvalidIdentifier { field, value } => {
                write!(
                    formatter,
                    "{field} is not a safe PostgreSQL identifier: {value}"
                )
            }
            Self::UnsafeSecretRef(field) => {
                write!(formatter, "{field} must be a non-SQL secret reference")
            }
            Self::SecretRefsMustDiffer => write!(formatter, "old and new secret refs must differ"),
            Self::InvalidPositive(field) => write!(formatter, "{field} must be greater than zero"),
        }
    }
}

impl Error for FdwRotationError {}

pub fn canonical_fdw_credential_rotation_plan() -> FdwCredentialRotationPlan {
    FdwCredentialRotationPlan {
        server_name: "ai_blaise_remote".to_string(),
        mapping_user: FdwUserMapping::CurrentUser,
        old_secret_ref: "k8s/fdw-remote/old-password".to_string(),
        new_secret_ref: "k8s/fdw-remote/new-password".to_string(),
        validation_table: "public.fdw_items_remote".to_string(),
        validation_min_rows: 1,
    }
}

pub fn canonical_fdw_credential_rotation_sql_plan(
) -> Result<FdwCredentialRotationSqlPlan, FdwRotationError> {
    canonical_fdw_credential_rotation_plan().to_sql_plan()
}

pub fn canonical_fdw_credential_rotation_report(
) -> Result<FdwCredentialRotationReport, FdwRotationError> {
    canonical_fdw_credential_rotation_plan().report()
}

fn validate_secret_ref(field: &'static str, value: &str) -> Result<(), FdwRotationError> {
    if value.trim().is_empty() {
        return Err(FdwRotationError::MissingRequiredField(field));
    }
    if value.len() > 256
        || value
            .chars()
            .any(|character| character.is_control() || matches!(character, '\'' | '"' | ';'))
    {
        return Err(FdwRotationError::UnsafeSecretRef(field));
    }
    Ok(())
}

fn quote_qualified_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, FdwRotationError> {
    if value.trim().is_empty() {
        return Err(FdwRotationError::MissingRequiredField(field));
    }

    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() > 3 {
        return Err(FdwRotationError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }

    parts
        .iter()
        .map(|part| quote_identifier(field, part))
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("."))
}

fn quote_identifier(field: &'static str, value: &str) -> Result<String, FdwRotationError> {
    validate_identifier(field, value)?;
    Ok(format!("\"{value}\""))
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), FdwRotationError> {
    if value.trim().is_empty() {
        return Err(FdwRotationError::MissingRequiredField(field));
    }
    if value.len() > 63
        || value
            .chars()
            .next()
            .is_some_and(|character| !(character == '_' || character.is_ascii_alphabetic()))
        || !value
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return Err(FdwRotationError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_fdw_rotation_report_is_secret_safe() {
        let report = canonical_fdw_credential_rotation_report().expect("report");

        assert_eq!(report.feature_id, "F4");
        assert_eq!(report.server_name, "ai_blaise_remote");
        assert_eq!(report.mapping_user, "CURRENT_USER");
        assert_eq!(report.validation_table, "public.fdw_items_remote");
        assert_eq!(report.statement_count, 6);
        assert_eq!(report.disconnect_calls, 2);
        assert!(report.uses_secret_variable);
        assert!(!report.plan_secret_literals);
    }

    #[test]
    fn fdw_rotation_sql_uses_psql_secret_variable() {
        let sql_plan = canonical_fdw_credential_rotation_sql_plan().expect("sql plan");
        let script = sql_plan.render_psql_script();

        assert!(script.contains("ALTER USER MAPPING FOR CURRENT_USER"));
        assert!(script.contains("SERVER \"ai_blaise_remote\""));
        assert!(script.contains("OPTIONS (SET password :'fdw_new_password')"));
        assert!(script.contains("SELECT postgres_fdw_disconnect_all();"));
        assert!(script.contains("FROM \"public\".\"fdw_items_remote\""));
        assert!(!sql_plan.contains_literal_secret_reference());
    }

    #[test]
    fn fdw_rotation_rejects_same_secret_refs() {
        let mut plan = canonical_fdw_credential_rotation_plan();
        plan.new_secret_ref.clone_from(&plan.old_secret_ref);

        assert_eq!(plan.validate(), Err(FdwRotationError::SecretRefsMustDiffer));
    }

    #[test]
    fn fdw_rotation_rejects_unsafe_identifiers() {
        let mut plan = canonical_fdw_credential_rotation_plan();
        plan.server_name = "remote;drop".to_string();

        assert_eq!(
            plan.validate(),
            Err(FdwRotationError::InvalidIdentifier {
                field: "server_name",
                value: "remote;drop".to_string(),
            })
        );
    }

    #[test]
    fn fdw_rotation_renders_named_user_mapping() {
        let mut plan = canonical_fdw_credential_rotation_plan();
        plan.mapping_user = FdwUserMapping::User("fdw_owner".to_string());

        let script = plan.to_sql_plan().expect("sql plan").render_psql_script();

        assert!(script.contains("ALTER USER MAPPING FOR \"fdw_owner\""));
    }
}
