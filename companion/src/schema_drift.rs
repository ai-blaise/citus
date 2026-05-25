// FEATURE: M4

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

const FEATURE_ID: &str = "M4";
const EXPECTED_COLUMNS_TABLE: &str = "ai_blaise_expected_schema_columns";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SchemaDriftPlan {
    pub expected_columns: Vec<ExpectedSchemaColumn>,
}

impl SchemaDriftPlan {
    pub fn validate(&self) -> Result<(), SchemaDriftError> {
        if self.expected_columns.is_empty() {
            return Err(SchemaDriftError::MissingRequiredField("expected_columns"));
        }

        let mut seen = BTreeSet::new();
        for column in &self.expected_columns {
            column.validate()?;
            let key = (
                column.schema_name.as_str(),
                column.table_name.as_str(),
                column.column_name.as_str(),
            );
            if !seen.insert(key) {
                return Err(SchemaDriftError::DuplicateColumn {
                    schema_name: column.schema_name.clone(),
                    table_name: column.table_name.clone(),
                    column_name: column.column_name.clone(),
                });
            }
        }

        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<SchemaDriftSqlPlan, SchemaDriftError> {
        self.validate()?;

        let insert_values = self
            .expected_columns
            .iter()
            .map(ExpectedSchemaColumn::to_insert_tuple)
            .collect::<Vec<_>>()
            .join(",\n  ");
        let statements = vec![
            format!(
                "CREATE TEMP TABLE {EXPECTED_COLUMNS_TABLE} (\n  table_schema text NOT NULL,\n  table_name text NOT NULL,\n  column_name text NOT NULL,\n  data_type text NOT NULL,\n  is_nullable text NOT NULL CHECK (is_nullable IN ('YES', 'NO'))\n)"
            ),
            format!(
                "INSERT INTO {EXPECTED_COLUMNS_TABLE} (table_schema, table_name, column_name, data_type, is_nullable)\nVALUES\n  {insert_values}"
            ),
            render_drift_query(),
        ];

        Ok(SchemaDriftSqlPlan {
            feature_id: FEATURE_ID,
            statements,
            expected_columns: self.expected_columns.clone(),
        })
    }

    pub fn report(&self) -> Result<SchemaDriftReport, SchemaDriftError> {
        let sql_plan = self.to_sql_plan()?;
        Ok(SchemaDriftReport {
            feature_id: FEATURE_ID,
            expected_columns: self.expected_columns.len(),
            statement_count: sql_plan.statements.len(),
            drift_kinds: vec![
                SchemaDriftKind::MissingColumn.as_str(),
                SchemaDriftKind::TypeMismatch.as_str(),
                SchemaDriftKind::NullabilityMismatch.as_str(),
                SchemaDriftKind::UnexpectedColumn.as_str(),
            ],
            information_schema_queries: sql_plan
                .statements
                .iter()
                .filter(|statement| statement.contains("information_schema.columns"))
                .count(),
            temporary_tables: sql_plan
                .statements
                .iter()
                .filter(|statement| statement.contains("CREATE TEMP TABLE"))
                .count(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExpectedSchemaColumn {
    pub schema_name: String,
    pub table_name: String,
    pub column_name: String,
    pub data_type: String,
    pub is_nullable: bool,
}

impl ExpectedSchemaColumn {
    pub fn new(
        schema_name: &str,
        table_name: &str,
        column_name: &str,
        data_type: &str,
        is_nullable: bool,
    ) -> Self {
        Self {
            schema_name: schema_name.to_string(),
            table_name: table_name.to_string(),
            column_name: column_name.to_string(),
            data_type: data_type.to_string(),
            is_nullable,
        }
    }

    fn validate(&self) -> Result<(), SchemaDriftError> {
        validate_identifier("schema_name", &self.schema_name)?;
        validate_identifier("table_name", &self.table_name)?;
        validate_identifier("column_name", &self.column_name)?;
        validate_data_type(&self.data_type)?;
        Ok(())
    }

    fn to_insert_tuple(&self) -> String {
        format!(
            "('{}', '{}', '{}', '{}', '{}')",
            sql_literal(&self.schema_name),
            sql_literal(&self.table_name),
            sql_literal(&self.column_name),
            sql_literal(&self.data_type),
            if self.is_nullable { "YES" } else { "NO" }
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SchemaDriftSqlPlan {
    pub feature_id: &'static str,
    pub statements: Vec<String>,
    pub expected_columns: Vec<ExpectedSchemaColumn>,
}

impl SchemaDriftSqlPlan {
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
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SchemaDriftReport {
    pub feature_id: &'static str,
    pub expected_columns: usize,
    pub statement_count: usize,
    pub drift_kinds: Vec<&'static str>,
    pub information_schema_queries: usize,
    pub temporary_tables: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SchemaDriftKind {
    MissingColumn,
    TypeMismatch,
    NullabilityMismatch,
    UnexpectedColumn,
}

impl SchemaDriftKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingColumn => "missing_column",
            Self::TypeMismatch => "type_mismatch",
            Self::NullabilityMismatch => "nullability_mismatch",
            Self::UnexpectedColumn => "unexpected_column",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SchemaDriftError {
    MissingRequiredField(&'static str),
    InvalidIdentifier {
        field: &'static str,
        value: String,
    },
    UnsafeDataType(String),
    DuplicateColumn {
        schema_name: String,
        table_name: String,
        column_name: String,
    },
}

impl fmt::Display for SchemaDriftError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(formatter, "{field} is required"),
            Self::InvalidIdentifier { field, value } => {
                write!(formatter, "{field} is not a safe identifier: {value}")
            }
            Self::UnsafeDataType(data_type) => {
                write!(
                    formatter,
                    "data type is not safe for schema drift SQL: {data_type}"
                )
            }
            Self::DuplicateColumn {
                schema_name,
                table_name,
                column_name,
            } => write!(
                formatter,
                "duplicate schema drift expectation for {schema_name}.{table_name}.{column_name}"
            ),
        }
    }
}

impl Error for SchemaDriftError {}

pub fn canonical_schema_drift_plan() -> SchemaDriftPlan {
    SchemaDriftPlan {
        expected_columns: vec![
            ExpectedSchemaColumn::new("public", "accounts", "id", "integer", false),
            ExpectedSchemaColumn::new("public", "accounts", "tenant_id", "text", false),
            ExpectedSchemaColumn::new("public", "accounts", "email", "text", false),
            ExpectedSchemaColumn::new(
                "public",
                "accounts",
                "updated_at",
                "timestamp with time zone",
                false,
            ),
        ],
    }
}

pub fn canonical_schema_drift_sql_plan() -> Result<SchemaDriftSqlPlan, SchemaDriftError> {
    canonical_schema_drift_plan().to_sql_plan()
}

pub fn canonical_schema_drift_report() -> Result<SchemaDriftReport, SchemaDriftError> {
    canonical_schema_drift_plan().report()
}

fn render_drift_query() -> String {
    format!(
        "WITH expected AS (\n  SELECT table_schema, table_name, column_name, data_type, is_nullable\n  FROM {EXPECTED_COLUMNS_TABLE}\n), observed AS (\n  SELECT table_schema, table_name, column_name, data_type, is_nullable\n  FROM information_schema.columns\n  WHERE table_schema NOT IN ('pg_catalog', 'information_schema')\n), target_tables AS (\n  SELECT DISTINCT table_schema, table_name\n  FROM expected\n), drift AS (\n  SELECT e.table_schema, e.table_name, e.column_name,\n         e.data_type AS expected_data_type, '' AS observed_data_type,\n         e.is_nullable AS expected_is_nullable, '' AS observed_is_nullable,\n         'missing_column' AS drift_kind\n  FROM expected e\n  LEFT JOIN observed o\n    ON o.table_schema = e.table_schema\n   AND o.table_name = e.table_name\n   AND o.column_name = e.column_name\n  WHERE o.column_name IS NULL\n  UNION ALL\n  SELECT e.table_schema, e.table_name, e.column_name,\n         e.data_type AS expected_data_type, o.data_type AS observed_data_type,\n         e.is_nullable AS expected_is_nullable, o.is_nullable AS observed_is_nullable,\n         'type_mismatch' AS drift_kind\n  FROM expected e\n  JOIN observed o\n    ON o.table_schema = e.table_schema\n   AND o.table_name = e.table_name\n   AND o.column_name = e.column_name\n  WHERE o.data_type <> e.data_type\n  UNION ALL\n  SELECT e.table_schema, e.table_name, e.column_name,\n         e.data_type AS expected_data_type, o.data_type AS observed_data_type,\n         e.is_nullable AS expected_is_nullable, o.is_nullable AS observed_is_nullable,\n         'nullability_mismatch' AS drift_kind\n  FROM expected e\n  JOIN observed o\n    ON o.table_schema = e.table_schema\n   AND o.table_name = e.table_name\n   AND o.column_name = e.column_name\n  WHERE o.is_nullable <> e.is_nullable\n  UNION ALL\n  SELECT o.table_schema, o.table_name, o.column_name,\n         '' AS expected_data_type, o.data_type AS observed_data_type,\n         '' AS expected_is_nullable, o.is_nullable AS observed_is_nullable,\n         'unexpected_column' AS drift_kind\n  FROM observed o\n  JOIN target_tables t\n    ON t.table_schema = o.table_schema\n   AND t.table_name = o.table_name\n  WHERE NOT EXISTS (\n    SELECT 1\n    FROM expected e\n    WHERE e.table_schema = o.table_schema\n      AND e.table_name = o.table_name\n      AND e.column_name = o.column_name\n  )\n)\nSELECT table_schema, table_name, column_name, expected_data_type, observed_data_type,\n       expected_is_nullable, observed_is_nullable, drift_kind\nFROM drift\nORDER BY table_schema, table_name, column_name, drift_kind"
    )
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), SchemaDriftError> {
    if value.trim().is_empty() {
        return Err(SchemaDriftError::MissingRequiredField(field));
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
        return Err(SchemaDriftError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn validate_data_type(data_type: &str) -> Result<(), SchemaDriftError> {
    if data_type.trim().is_empty() {
        return Err(SchemaDriftError::MissingRequiredField("data_type"));
    }
    if data_type.len() > 128
        || data_type
            .chars()
            .any(|character| character.is_control() || matches!(character, '\'' | ';'))
    {
        return Err(SchemaDriftError::UnsafeDataType(data_type.to_string()));
    }
    Ok(())
}

fn sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_schema_drift_report_is_deterministic() {
        let report = canonical_schema_drift_report().expect("report");

        assert_eq!(report.feature_id, "M4");
        assert_eq!(report.expected_columns, 4);
        assert_eq!(report.statement_count, 3);
        assert_eq!(report.information_schema_queries, 1);
        assert_eq!(report.temporary_tables, 1);
        assert_eq!(
            report.drift_kinds,
            vec![
                "missing_column",
                "type_mismatch",
                "nullability_mismatch",
                "unexpected_column"
            ]
        );
    }

    #[test]
    fn schema_drift_sql_uses_information_schema_and_temp_expectations() {
        let sql_plan = canonical_schema_drift_sql_plan().expect("sql plan");
        let script = sql_plan.render_psql_script();

        assert!(script.contains("CREATE TEMP TABLE ai_blaise_expected_schema_columns"));
        assert!(script.contains("information_schema.columns"));
        assert!(script.contains("'missing_column' AS drift_kind"));
        assert!(script.contains("'type_mismatch' AS drift_kind"));
        assert!(script.contains("'nullability_mismatch' AS drift_kind"));
        assert!(script.contains("'unexpected_column' AS drift_kind"));
        assert!(script.contains("ORDER BY table_schema, table_name, column_name, drift_kind;"));
    }

    #[test]
    fn schema_drift_rejects_duplicate_columns() {
        let mut plan = canonical_schema_drift_plan();
        plan.expected_columns.push(plan.expected_columns[0].clone());

        assert_eq!(
            plan.validate(),
            Err(SchemaDriftError::DuplicateColumn {
                schema_name: "public".to_string(),
                table_name: "accounts".to_string(),
                column_name: "id".to_string(),
            })
        );
    }

    #[test]
    fn schema_drift_rejects_unsafe_data_types() {
        let mut plan = canonical_schema_drift_plan();
        plan.expected_columns[0].data_type = "integer; drop table accounts".to_string();

        assert_eq!(
            plan.validate(),
            Err(SchemaDriftError::UnsafeDataType(
                "integer; drop table accounts".to_string()
            ))
        );
    }
}
