// FEATURE: T13
// FEATURE: T14

use std::error::Error;
use std::fmt;

const FEATURE_IDS: &[&str] = &["T13", "T14"];
const DEFAULT_TABLE: &str = "public.txn_state_orders";
const DEFAULT_CURSOR_NAME: &str = "ai_blaise_txn_cursor";
const DEFAULT_SAVEPOINT_NAME: &str = "ai_blaise_before_extra";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DistributedTransactionStatePlan {
    pub table_name: String,
    pub tenant_column: String,
    pub order_column: String,
    pub amount_column: String,
    pub tenant_id: i64,
    pub cursor_name: String,
    pub savepoint_name: String,
    pub fetch_batch_rows: u32,
    pub sentinel_order_id: i64,
}

impl DistributedTransactionStatePlan {
    pub fn validate(&self) -> Result<(), TransactionStateError> {
        validate_qualified_identifier("table_name", &self.table_name)?;
        validate_identifier("tenant_column", &self.tenant_column)?;
        validate_identifier("order_column", &self.order_column)?;
        validate_identifier("amount_column", &self.amount_column)?;
        validate_identifier("cursor_name", &self.cursor_name)?;
        validate_identifier("savepoint_name", &self.savepoint_name)?;
        if self.fetch_batch_rows == 0 {
            return Err(TransactionStateError::InvalidPositive("fetch_batch_rows"));
        }
        if self.tenant_id <= 0 {
            return Err(TransactionStateError::InvalidPositive("tenant_id"));
        }
        if self.sentinel_order_id <= 0 {
            return Err(TransactionStateError::InvalidPositive("sentinel_order_id"));
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<DistributedTransactionStateSqlPlan, TransactionStateError> {
        self.validate()?;
        let table = quote_qualified_identifier("table_name", &self.table_name)?;
        let tenant_column = quote_identifier("tenant_column", &self.tenant_column)?;
        let order_column = quote_identifier("order_column", &self.order_column)?;
        let amount_column = quote_identifier("amount_column", &self.amount_column)?;
        let cursor_name = quote_identifier("cursor_name", &self.cursor_name)?;
        let savepoint_name = quote_identifier("savepoint_name", &self.savepoint_name)?;
        let statements = vec![
            "BEGIN".to_string(),
            format!(
                "DECLARE {cursor_name} NO SCROLL CURSOR FOR\nSELECT 'cursor_row' AS marker, {order_column}::text AS value, {amount_column}::text AS detail\nFROM {table}\nWHERE {tenant_column} = {tenant_id}\nORDER BY {order_column}",
                tenant_id = self.tenant_id
            ),
            format!("FETCH {} FROM {cursor_name}", self.fetch_batch_rows),
            format!("SAVEPOINT {savepoint_name}"),
            format!(
                "INSERT INTO {table} ({tenant_column}, {order_column}, {amount_column})\nVALUES ({tenant_id}, {sentinel_order_id}, {sentinel_order_id})",
                tenant_id = self.tenant_id,
                sentinel_order_id = self.sentinel_order_id,
            ),
            format!(
                "SELECT 'count_after_insert' AS marker, count(*)::text AS value, '' AS detail\nFROM {table}\nWHERE {tenant_column} = {tenant_id}",
                tenant_id = self.tenant_id,
            ),
            format!("ROLLBACK TO SAVEPOINT {savepoint_name}"),
            format!(
                "SELECT 'count_after_rollback' AS marker, count(*)::text AS value, '' AS detail\nFROM {table}\nWHERE {tenant_column} = {tenant_id}",
                tenant_id = self.tenant_id,
            ),
            format!("FETCH ALL FROM {cursor_name}"),
            "COMMIT".to_string(),
            format!(
                "SELECT 'final_count' AS marker, count(*)::text AS value, '' AS detail\nFROM {table}\nWHERE {tenant_column} = {tenant_id}",
                tenant_id = self.tenant_id,
            ),
            format!(
                "EXPLAIN (COSTS OFF) SELECT count(*) FROM {table} WHERE {tenant_column} = {tenant_id}",
                tenant_id = self.tenant_id,
            ),
        ];

        Ok(DistributedTransactionStateSqlPlan {
            feature_ids: FEATURE_IDS,
            statements,
            table_name: self.table_name.clone(),
            fetch_batch_rows: self.fetch_batch_rows,
        })
    }

    pub fn report(&self) -> Result<DistributedTransactionStateReport, TransactionStateError> {
        let sql_plan = self.to_sql_plan()?;
        let script = sql_plan.render_psql_script();
        Ok(DistributedTransactionStateReport {
            feature_ids: FEATURE_IDS,
            table_name: self.table_name.clone(),
            statement_count: sql_plan.statements.len(),
            cursor_declared: script.contains("DECLARE") && script.contains("NO SCROLL CURSOR"),
            cursor_fetches: script.matches("FETCH ").count(),
            savepoint_declared: script.contains("SAVEPOINT"),
            rollback_to_savepoint: script.contains("ROLLBACK TO SAVEPOINT"),
            citus_explain_required: script.contains("EXPLAIN (COSTS OFF)"),
            fetch_batch_rows: self.fetch_batch_rows,
            fail_closed_checks: canonical_transaction_state_fail_closed_checks(),
            coordinator_failover_exercised: false,
            multi_worker_cleanup_exercised: false,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DistributedTransactionStateSqlPlan {
    pub feature_ids: &'static [&'static str],
    pub statements: Vec<String>,
    pub table_name: String,
    pub fetch_batch_rows: u32,
}

impl DistributedTransactionStateSqlPlan {
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
pub struct DistributedTransactionStateReport {
    pub feature_ids: &'static [&'static str],
    pub table_name: String,
    pub statement_count: usize,
    pub cursor_declared: bool,
    pub cursor_fetches: usize,
    pub savepoint_declared: bool,
    pub rollback_to_savepoint: bool,
    pub citus_explain_required: bool,
    pub fetch_batch_rows: u32,
    pub fail_closed_checks: usize,
    pub coordinator_failover_exercised: bool,
    pub multi_worker_cleanup_exercised: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TransactionStateError {
    MissingRequiredField(&'static str),
    InvalidIdentifier { field: &'static str, value: String },
    InvalidPositive(&'static str),
}

impl fmt::Display for TransactionStateError {
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

impl Error for TransactionStateError {}

pub fn canonical_transaction_state_plan() -> DistributedTransactionStatePlan {
    DistributedTransactionStatePlan {
        table_name: DEFAULT_TABLE.to_string(),
        tenant_column: "tenant_id".to_string(),
        order_column: "order_id".to_string(),
        amount_column: "total".to_string(),
        tenant_id: 1,
        cursor_name: DEFAULT_CURSOR_NAME.to_string(),
        savepoint_name: DEFAULT_SAVEPOINT_NAME.to_string(),
        fetch_batch_rows: 2,
        sentinel_order_id: 99,
    }
}

pub fn canonical_transaction_state_sql_plan(
) -> Result<DistributedTransactionStateSqlPlan, TransactionStateError> {
    canonical_transaction_state_plan().to_sql_plan()
}

pub fn canonical_transaction_state_report(
) -> Result<DistributedTransactionStateReport, TransactionStateError> {
    canonical_transaction_state_plan().report()
}

pub fn canonical_transaction_state_fail_closed_checks() -> usize {
    let mut checks = 0;

    let mut missing_table = canonical_transaction_state_plan();
    missing_table.table_name.clear();
    if matches!(
        missing_table.validate(),
        Err(TransactionStateError::MissingRequiredField("table_name"))
    ) {
        checks += 1;
    }

    let mut unsafe_cursor = canonical_transaction_state_plan();
    unsafe_cursor.cursor_name = "cursor;drop".to_string();
    if matches!(
        unsafe_cursor.validate(),
        Err(TransactionStateError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut unsafe_savepoint = canonical_transaction_state_plan();
    unsafe_savepoint.savepoint_name = "save point".to_string();
    if matches!(
        unsafe_savepoint.validate(),
        Err(TransactionStateError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut zero_fetch = canonical_transaction_state_plan();
    zero_fetch.fetch_batch_rows = 0;
    if matches!(
        zero_fetch.validate(),
        Err(TransactionStateError::InvalidPositive("fetch_batch_rows"))
    ) {
        checks += 1;
    }

    let mut zero_tenant = canonical_transaction_state_plan();
    zero_tenant.tenant_id = 0;
    if matches!(
        zero_tenant.validate(),
        Err(TransactionStateError::InvalidPositive("tenant_id"))
    ) {
        checks += 1;
    }

    checks
}

fn quote_qualified_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, TransactionStateError> {
    validate_qualified_identifier(field, value)?;
    value
        .split('.')
        .map(|part| quote_identifier(field, part))
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("."))
}

fn validate_qualified_identifier(
    field: &'static str,
    value: &str,
) -> Result<(), TransactionStateError> {
    if value.trim().is_empty() {
        return Err(TransactionStateError::MissingRequiredField(field));
    }
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 2 {
        return Err(TransactionStateError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    for part in parts {
        validate_identifier(field, part)?;
    }
    Ok(())
}

fn quote_identifier(field: &'static str, value: &str) -> Result<String, TransactionStateError> {
    validate_identifier(field, value)?;
    Ok(format!("\"{value}\""))
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), TransactionStateError> {
    if value.trim().is_empty() {
        return Err(TransactionStateError::MissingRequiredField(field));
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
        return Err(TransactionStateError::InvalidIdentifier {
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
    fn canonical_report_covers_cursor_and_savepoint_boundaries() {
        let report = canonical_transaction_state_report().expect("report");

        assert_eq!(report.feature_ids, ["T13", "T14"]);
        assert_eq!(report.table_name, DEFAULT_TABLE);
        assert_eq!(report.statement_count, 12);
        assert!(report.cursor_declared);
        assert_eq!(report.cursor_fetches, 2);
        assert!(report.savepoint_declared);
        assert!(report.rollback_to_savepoint);
        assert!(report.citus_explain_required);
        assert_eq!(report.fetch_batch_rows, 2);
        assert_eq!(report.fail_closed_checks, 5);
        assert!(!report.coordinator_failover_exercised);
        assert!(!report.multi_worker_cleanup_exercised);
    }

    #[test]
    fn sql_plan_renders_transaction_cursor_savepoint_and_explain() {
        let sql_plan = canonical_transaction_state_sql_plan().expect("sql plan");
        let script = sql_plan.render_psql_script();

        assert!(script.contains("BEGIN;"));
        assert!(script.contains("DECLARE \"ai_blaise_txn_cursor\" NO SCROLL CURSOR"));
        assert!(script.contains("FETCH 2 FROM \"ai_blaise_txn_cursor\";"));
        assert!(script.contains("SAVEPOINT \"ai_blaise_before_extra\";"));
        assert!(script.contains("ROLLBACK TO SAVEPOINT \"ai_blaise_before_extra\";"));
        assert!(script.contains("SELECT 'count_after_rollback' AS marker"));
        assert!(script.contains("COMMIT;"));
        assert!(script.contains("EXPLAIN (COSTS OFF) SELECT count(*)"));
    }

    #[test]
    fn rejects_unsafe_cursor_name() {
        let mut plan = canonical_transaction_state_plan();
        plan.cursor_name = "cursor;drop".to_string();

        assert_eq!(
            plan.validate(),
            Err(TransactionStateError::InvalidIdentifier {
                field: "cursor_name",
                value: "cursor;drop".to_string(),
            })
        );
    }

    #[test]
    fn rejects_unsafe_savepoint_name() {
        let mut plan = canonical_transaction_state_plan();
        plan.savepoint_name = "save point".to_string();

        assert_eq!(
            plan.validate(),
            Err(TransactionStateError::InvalidIdentifier {
                field: "savepoint_name",
                value: "save point".to_string(),
            })
        );
    }

    #[test]
    fn rejects_zero_fetch_batch() {
        let mut plan = canonical_transaction_state_plan();
        plan.fetch_batch_rows = 0;

        assert_eq!(
            plan.validate(),
            Err(TransactionStateError::InvalidPositive("fetch_batch_rows"))
        );
    }
}
