// FEATURE: S8
// FEATURE: S12

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

const FEATURE_IDS: &[&str] = &["S8", "S12"];
const DEFAULT_LOCALITY_TABLE: &str = "public.locality_orders";
const DEFAULT_LOCALITY_COLUMN: &str = "locality_key";
const DEFAULT_TENANT_COLUMN: &str = "tenant_id";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionalPlacementPlan {
    pub locality_pk: LocalityPrefixedPrimaryKey,
    pub tablespaces: Vec<RegionTablespaceMapping>,
}

impl RegionalPlacementPlan {
    pub fn validate(&self) -> Result<(), RegionalPlacementError> {
        self.locality_pk.validate()?;
        if self.tablespaces.is_empty() {
            return Err(RegionalPlacementError::MissingRequiredField("tablespaces"));
        }

        let mut regions = BTreeSet::new();
        let mut tables = BTreeSet::new();
        for mapping in &self.tablespaces {
            mapping.validate()?;
            if !regions.insert(mapping.region.as_str()) {
                return Err(RegionalPlacementError::DuplicateRegion(
                    mapping.region.clone(),
                ));
            }
            if !tables.insert(mapping.table_name.as_str()) {
                return Err(RegionalPlacementError::DuplicateTable(
                    mapping.table_name.clone(),
                ));
            }
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<RegionalPlacementSqlPlan, RegionalPlacementError> {
        self.validate()?;
        let locality_table_literal = sql_literal(&self.locality_pk.table_name);
        let locality_column_literal = sql_literal(&self.locality_pk.locality_column);
        let tenant_column_literal = sql_literal(&self.locality_pk.tenant_column);
        let tablespace_values = self
            .tablespaces
            .iter()
            .map(|mapping| {
                Ok(format!(
                    "('{}', '{}', '{}')",
                    sql_literal(&mapping.region),
                    sql_literal(&mapping.table_name),
                    sql_literal(&mapping.tablespace_name)
                ))
            })
            .collect::<Result<Vec<_>, RegionalPlacementError>>()?
            .join(",\n  ");

        let query = format!(
            "WITH locality_pk AS (\n  SELECT array_agg(a.attname ORDER BY ord.ordinality) AS pk_columns\n  FROM pg_index i\n  CROSS JOIN LATERAL unnest(i.indkey) WITH ORDINALITY AS ord(attnum, ordinality)\n  JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ord.attnum\n  WHERE i.indrelid = '{locality_table}'::regclass\n    AND i.indisprimary\n), citus_distribution AS (\n  SELECT\n    count(*) > 0 AS distributed,\n    COALESCE(bool_or(partkey LIKE '%:varattno 1%'), false) AS distribution_first_column\n  FROM pg_dist_partition\n  WHERE logicalrelid = '{locality_table}'::regclass\n), expected_tablespaces(region_name, table_name, tablespace_name) AS (\n  VALUES\n  {tablespace_values}\n), observed_tablespaces AS (\n  SELECT\n    e.region_name,\n    e.table_name,\n    e.tablespace_name,\n    spc.spcname AS observed_tablespace,\n    spc.spcname = e.tablespace_name AS tablespace_matches\n  FROM expected_tablespaces e\n  JOIN pg_class c ON c.oid = e.table_name::regclass\n  JOIN pg_tablespace spc ON spc.oid = c.reltablespace\n)\nSELECT\n  'S8,S12' AS feature_ids,\n  '{locality_table}' AS locality_table,\n  COALESCE((SELECT pk_columns[1:2]::text[] = ARRAY['{locality_column}', '{tenant_column}'] FROM locality_pk), false) AS locality_prefix_valid,\n  2 AS pk_prefix_columns,\n  COALESCE((SELECT distributed AND distribution_first_column FROM citus_distribution), false) AS citus_distribution_present,\n  (SELECT count(*) FROM observed_tablespaces) AS region_tablespace_count,\n  COALESCE((SELECT bool_and(tablespace_matches) FROM observed_tablespaces), false) AS region_tablespaces_valid,\n  false AS automatic_rebalance_executed,\n  false AS shard_movement_executed",
            locality_table = locality_table_literal,
            locality_column = locality_column_literal,
            tenant_column = tenant_column_literal,
            tablespace_values = tablespace_values,
        );

        Ok(RegionalPlacementSqlPlan {
            feature_ids: FEATURE_IDS,
            statements: vec![query],
            locality_table: self.locality_pk.table_name.clone(),
            region_tablespace_count: self.tablespaces.len(),
        })
    }

    pub fn report(&self) -> Result<RegionalPlacementReport, RegionalPlacementError> {
        let sql_plan = self.to_sql_plan()?;
        Ok(RegionalPlacementReport {
            feature_ids: FEATURE_IDS,
            locality_table: self.locality_pk.table_name.clone(),
            locality_column: self.locality_pk.locality_column.clone(),
            tenant_column: self.locality_pk.tenant_column.clone(),
            pk_prefix_columns: 2,
            region_tablespace_count: self.tablespaces.len(),
            statement_count: sql_plan.statements.len(),
            catalog_tables: sql_plan.catalog_tables(),
            read_only_sql: !sql_plan.contains_mutating_statement(),
            fail_closed_checks: canonical_regional_placement_fail_closed_checks(),
            automatic_rebalance_executed: false,
            shard_movement_executed: false,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LocalityPrefixedPrimaryKey {
    pub table_name: String,
    pub locality_column: String,
    pub tenant_column: String,
    pub distribution_column: String,
}

impl LocalityPrefixedPrimaryKey {
    fn validate(&self) -> Result<(), RegionalPlacementError> {
        validate_qualified_identifier("locality_pk.table_name", &self.table_name)?;
        validate_identifier("locality_pk.locality_column", &self.locality_column)?;
        validate_identifier("locality_pk.tenant_column", &self.tenant_column)?;
        validate_identifier("locality_pk.distribution_column", &self.distribution_column)?;
        if self.locality_column == self.tenant_column {
            return Err(RegionalPlacementError::DuplicateColumn(
                self.locality_column.clone(),
            ));
        }
        if self.distribution_column != self.locality_column {
            return Err(RegionalPlacementError::DistributionMustUseLocalityColumn);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionTablespaceMapping {
    pub region: String,
    pub table_name: String,
    pub tablespace_name: String,
}

impl RegionTablespaceMapping {
    pub fn new(region: &str, table_name: &str, tablespace_name: &str) -> Self {
        Self {
            region: region.to_string(),
            table_name: table_name.to_string(),
            tablespace_name: tablespace_name.to_string(),
        }
    }

    fn validate(&self) -> Result<(), RegionalPlacementError> {
        validate_region("tablespace.region", &self.region)?;
        validate_qualified_identifier("tablespace.table_name", &self.table_name)?;
        validate_identifier("tablespace.tablespace_name", &self.tablespace_name)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionalPlacementSqlPlan {
    pub feature_ids: &'static [&'static str],
    pub statements: Vec<String>,
    pub locality_table: String,
    pub region_tablespace_count: usize,
}

impl RegionalPlacementSqlPlan {
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

    pub fn contains_mutating_statement(&self) -> bool {
        let script = self.render_psql_script().to_ascii_uppercase();
        [
            "INSERT ",
            "UPDATE ",
            "DELETE ",
            "ALTER ",
            "DROP ",
            "CREATE ",
            "TRUNCATE ",
        ]
        .iter()
        .any(|needle| script.contains(needle))
    }

    pub fn catalog_tables(&self) -> Vec<&'static str> {
        let script = self.render_psql_script();
        [
            "pg_index",
            "pg_attribute",
            "pg_dist_partition",
            "pg_class",
            "pg_tablespace",
        ]
        .iter()
        .copied()
        .filter(|table| script.contains(table))
        .collect()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionalPlacementReport {
    pub feature_ids: &'static [&'static str],
    pub locality_table: String,
    pub locality_column: String,
    pub tenant_column: String,
    pub pk_prefix_columns: usize,
    pub region_tablespace_count: usize,
    pub statement_count: usize,
    pub catalog_tables: Vec<&'static str>,
    pub read_only_sql: bool,
    pub fail_closed_checks: usize,
    pub automatic_rebalance_executed: bool,
    pub shard_movement_executed: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RegionalPlacementError {
    MissingRequiredField(&'static str),
    InvalidIdentifier { field: &'static str, value: String },
    InvalidRegion(String),
    DuplicateColumn(String),
    DuplicateRegion(String),
    DuplicateTable(String),
    DistributionMustUseLocalityColumn,
}

impl fmt::Display for RegionalPlacementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(formatter, "{field} is required"),
            Self::InvalidIdentifier { field, value } => {
                write!(
                    formatter,
                    "{field} is not a safe PostgreSQL identifier: {value}"
                )
            }
            Self::InvalidRegion(region) => write!(formatter, "invalid region: {region}"),
            Self::DuplicateColumn(column) => {
                write!(formatter, "duplicate locality column: {column}")
            }
            Self::DuplicateRegion(region) => {
                write!(formatter, "duplicate region mapping: {region}")
            }
            Self::DuplicateTable(table) => {
                write!(formatter, "duplicate regional table mapping: {table}")
            }
            Self::DistributionMustUseLocalityColumn => write!(
                formatter,
                "distribution_column must equal the locality-prefixed primary-key column"
            ),
        }
    }
}

impl Error for RegionalPlacementError {}

pub fn canonical_regional_placement_plan() -> RegionalPlacementPlan {
    RegionalPlacementPlan {
        locality_pk: LocalityPrefixedPrimaryKey {
            table_name: DEFAULT_LOCALITY_TABLE.to_string(),
            locality_column: DEFAULT_LOCALITY_COLUMN.to_string(),
            tenant_column: DEFAULT_TENANT_COLUMN.to_string(),
            distribution_column: DEFAULT_LOCALITY_COLUMN.to_string(),
        },
        tablespaces: vec![
            RegionTablespaceMapping::new(
                "us-east-1",
                "public.locality_orders",
                "ai_blaise_us_east_1",
            ),
            RegionTablespaceMapping::new(
                "eu-west-1",
                "public.locality_orders_eu",
                "ai_blaise_eu_west_1",
            ),
        ],
    }
}

pub fn canonical_regional_placement_sql_plan(
) -> Result<RegionalPlacementSqlPlan, RegionalPlacementError> {
    canonical_regional_placement_plan().to_sql_plan()
}

pub fn canonical_regional_placement_report(
) -> Result<RegionalPlacementReport, RegionalPlacementError> {
    canonical_regional_placement_plan().report()
}

pub fn canonical_regional_placement_fail_closed_checks() -> usize {
    let mut checks = 0;

    let mut missing_table = canonical_regional_placement_plan();
    missing_table.locality_pk.table_name.clear();
    if matches!(
        missing_table.validate(),
        Err(RegionalPlacementError::MissingRequiredField(
            "locality_pk.table_name"
        ))
    ) {
        checks += 1;
    }

    let mut unsafe_column = canonical_regional_placement_plan();
    unsafe_column.locality_pk.locality_column = "locality-key".to_string();
    if matches!(
        unsafe_column.validate(),
        Err(RegionalPlacementError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut mismatch = canonical_regional_placement_plan();
    mismatch.locality_pk.distribution_column = "tenant_id".to_string();
    if matches!(
        mismatch.validate(),
        Err(RegionalPlacementError::DistributionMustUseLocalityColumn)
    ) {
        checks += 1;
    }

    let mut duplicate_region = canonical_regional_placement_plan();
    duplicate_region.tablespaces[1].region = duplicate_region.tablespaces[0].region.clone();
    if matches!(
        duplicate_region.validate(),
        Err(RegionalPlacementError::DuplicateRegion(_))
    ) {
        checks += 1;
    }

    let mut unsafe_tablespace = canonical_regional_placement_plan();
    unsafe_tablespace.tablespaces[0].tablespace_name = "bad tablespace".to_string();
    if matches!(
        unsafe_tablespace.validate(),
        Err(RegionalPlacementError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut empty_tablespaces = canonical_regional_placement_plan();
    empty_tablespaces.tablespaces.clear();
    if matches!(
        empty_tablespaces.validate(),
        Err(RegionalPlacementError::MissingRequiredField("tablespaces"))
    ) {
        checks += 1;
    }

    checks
}

fn validate_qualified_identifier(
    field: &'static str,
    value: &str,
) -> Result<(), RegionalPlacementError> {
    if value.trim().is_empty() {
        return Err(RegionalPlacementError::MissingRequiredField(field));
    }
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 2 {
        return Err(RegionalPlacementError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    for part in parts {
        validate_identifier(field, part)?;
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), RegionalPlacementError> {
    if value.trim().is_empty() {
        return Err(RegionalPlacementError::MissingRequiredField(field));
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
        return Err(RegionalPlacementError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn validate_region(field: &'static str, value: &str) -> Result<(), RegionalPlacementError> {
    if value.trim().is_empty() {
        return Err(RegionalPlacementError::MissingRequiredField(field));
    }
    if value.len() > 63
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || character == '-'))
    {
        return Err(RegionalPlacementError::InvalidRegion(value.to_string()));
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
    fn canonical_report_is_read_only_and_catalog_backed() {
        let report = canonical_regional_placement_report().expect("report");

        assert_eq!(report.feature_ids, ["S8", "S12"]);
        assert_eq!(report.locality_table, "public.locality_orders");
        assert_eq!(report.locality_column, "locality_key");
        assert_eq!(report.tenant_column, "tenant_id");
        assert_eq!(report.pk_prefix_columns, 2);
        assert_eq!(report.region_tablespace_count, 2);
        assert_eq!(report.statement_count, 1);
        assert_eq!(report.fail_closed_checks, 6);
        assert!(report.read_only_sql);
        assert!(!report.automatic_rebalance_executed);
        assert!(!report.shard_movement_executed);
        assert_eq!(
            report.catalog_tables,
            vec![
                "pg_index",
                "pg_attribute",
                "pg_dist_partition",
                "pg_class",
                "pg_tablespace"
            ]
        );
    }

    #[test]
    fn sql_checks_primary_key_distribution_and_tablespaces() {
        let sql_plan = canonical_regional_placement_sql_plan().expect("sql plan");
        let script = sql_plan.render_psql_script();

        assert!(script.contains("FROM pg_index i"));
        assert!(script.contains("JOIN pg_attribute a"));
        assert!(script.contains("FROM pg_dist_partition"));
        assert!(script.contains("partkey LIKE '%:varattno 1%'"));
        assert!(script.contains("JOIN pg_tablespace spc"));
        assert!(script.contains("ai_blaise_us_east_1"));
        assert!(script.contains("ai_blaise_eu_west_1"));
        assert!(script.contains("automatic_rebalance_executed"));
        assert!(script.contains("shard_movement_executed"));
        assert!(!sql_plan.contains_mutating_statement());
    }

    #[test]
    fn rejects_distribution_column_that_is_not_locality_prefix() {
        let mut plan = canonical_regional_placement_plan();
        plan.locality_pk.distribution_column = "tenant_id".to_string();

        assert_eq!(
            plan.validate(),
            Err(RegionalPlacementError::DistributionMustUseLocalityColumn)
        );
    }

    #[test]
    fn rejects_duplicate_region_mappings() {
        let mut plan = canonical_regional_placement_plan();
        plan.tablespaces[1].region = plan.tablespaces[0].region.clone();

        assert_eq!(
            plan.validate(),
            Err(RegionalPlacementError::DuplicateRegion(
                "us-east-1".to_string()
            ))
        );
    }

    #[test]
    fn rejects_unsafe_tablespace_name() {
        let mut plan = canonical_regional_placement_plan();
        plan.tablespaces[0].tablespace_name = "bad tablespace".to_string();

        assert_eq!(
            plan.validate(),
            Err(RegionalPlacementError::InvalidIdentifier {
                field: "tablespace.tablespace_name",
                value: "bad tablespace".to_string(),
            })
        );
    }
}
