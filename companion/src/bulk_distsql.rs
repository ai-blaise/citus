// FEATURE: T10
// FEATURE: T11

use std::error::Error;
use std::fmt;

const FEATURE_IDS: &[&str] = &["T10", "T11"];
const DEFAULT_TABLE: &str = "public.bulk_distsql_orders";
const DEFAULT_CURSOR_NAME: &str = "ai_blaise_bulk_fetch_cursor";
const DEFAULT_BATCH_ROWS: u32 = 4096;
const DEFAULT_WORKER_TASK_BUDGET: u32 = 16;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BulkDistSqlPlan {
    pub table_name: String,
    pub tenant_column: String,
    pub order_column: String,
    pub amount_column: String,
    pub tenant_id: i64,
    pub cursor_name: String,
    pub max_batch_rows: u32,
    pub worker_task_budget: u32,
}

impl BulkDistSqlPlan {
    pub fn validate(&self) -> Result<(), BulkDistSqlError> {
        validate_qualified_identifier("table_name", &self.table_name)?;
        validate_identifier("tenant_column", &self.tenant_column)?;
        validate_identifier("order_column", &self.order_column)?;
        validate_identifier("amount_column", &self.amount_column)?;
        validate_identifier("cursor_name", &self.cursor_name)?;
        if self.tenant_id <= 0 {
            return Err(BulkDistSqlError::InvalidPositive("tenant_id"));
        }
        if self.max_batch_rows == 0 {
            return Err(BulkDistSqlError::InvalidPositive("max_batch_rows"));
        }
        if self.worker_task_budget == 0 {
            return Err(BulkDistSqlError::InvalidPositive("worker_task_budget"));
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<BulkDistSqlSqlPlan, BulkDistSqlError> {
        self.validate()?;
        let table = quote_qualified_identifier("table_name", &self.table_name)?;
        let tenant_column = quote_identifier("tenant_column", &self.tenant_column)?;
        let order_column = quote_identifier("order_column", &self.order_column)?;
        let amount_column = quote_identifier("amount_column", &self.amount_column)?;
        let cursor_name = quote_identifier("cursor_name", &self.cursor_name)?;
        let statements = vec![
            "BEGIN".to_string(),
            format!(
                "DECLARE {cursor_name} NO SCROLL CURSOR FOR\nSELECT 'bulk_fetch_row' AS marker, {order_column}::text AS value, {amount_column}::text AS detail\nFROM {table}\nWHERE {tenant_column} = {tenant_id}\nORDER BY {order_column}",
                tenant_id = self.tenant_id,
            ),
            format!("FETCH {} FROM {cursor_name}", self.max_batch_rows),
            "COMMIT".to_string(),
            format!(
                "SELECT 'bulk_fetch_rows_returned' AS marker, count(*)::text AS value, '' AS detail\nFROM (\n  SELECT 1\n  FROM {table}\n  WHERE {tenant_column} = {tenant_id}\n  ORDER BY {order_column}\n  LIMIT {max_batch_rows}\n) AS fetched",
                tenant_id = self.tenant_id,
                max_batch_rows = self.max_batch_rows,
            ),
            format!(
                "EXPLAIN (COSTS OFF) SELECT {tenant_column}, count(*) AS row_count, sum({amount_column}) AS total_amount\nFROM {table}\nWHERE {tenant_column} = {tenant_id}\nGROUP BY {tenant_column}",
                tenant_id = self.tenant_id,
            ),
        ];

        Ok(BulkDistSqlSqlPlan {
            feature_ids: FEATURE_IDS,
            statements,
            table_name: self.table_name.clone(),
            max_batch_rows: self.max_batch_rows,
            worker_task_budget: self.worker_task_budget,
        })
    }

    pub fn report(&self) -> Result<BulkDistSqlReport, BulkDistSqlError> {
        let sql_plan = self.to_sql_plan()?;
        let script = sql_plan.render_psql_script();
        Ok(BulkDistSqlReport {
            feature_ids: FEATURE_IDS,
            table_name: self.table_name.clone(),
            statement_count: sql_plan.statements.len(),
            cursor_declared: script.contains("DECLARE") && script.contains("NO SCROLL CURSOR"),
            bulk_fetch_budget_enforced: script
                .contains(&format!("FETCH {} FROM", self.max_batch_rows)),
            distsql_explain_required: script.contains("EXPLAIN (COSTS OFF) SELECT"),
            max_batch_rows: self.max_batch_rows,
            worker_task_budget: self.worker_task_budget,
            fail_closed_checks: canonical_bulk_distsql_fail_closed_checks(),
            wire_protocol_implementation_exercised: false,
            backpressure_scheduler_exercised: false,
            physical_plan_rewrite_exercised: false,
            multi_worker_fanout_exercised: false,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BulkDistSqlSqlPlan {
    pub feature_ids: &'static [&'static str],
    pub statements: Vec<String>,
    pub table_name: String,
    pub max_batch_rows: u32,
    pub worker_task_budget: u32,
}

impl BulkDistSqlSqlPlan {
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
pub struct BulkDistSqlReport {
    pub feature_ids: &'static [&'static str],
    pub table_name: String,
    pub statement_count: usize,
    pub cursor_declared: bool,
    pub bulk_fetch_budget_enforced: bool,
    pub distsql_explain_required: bool,
    pub max_batch_rows: u32,
    pub worker_task_budget: u32,
    pub fail_closed_checks: usize,
    pub wire_protocol_implementation_exercised: bool,
    pub backpressure_scheduler_exercised: bool,
    pub physical_plan_rewrite_exercised: bool,
    pub multi_worker_fanout_exercised: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BulkDistSqlError {
    MissingRequiredField(&'static str),
    InvalidIdentifier { field: &'static str, value: String },
    InvalidPositive(&'static str),
}

impl fmt::Display for BulkDistSqlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(formatter, "{field} is required"),
            Self::InvalidIdentifier { field, value } => {
                write!(
                    formatter,
                    "{field} is not a safe PostgreSQL identifier: {value}"
                )
            }
            Self::InvalidPositive(field) => write!(formatter, "{field} must be greater than zero"),
        }
    }
}

impl Error for BulkDistSqlError {}

pub fn canonical_bulk_distsql_plan() -> BulkDistSqlPlan {
    BulkDistSqlPlan {
        table_name: DEFAULT_TABLE.to_string(),
        tenant_column: "tenant_id".to_string(),
        order_column: "order_id".to_string(),
        amount_column: "total".to_string(),
        tenant_id: 1,
        cursor_name: DEFAULT_CURSOR_NAME.to_string(),
        max_batch_rows: DEFAULT_BATCH_ROWS,
        worker_task_budget: DEFAULT_WORKER_TASK_BUDGET,
    }
}

pub fn canonical_bulk_distsql_sql_plan() -> Result<BulkDistSqlSqlPlan, BulkDistSqlError> {
    canonical_bulk_distsql_plan().to_sql_plan()
}

pub fn canonical_bulk_distsql_report() -> Result<BulkDistSqlReport, BulkDistSqlError> {
    canonical_bulk_distsql_plan().report()
}

pub fn canonical_bulk_distsql_fail_closed_checks() -> usize {
    let mut checks = 0;

    let mut missing_table = canonical_bulk_distsql_plan();
    missing_table.table_name.clear();
    if matches!(
        missing_table.validate(),
        Err(BulkDistSqlError::MissingRequiredField("table_name"))
    ) {
        checks += 1;
    }

    let mut unsafe_cursor = canonical_bulk_distsql_plan();
    unsafe_cursor.cursor_name = "cursor;drop".to_string();
    if matches!(
        unsafe_cursor.validate(),
        Err(BulkDistSqlError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut zero_batch = canonical_bulk_distsql_plan();
    zero_batch.max_batch_rows = 0;
    if matches!(
        zero_batch.validate(),
        Err(BulkDistSqlError::InvalidPositive("max_batch_rows"))
    ) {
        checks += 1;
    }

    let mut zero_budget = canonical_bulk_distsql_plan();
    zero_budget.worker_task_budget = 0;
    if matches!(
        zero_budget.validate(),
        Err(BulkDistSqlError::InvalidPositive("worker_task_budget"))
    ) {
        checks += 1;
    }

    let mut zero_tenant = canonical_bulk_distsql_plan();
    zero_tenant.tenant_id = 0;
    if matches!(
        zero_tenant.validate(),
        Err(BulkDistSqlError::InvalidPositive("tenant_id"))
    ) {
        checks += 1;
    }

    checks
}

fn quote_qualified_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, BulkDistSqlError> {
    validate_qualified_identifier(field, value)?;
    value
        .split('.')
        .map(|part| quote_identifier(field, part))
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("."))
}

fn validate_qualified_identifier(field: &'static str, value: &str) -> Result<(), BulkDistSqlError> {
    if value.trim().is_empty() {
        return Err(BulkDistSqlError::MissingRequiredField(field));
    }
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 2 {
        return Err(BulkDistSqlError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    for part in parts {
        validate_identifier(field, part)?;
    }
    Ok(())
}

fn quote_identifier(field: &'static str, value: &str) -> Result<String, BulkDistSqlError> {
    validate_identifier(field, value)?;
    Ok(format!("\"{value}\""))
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), BulkDistSqlError> {
    if value.trim().is_empty() {
        return Err(BulkDistSqlError::MissingRequiredField(field));
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
        return Err(BulkDistSqlError::InvalidIdentifier {
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
    fn canonical_report_covers_bulk_fetch_and_distsql_boundaries() {
        let report = canonical_bulk_distsql_report().expect("report");

        assert_eq!(report.feature_ids, ["T10", "T11"]);
        assert_eq!(report.table_name, DEFAULT_TABLE);
        assert_eq!(report.statement_count, 6);
        assert!(report.cursor_declared);
        assert!(report.bulk_fetch_budget_enforced);
        assert!(report.distsql_explain_required);
        assert_eq!(report.max_batch_rows, 4096);
        assert_eq!(report.worker_task_budget, 16);
        assert_eq!(report.fail_closed_checks, 5);
        assert!(!report.wire_protocol_implementation_exercised);
        assert!(!report.backpressure_scheduler_exercised);
        assert!(!report.physical_plan_rewrite_exercised);
        assert!(!report.multi_worker_fanout_exercised);
    }

    #[test]
    fn sql_plan_renders_cursor_fetch_budget_and_explain() {
        let sql_plan = canonical_bulk_distsql_sql_plan().expect("sql plan");
        let script = sql_plan.render_psql_script();

        assert!(script.contains("DECLARE \"ai_blaise_bulk_fetch_cursor\" NO SCROLL CURSOR"));
        assert!(script.contains("SELECT 'bulk_fetch_row' AS marker"));
        assert!(script.contains("FETCH 4096 FROM \"ai_blaise_bulk_fetch_cursor\";"));
        assert!(script.contains("SELECT 'bulk_fetch_rows_returned' AS marker"));
        assert!(script.contains("EXPLAIN (COSTS OFF) SELECT"));
        assert!(script.contains("GROUP BY \"tenant_id\";"));
    }

    #[test]
    fn rejects_unsafe_cursor_name() {
        let mut plan = canonical_bulk_distsql_plan();
        plan.cursor_name = "cursor;drop".to_string();

        assert_eq!(
            plan.validate(),
            Err(BulkDistSqlError::InvalidIdentifier {
                field: "cursor_name",
                value: "cursor;drop".to_string(),
            })
        );
    }

    #[test]
    fn rejects_zero_batch_rows() {
        let mut plan = canonical_bulk_distsql_plan();
        plan.max_batch_rows = 0;

        assert_eq!(
            plan.validate(),
            Err(BulkDistSqlError::InvalidPositive("max_batch_rows"))
        );
    }

    #[test]
    fn rejects_zero_worker_task_budget() {
        let mut plan = canonical_bulk_distsql_plan();
        plan.worker_task_budget = 0;

        assert_eq!(
            plan.validate(),
            Err(BulkDistSqlError::InvalidPositive("worker_task_budget"))
        );
    }
}
