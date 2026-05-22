// FEATURE: TS9
// FEATURE: M7

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DbDoctorPlan {
    pub schemas: Vec<String>,
    pub rules: Vec<DoctorRule>,
}

impl DbDoctorPlan {
    pub fn validate(&self) -> Result<(), DbDoctorError> {
        validate_required_list("schemas", &self.schemas)?;
        if self.rules.is_empty() {
            return Err(DbDoctorError::MissingRequiredField("rules"));
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<DbDoctorSqlPlan, DbDoctorError> {
        self.validate()?;
        let rules = self
            .rules
            .iter()
            .map(|rule| rule.as_sql())
            .map(str::to_string)
            .collect::<Vec<_>>();

        DbDoctorSqlPlan::new(
            "TS9",
            vec![format!(
                "SELECT companion_internal.get_violations({}, {})::jsonb;",
                array_literal(&self.schemas),
                array_literal(&rules)
            )],
        )
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DoctorRule {
    CohabitExtensions,
    NonColocatedJoin,
    MissingDistributionColumn,
    HypertableBridge,
    ChunkIntervalOutOfBand,
}

impl DoctorRule {
    fn as_sql(self) -> &'static str {
        match self {
            Self::CohabitExtensions => "cohabit_extensions",
            Self::NonColocatedJoin => "non_colocated_join",
            Self::MissingDistributionColumn => "missing_distribution_column",
            Self::HypertableBridge => "hypertable_bridge",
            Self::ChunkIntervalOutOfBand => "chunk_interval_out_of_band",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CohabitPreflightPlan {
    pub shared_preload_libraries: Vec<String>,
    pub required_extensions: Vec<String>,
}

impl CohabitPreflightPlan {
    pub fn validate(&self) -> Result<(), DbDoctorError> {
        validate_required_list("shared_preload_libraries", &self.shared_preload_libraries)?;
        validate_required_list("required_extensions", &self.required_extensions)?;
        if !contains_value(&self.shared_preload_libraries, "citus") {
            return Err(DbDoctorError::MissingCitusPreload);
        }
        if self.missing_extensions().is_empty() {
            Ok(())
        } else {
            Err(DbDoctorError::MissingCohabitExtension)
        }
    }

    pub fn missing_extensions(&self) -> Vec<&str> {
        self.required_extensions
            .iter()
            .filter_map(|extension| {
                if contains_value(&self.shared_preload_libraries, extension) {
                    None
                } else {
                    Some(extension.as_str())
                }
            })
            .collect()
    }

    pub fn to_sql_plan(&self) -> Result<DbDoctorSqlPlan, DbDoctorError> {
        self.validate()?;
        DbDoctorSqlPlan::new(
            "M7",
            vec![
                format!(
                    "SELECT companion_internal.assert_shared_preload_libraries({}, {});",
                    array_literal(&self.shared_preload_libraries),
                    array_literal(&self.required_extensions)
                ),
                "SELECT companion_internal.assert_citus_cohabit_extension_order();".to_string(),
            ],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DoctorViolation {
    pub rule_id: String,
    pub severity: DoctorSeverity,
    pub object_name: String,
    pub message: String,
}

impl DoctorViolation {
    pub fn validate(&self) -> Result<(), DbDoctorError> {
        validate_required("rule_id", &self.rule_id)?;
        validate_required("object_name", &self.object_name)?;
        validate_required("message", &self.message)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DoctorSeverity {
    Error,
    Warning,
    Note,
}

impl DoctorSeverity {
    fn as_sarif(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DbDoctorReport {
    pub violations: Vec<DoctorViolation>,
}

impl DbDoctorReport {
    pub fn validate(&self) -> Result<(), DbDoctorError> {
        for violation in &self.violations {
            violation.validate()?;
        }
        Ok(())
    }

    pub fn to_sarif(&self) -> Result<String, DbDoctorError> {
        self.validate()?;
        let results = self
            .violations
            .iter()
            .map(|violation| {
                format!(
                    "{{\"ruleId\":\"{}\",\"level\":\"{}\",\"message\":{{\"text\":\"{}\"}},\"locations\":[{{\"physicalLocation\":{{\"artifactLocation\":{{\"uri\":\"{}\"}}}}}}]}}",
                    json_escape(&violation.rule_id),
                    violation.severity.as_sarif(),
                    json_escape(&violation.message),
                    json_escape(&violation.object_name)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        Ok(format!(
            "{{\"version\":\"2.1.0\",\"runs\":[{{\"tool\":{{\"driver\":{{\"name\":\"ai-blaise-db-doctor\"}}}},\"results\":[{results}]}}]}}"
        ))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DbDoctorSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl DbDoctorSqlPlan {
    fn new(feature_id: &'static str, commands: Vec<String>) -> Result<Self, DbDoctorError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(DbDoctorError::MissingRequiredField("commands"));
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
pub enum DbDoctorError {
    MissingCitusPreload,
    MissingCohabitExtension,
    MissingRequiredField(&'static str),
}

impl fmt::Display for DbDoctorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCitusPreload => write!(formatter, "citus must be preloaded"),
            Self::MissingCohabitExtension => {
                write!(formatter, "required cohabiting extension is not preloaded")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for DbDoctorError {}

fn contains_value(values: &[String], needle: &str) -> bool {
    values
        .iter()
        .any(|value| value.trim().eq_ignore_ascii_case(needle))
}

fn validate_required(field: &'static str, value: &str) -> Result<(), DbDoctorError> {
    if value.trim().is_empty() {
        return Err(DbDoctorError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(field: &'static str, values: &[String]) -> Result<(), DbDoctorError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(DbDoctorError::MissingRequiredField(field));
    }
    Ok(())
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn array_literal(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| sql_literal(value))
        .collect::<Vec<_>>()
        .join(", ");
    format!("ARRAY[{values}]")
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_doctor_renders_citus_aware_rules() {
        let plan = DbDoctorPlan {
            schemas: vec!["public".to_string(), "tenant_a".to_string()],
            rules: vec![
                DoctorRule::NonColocatedJoin,
                DoctorRule::MissingDistributionColumn,
            ],
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "TS9");
        assert!(plan.script().contains("get_violations"));
        assert!(plan.script().contains("non_colocated_join"));
    }

    #[test]
    fn cohabit_preflight_reports_missing_extension() {
        let plan = CohabitPreflightPlan {
            shared_preload_libraries: vec!["citus".to_string()],
            required_extensions: vec!["timescaledb".to_string()],
        };

        assert_eq!(plan.missing_extensions(), vec!["timescaledb"]);
        assert_eq!(plan.validate(), Err(DbDoctorError::MissingCohabitExtension));
    }

    #[test]
    fn doctor_report_renders_sarif() {
        let report = DbDoctorReport {
            violations: vec![DoctorViolation {
                rule_id: "missing_distribution_column".to_string(),
                severity: DoctorSeverity::Error,
                object_name: "public.events".to_string(),
                message: "table is missing a distribution column".to_string(),
            }],
        };

        let sarif = report.to_sarif().unwrap();
        assert!(sarif.contains("\"version\":\"2.1.0\""));
        assert!(sarif.contains("missing_distribution_column"));
    }
}
