// FEATURE: JS2
// FEATURE: M13

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JsonSchemaDistributedPlan {
    pub table: String,
    pub json_column: String,
    pub schema_name: String,
    pub schema_document: String,
    pub timing: ValidationTiming,
}

impl JsonSchemaDistributedPlan {
    pub fn validate(&self) -> Result<(), JsonSchemaError> {
        validate_required("table", &self.table)?;
        validate_required("json_column", &self.json_column)?;
        validate_required("schema_name", &self.schema_name)?;
        validate_required("schema_document", &self.schema_document)
    }

    pub fn to_sql_plan(&self) -> Result<JsonSchemaSqlPlan, JsonSchemaError> {
        self.validate()?;
        JsonSchemaSqlPlan::new(
            "JS2",
            vec![
                format!(
                    "SELECT companion_internal.register_json_schema({}, {}::jsonb);",
                    sql_literal(&self.schema_name),
                    sql_literal(&self.schema_document)
                ),
                format!(
                    "SELECT companion_internal.install_jsonschema_trigger({}, {}, {}, {});",
                    sql_literal(&self.table),
                    sql_literal(&self.json_column),
                    sql_literal(&self.schema_name),
                    sql_literal(self.timing.as_sql())
                ),
                format!(
                    "SELECT create_distributed_function('companion_internal.validate_jsonschema_shard(regclass,text,text)');"
                ),
            ],
        )
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ValidationTiming {
    BeforeInsertOrUpdate,
    AfterInsertOrUpdate,
}

impl ValidationTiming {
    fn as_sql(self) -> &'static str {
        match self {
            Self::BeforeInsertOrUpdate => "BEFORE INSERT OR UPDATE",
            Self::AfterInsertOrUpdate => "AFTER INSERT OR UPDATE",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JsonSchemaSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl JsonSchemaSqlPlan {
    fn new(feature_id: &'static str, commands: Vec<String>) -> Result<Self, JsonSchemaError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(JsonSchemaError::MissingRequiredField("commands"));
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
pub enum JsonSchemaError {
    MissingRequiredField(&'static str),
}

impl fmt::Display for JsonSchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for JsonSchemaError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), JsonSchemaError> {
    if value.trim().is_empty() {
        return Err(JsonSchemaError::MissingRequiredField(field));
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
    fn distributed_jsonschema_plan_installs_schema_and_trigger() {
        let plan = JsonSchemaDistributedPlan {
            table: "tenant_a.events".to_string(),
            json_column: "payload".to_string(),
            schema_name: "event_schema".to_string(),
            schema_document: r#"{"type":"object"}"#.to_string(),
            timing: ValidationTiming::BeforeInsertOrUpdate,
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "JS2");
        assert!(plan.script().contains("register_json_schema"));
        assert!(plan.script().contains("install_jsonschema_trigger"));
        assert!(plan.script().contains("create_distributed_function"));
    }
}
