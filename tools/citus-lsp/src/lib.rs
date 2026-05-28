//! Citus-aware language-server diagnostic contracts.

// FEATURE: D4
// FEATURE: M5
// FEATURE: TS8

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CitusLspPlan {
    pub metadata: LspMetadataSnapshot,
    pub rules: Vec<LspRule>,
}

impl CitusLspPlan {
    pub fn new(metadata: LspMetadataSnapshot, rules: Vec<LspRule>) -> Result<Self, CitusLspError> {
        metadata.validate()?;
        if rules.is_empty() {
            return Err(CitusLspError::MissingRequiredField("rules"));
        }

        Ok(Self { metadata, rules })
    }

    pub fn analyze(&self, request: &SqlAnalysisRequest) -> Result<LspAnalysis, CitusLspError> {
        self.metadata.validate()?;
        request.validate()?;

        let mut diagnostics = Vec::new();
        for intent in &request.intents {
            match intent {
                SqlIntent::CreateTable {
                    table,
                    columns,
                    distribution_column,
                    tenant_column,
                } if self.rule_enabled(LspRule::MissingDistributionColumnQuickFix)
                    && distribution_column
                        .as_deref()
                        .unwrap_or("")
                        .trim()
                        .is_empty() =>
                {
                    diagnostics.push(missing_distribution_column_diagnostic(
                        table,
                        columns,
                        tenant_column.as_deref(),
                    )?);
                }
                SqlIntent::Join {
                    left_table,
                    right_table,
                } if self.rule_enabled(LspRule::NonColocatedJoin) => {
                    if let (Some(left), Some(right)) = (
                        self.metadata.distributed_table(left_table),
                        self.metadata.distributed_table(right_table),
                    ) {
                        if left.colocation_group != right.colocation_group {
                            diagnostics.push(LspDiagnostic {
                                code: LspDiagnosticCode::NonColocatedJoin,
                                severity: DiagnosticSeverity::Warning,
                                message: format!(
                                    "{} and {} are distributed in different colocation groups",
                                    left.table, right.table
                                ),
                                quick_fix: Some(LspQuickFix {
                                    title: "Align distribution columns".to_string(),
                                    action: LspQuickFixAction::AlignColocation {
                                        left_table: left.table.clone(),
                                        right_table: right.table.clone(),
                                        distribution_column: left.distribution_column.clone(),
                                    },
                                }),
                            });
                        }
                    }
                }
                SqlIntent::AlterColumn {
                    table,
                    column,
                    action,
                } if self.rule_enabled(LspRule::DistributionColumnAlter) => {
                    if let Some(metadata) = self.metadata.distributed_table(table) {
                        if metadata.distribution_column == *column && action.is_unsafe() {
                            diagnostics.push(LspDiagnostic {
                                code: LspDiagnosticCode::DistributionColumnAlter,
                                severity: DiagnosticSeverity::Error,
                                message: format!(
                                    "cannot {:?} distribution column {} on distributed table {}",
                                    action, column, table
                                ),
                                quick_fix: None,
                            });
                        }
                    }
                }
                SqlIntent::CreateHypertable {
                    table,
                    time_column,
                    uses_distributed_bridge,
                } if self.rule_enabled(LspRule::HypertableInvariant) => {
                    if time_column.as_deref().unwrap_or("").trim().is_empty() {
                        diagnostics.push(LspDiagnostic {
                            code: LspDiagnosticCode::HypertableInvariant,
                            severity: DiagnosticSeverity::Error,
                            message: format!("hypertable {} must declare a time column", table),
                            quick_fix: None,
                        });
                    }

                    if self.metadata.distributed_table(table).is_some() && !uses_distributed_bridge
                    {
                        diagnostics.push(LspDiagnostic {
                            code: LspDiagnosticCode::HypertableInvariant,
                            severity: DiagnosticSeverity::Warning,
                            message: format!(
                                "distributed hypertable {} must use the companion bridge",
                                table
                            ),
                            quick_fix: time_column.as_ref().map(|time_column| LspQuickFix {
                                title: "Use distributed hypertable bridge".to_string(),
                                action: LspQuickFixAction::UseDistributedHypertableBridge {
                                    table: table.clone(),
                                    time_column: time_column.clone(),
                                },
                            }),
                        });
                    }
                }
                SqlIntent::Select {
                    table,
                    where_columns,
                } if self.rule_enabled(LspRule::MissingTenantFilter) => {
                    if let Some(metadata) = self.metadata.distributed_table(table) {
                        if let Some(tenant_column) = &metadata.tenant_column {
                            if !where_columns.iter().any(|column| column == tenant_column) {
                                diagnostics.push(LspDiagnostic {
                                    code: LspDiagnosticCode::MissingTenantFilter,
                                    severity: DiagnosticSeverity::Warning,
                                    message: format!(
                                        "query on {} should filter tenant column {}",
                                        table, tenant_column
                                    ),
                                    quick_fix: Some(LspQuickFix {
                                        title: "Add tenant filter".to_string(),
                                        action: LspQuickFixAction::AddTenantFilter {
                                            table: table.clone(),
                                            tenant_column: tenant_column.clone(),
                                        },
                                    }),
                                });
                            }
                        }
                    }
                }
                SqlIntent::CreateSearchIndex {
                    index_name,
                    table,
                    analyzer,
                } if self.rule_enabled(LspRule::MissingSearchAnalyzer)
                    && analyzer.as_deref().unwrap_or("").trim().is_empty() =>
                {
                    diagnostics.push(LspDiagnostic {
                        code: LspDiagnosticCode::MissingSearchAnalyzer,
                        severity: DiagnosticSeverity::Warning,
                        message: format!(
                            "search index {} on {} should declare an analyzer",
                            index_name, table
                        ),
                        quick_fix: Some(LspQuickFix {
                            title: "Use default analyzer".to_string(),
                            action: LspQuickFixAction::SetSearchAnalyzer {
                                index_name: index_name.clone(),
                                analyzer: "english".to_string(),
                            },
                        }),
                    });
                }
                _ => {}
            }
        }

        Ok(LspAnalysis {
            uri: request.uri.clone(),
            diagnostics,
        })
    }

    fn rule_enabled(&self, rule: LspRule) -> bool {
        self.rules.contains(&rule)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LspMetadataSnapshot {
    pub distributed_tables: Vec<DistributedTableMetadata>,
    pub hypertables: Vec<HypertableMetadata>,
    pub search_indexes: Vec<SearchIndexMetadata>,
    pub tenants: Vec<TenantMetadata>,
}

impl LspMetadataSnapshot {
    pub fn validate(&self) -> Result<(), CitusLspError> {
        for table in &self.distributed_tables {
            table.validate()?;
        }
        for hypertable in &self.hypertables {
            hypertable.validate()?;
        }
        for index in &self.search_indexes {
            index.validate()?;
        }
        for tenant in &self.tenants {
            tenant.validate()?;
        }
        Ok(())
    }

    fn distributed_table(&self, table: &str) -> Option<&DistributedTableMetadata> {
        self.distributed_tables
            .iter()
            .find(|metadata| metadata.table == table)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DistributedTableMetadata {
    pub table: String,
    pub distribution_column: String,
    pub colocation_group: String,
    pub tenant_column: Option<String>,
}

impl DistributedTableMetadata {
    fn validate(&self) -> Result<(), CitusLspError> {
        validate_required("distributed_tables.table", &self.table)?;
        validate_required(
            "distributed_tables.distribution_column",
            &self.distribution_column,
        )?;
        validate_required(
            "distributed_tables.colocation_group",
            &self.colocation_group,
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HypertableMetadata {
    pub table: String,
    pub time_column: String,
    pub distributed_parent: Option<String>,
}

impl HypertableMetadata {
    fn validate(&self) -> Result<(), CitusLspError> {
        validate_required("hypertables.table", &self.table)?;
        validate_required("hypertables.time_column", &self.time_column)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchIndexMetadata {
    pub index_name: String,
    pub table: String,
    pub analyzer: Option<String>,
}

impl SearchIndexMetadata {
    fn validate(&self) -> Result<(), CitusLspError> {
        validate_required("search_indexes.index_name", &self.index_name)?;
        validate_required("search_indexes.table", &self.table)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantMetadata {
    pub tenant_id: String,
    pub schema: String,
}

impl TenantMetadata {
    fn validate(&self) -> Result<(), CitusLspError> {
        validate_required("tenants.tenant_id", &self.tenant_id)?;
        validate_required("tenants.schema", &self.schema)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LspRule {
    NonColocatedJoin,
    DistributionColumnAlter,
    HypertableInvariant,
    MissingTenantFilter,
    MissingSearchAnalyzer,
    MissingDistributionColumnQuickFix,
}

pub fn all_lsp_rules() -> Vec<LspRule> {
    vec![
        LspRule::NonColocatedJoin,
        LspRule::DistributionColumnAlter,
        LspRule::HypertableInvariant,
        LspRule::MissingTenantFilter,
        LspRule::MissingSearchAnalyzer,
        LspRule::MissingDistributionColumnQuickFix,
    ]
}

pub fn canonical_lsp_plan() -> Result<CitusLspPlan, CitusLspError> {
    CitusLspPlan::new(
        LspMetadataSnapshot {
            distributed_tables: vec![
                DistributedTableMetadata {
                    table: "public.orders".to_string(),
                    distribution_column: "tenant_id".to_string(),
                    colocation_group: "tenant".to_string(),
                    tenant_column: Some("tenant_id".to_string()),
                },
                DistributedTableMetadata {
                    table: "public.line_items".to_string(),
                    distribution_column: "tenant_id".to_string(),
                    colocation_group: "tenant".to_string(),
                    tenant_column: Some("tenant_id".to_string()),
                },
                DistributedTableMetadata {
                    table: "public.events".to_string(),
                    distribution_column: "device_id".to_string(),
                    colocation_group: "device".to_string(),
                    tenant_column: None,
                },
            ],
            hypertables: vec![HypertableMetadata {
                table: "public.events".to_string(),
                time_column: "created_at".to_string(),
                distributed_parent: Some("public.events".to_string()),
            }],
            search_indexes: Vec::new(),
            tenants: vec![TenantMetadata {
                tenant_id: "tenant-a".to_string(),
                schema: "tenant_a".to_string(),
            }],
        },
        all_lsp_rules(),
    )
}

pub fn canonical_analysis_request() -> SqlAnalysisRequest {
    SqlAnalysisRequest {
        uri: "file:///workspace/canonical.sql".to_string(),
        intents: vec![
            SqlIntent::CreateTable {
                table: "tenant_a.invoices".to_string(),
                columns: vec!["tenant_id".to_string(), "invoice_id".to_string()],
                distribution_column: None,
                tenant_column: Some("tenant_id".to_string()),
            },
            SqlIntent::Join {
                left_table: "public.orders".to_string(),
                right_table: "public.events".to_string(),
            },
            SqlIntent::AlterColumn {
                table: "public.orders".to_string(),
                column: "tenant_id".to_string(),
                action: AlterColumnAction::Drop,
            },
            SqlIntent::CreateHypertable {
                table: "public.events".to_string(),
                time_column: Some("created_at".to_string()),
                uses_distributed_bridge: false,
            },
            SqlIntent::Select {
                table: "public.orders".to_string(),
                where_columns: vec!["status".to_string()],
            },
            SqlIntent::CreateSearchIndex {
                index_name: "orders_search".to_string(),
                table: "public.orders".to_string(),
                analyzer: None,
            },
        ],
    }
}

pub fn parse_metadata_tsv(input: &str) -> Result<LspMetadataSnapshot, CitusLspError> {
    let mut distributed_tables = Vec::new();
    let mut hypertables = Vec::new();
    let mut search_indexes = Vec::new();
    let mut tenants = Vec::new();

    for (line_index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let columns = line.split('\t').collect::<Vec<_>>();
        match columns.as_slice() {
            ["distributed_table", table, distribution_column, colocation_group, tenant_column] => {
                distributed_tables.push(DistributedTableMetadata {
                    table: normalize_identifier(table),
                    distribution_column: normalize_identifier(distribution_column),
                    colocation_group: colocation_group.trim().to_string(),
                    tenant_column: optional_field(tenant_column).map(normalize_identifier),
                });
            }
            ["hypertable", table, time_column, distributed_parent] => {
                hypertables.push(HypertableMetadata {
                    table: normalize_identifier(table),
                    time_column: normalize_identifier(time_column),
                    distributed_parent: optional_field(distributed_parent)
                        .map(normalize_identifier),
                });
            }
            ["search_index", index_name, table, analyzer] => {
                search_indexes.push(SearchIndexMetadata {
                    index_name: normalize_identifier(index_name),
                    table: normalize_identifier(table),
                    analyzer: optional_field(analyzer).map(str::to_string),
                });
            }
            ["tenant", tenant_id, schema] => {
                tenants.push(TenantMetadata {
                    tenant_id: tenant_id.trim().to_string(),
                    schema: normalize_identifier(schema),
                });
            }
            [kind, ..] => {
                return Err(CitusLspError::InvalidMetadataLine {
                    line: line_index + 1,
                    detail: format!("unknown metadata kind or wrong column count: {kind}"),
                });
            }
            [] => {}
        }
    }

    let snapshot = LspMetadataSnapshot {
        distributed_tables,
        hypertables,
        search_indexes,
        tenants,
    };
    snapshot.validate()?;
    Ok(snapshot)
}

pub fn parse_sql_document(
    uri: impl Into<String>,
    sql: &str,
) -> Result<SqlAnalysisRequest, CitusLspError> {
    validate_required("sql", sql)?;

    let statements = split_sql_statements(sql);
    if statements.is_empty() {
        return Err(CitusLspError::MissingRequiredField("sql_statements"));
    }

    let mut distributed_columns = BTreeMap::new();
    let mut bridge_tables = BTreeSet::new();
    for statement in &statements {
        if let Some((table, column)) =
            function_two_string_args(statement, "create_distributed_table")
        {
            distributed_columns.insert(table, column);
        }
        if let Some((table, distribution_column)) =
            function_two_string_args(statement, "apply_distribute_hypertable")
        {
            distributed_columns.insert(table.clone(), distribution_column);
            bridge_tables.insert(table);
        }
    }

    let mut intents = Vec::new();
    for statement in &statements {
        if let Some(intent) = parse_create_table(statement, &distributed_columns) {
            intents.push(intent);
        }
        if let Some(intent) = parse_join(statement) {
            intents.push(intent);
        }
        if let Some(intent) = parse_alter_table(statement) {
            intents.push(intent);
        }
        if let Some(intent) = parse_create_hypertable(statement, &bridge_tables) {
            intents.push(intent);
        }
        if let Some(intent) = parse_select(statement) {
            intents.push(intent);
        }
        if let Some(intent) = parse_search_index(statement) {
            intents.push(intent);
        }
    }

    SqlAnalysisRequest {
        uri: uri.into(),
        intents,
    }
    .tap_validate()
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SqlAnalysisRequest {
    pub uri: String,
    pub intents: Vec<SqlIntent>,
}

impl SqlAnalysisRequest {
    fn validate(&self) -> Result<(), CitusLspError> {
        validate_required("uri", &self.uri)?;
        if self.intents.is_empty() {
            return Err(CitusLspError::MissingRequiredField("intents"));
        }
        for intent in &self.intents {
            intent.validate()?;
        }
        Ok(())
    }

    fn tap_validate(self) -> Result<Self, CitusLspError> {
        self.validate()?;
        Ok(self)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SqlIntent {
    CreateTable {
        table: String,
        columns: Vec<String>,
        distribution_column: Option<String>,
        tenant_column: Option<String>,
    },
    Join {
        left_table: String,
        right_table: String,
    },
    AlterColumn {
        table: String,
        column: String,
        action: AlterColumnAction,
    },
    CreateHypertable {
        table: String,
        time_column: Option<String>,
        uses_distributed_bridge: bool,
    },
    Select {
        table: String,
        where_columns: Vec<String>,
    },
    CreateSearchIndex {
        index_name: String,
        table: String,
        analyzer: Option<String>,
    },
}

impl SqlIntent {
    fn validate(&self) -> Result<(), CitusLspError> {
        match self {
            Self::CreateTable { table, columns, .. } => {
                validate_required("table", table)?;
                validate_required_list("columns", columns)
            }
            Self::Join {
                left_table,
                right_table,
            } => {
                validate_required("left_table", left_table)?;
                validate_required("right_table", right_table)
            }
            Self::AlterColumn { table, column, .. } => {
                validate_required("table", table)?;
                validate_required("column", column)
            }
            Self::CreateHypertable { table, .. } => validate_required("table", table),
            Self::Select { table, .. } => validate_required("table", table),
            Self::CreateSearchIndex {
                index_name, table, ..
            } => {
                validate_required("index_name", index_name)?;
                validate_required("table", table)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AlterColumnAction {
    Drop,
    Rename,
    AlterType,
    SetNotNull,
}

impl AlterColumnAction {
    fn is_unsafe(self) -> bool {
        matches!(self, Self::Drop | Self::AlterType)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LspAnalysis {
    pub uri: String,
    pub diagnostics: Vec<LspDiagnostic>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LspDiagnostic {
    pub code: LspDiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub quick_fix: Option<LspQuickFix>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LspDiagnosticCode {
    NonColocatedJoin,
    DistributionColumnAlter,
    HypertableInvariant,
    MissingTenantFilter,
    MissingSearchAnalyzer,
    MissingDistributionColumn,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LspQuickFix {
    pub title: String,
    pub action: LspQuickFixAction,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LspQuickFixAction {
    AddDistributionColumn {
        table: String,
        column: String,
    },
    AlignColocation {
        left_table: String,
        right_table: String,
        distribution_column: String,
    },
    UseDistributedHypertableBridge {
        table: String,
        time_column: String,
    },
    AddTenantFilter {
        table: String,
        tenant_column: String,
    },
    SetSearchAnalyzer {
        index_name: String,
        analyzer: String,
    },
}

fn parse_create_table(
    statement: &str,
    distributed_columns: &BTreeMap<String, String>,
) -> Option<SqlIntent> {
    let lower = statement.to_ascii_lowercase();
    if !lower.starts_with("create table ")
        && !lower.starts_with("create unlogged table ")
        && !lower.starts_with("create temporary table ")
        && !lower.starts_with("create temp table ")
    {
        return None;
    }

    let table_keyword = lower.find(" table ")?;
    let mut after_table = statement[table_keyword + " table ".len()..].trim_start();
    if after_table
        .to_ascii_lowercase()
        .starts_with("if not exists ")
    {
        after_table = after_table["if not exists ".len()..].trim_start();
    }

    let open = after_table.find('(')?;
    let table = normalize_identifier(&after_table[..open]);
    let column_block = matching_parenthesized(after_table, open)?;
    let columns = split_top_level_commas(column_block)
        .into_iter()
        .filter_map(|column_def| first_identifier(&column_def))
        .filter(|column| {
            let lower_column = column.to_ascii_lowercase();
            !matches!(
                lower_column.as_str(),
                "constraint" | "primary" | "foreign" | "unique" | "check" | "exclude"
            )
        })
        .collect::<Vec<_>>();
    if columns.is_empty() {
        return None;
    }

    let tenant_column = columns
        .iter()
        .find(|column| column.eq_ignore_ascii_case("tenant_id"))
        .cloned();
    let distribution_column = distributed_columns
        .get(&table)
        .cloned()
        .or_else(|| distributed_columns.get(&unqualified_table(&table)).cloned());

    Some(SqlIntent::CreateTable {
        table,
        columns,
        distribution_column,
        tenant_column,
    })
}

fn parse_join(statement: &str) -> Option<SqlIntent> {
    let from_table = table_after_keyword(statement, "from")?;
    let right_table = table_after_keyword(statement, "join")?;
    Some(SqlIntent::Join {
        left_table: from_table,
        right_table,
    })
}

fn parse_alter_table(statement: &str) -> Option<SqlIntent> {
    let lower = statement.to_ascii_lowercase();
    if !lower.starts_with("alter table ") {
        return None;
    }

    let after_table = statement["alter table ".len()..].trim_start();
    let (table, remainder) = take_identifier_with_remainder(after_table)?;
    let remainder_lower = remainder.to_ascii_lowercase();

    if let Some(column) = identifier_after_phrase(remainder, "drop column") {
        return Some(SqlIntent::AlterColumn {
            table,
            column,
            action: AlterColumnAction::Drop,
        });
    }
    if let Some(column) = identifier_after_phrase(remainder, "alter column") {
        if remainder_lower.contains(" type ") {
            return Some(SqlIntent::AlterColumn {
                table,
                column,
                action: AlterColumnAction::AlterType,
            });
        }
        if remainder_lower.contains(" set not null") {
            return Some(SqlIntent::AlterColumn {
                table,
                column,
                action: AlterColumnAction::SetNotNull,
            });
        }
    }
    if let Some(column) = identifier_after_phrase(remainder, "rename column") {
        return Some(SqlIntent::AlterColumn {
            table,
            column,
            action: AlterColumnAction::Rename,
        });
    }

    None
}

fn parse_create_hypertable(statement: &str, bridge_tables: &BTreeSet<String>) -> Option<SqlIntent> {
    let (table, time_column) = function_two_string_args(statement, "create_hypertable")?;
    let uses_distributed_bridge =
        bridge_tables.contains(&table) || bridge_tables.contains(&unqualified_table(&table));
    Some(SqlIntent::CreateHypertable {
        table,
        time_column: Some(time_column),
        uses_distributed_bridge,
    })
}

fn parse_select(statement: &str) -> Option<SqlIntent> {
    let lower = statement.trim_start().to_ascii_lowercase();
    if !lower.starts_with("select ") && !lower.starts_with("with ") {
        return None;
    }
    let table = table_after_keyword(statement, "from")?;
    let where_columns = where_columns(statement);
    Some(SqlIntent::Select {
        table,
        where_columns,
    })
}

fn parse_search_index(statement: &str) -> Option<SqlIntent> {
    let lower = statement.to_ascii_lowercase();
    if !lower.starts_with("create index ") && !lower.starts_with("create unique index ") {
        return None;
    }
    if !lower.contains(" using bm25")
        && !lower.contains(" using hnsw")
        && !lower.contains(" using ivfflat")
    {
        return None;
    }

    let index_offset = if lower.starts_with("create unique index ") {
        "create unique index ".len()
    } else {
        "create index ".len()
    };
    let mut after_index = statement[index_offset..].trim_start();
    if after_index
        .to_ascii_lowercase()
        .starts_with("if not exists ")
    {
        after_index = after_index["if not exists ".len()..].trim_start();
    }
    let (index_name, _) = take_identifier_with_remainder(after_index)?;
    let table = table_after_keyword(statement, "on")?;
    let analyzer = if lower.contains("analyzer") {
        Some("configured".to_string())
    } else {
        None
    };

    Some(SqlIntent::CreateSearchIndex {
        index_name,
        table,
        analyzer,
    })
}

fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut chars = sql.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '-' && chars.peek() == Some(&'-') && !in_single_quote && !in_double_quote {
            for comment_ch in chars.by_ref() {
                if comment_ch == '\n' {
                    current.push('\n');
                    break;
                }
            }
            continue;
        }

        match ch {
            '\'' if !in_double_quote => {
                current.push(ch);
                if chars.peek() == Some(&'\'') {
                    current.push(chars.next().unwrap());
                } else {
                    in_single_quote = !in_single_quote;
                }
            }
            '"' if !in_single_quote => {
                current.push(ch);
                in_double_quote = !in_double_quote;
            }
            ';' if !in_single_quote && !in_double_quote => {
                let statement = current.trim();
                if !statement.is_empty() {
                    statements.push(statement.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let statement = current.trim();
    if !statement.is_empty() {
        statements.push(statement.to_string());
    }

    statements
}

fn function_two_string_args(statement: &str, function_name: &str) -> Option<(String, String)> {
    let lower = statement.to_ascii_lowercase();
    let function_offset = lower.find(function_name)?;
    let args_offset = function_offset
        + function_name.len()
        + statement[function_offset + function_name.len()..]
            .chars()
            .take_while(|ch| ch.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();
    if statement.as_bytes().get(args_offset) != Some(&b'(') {
        return None;
    }
    let args = matching_parenthesized(statement, args_offset)?;
    let quoted_args = single_quoted_args(args);
    if quoted_args.len() < 2 {
        return None;
    }
    Some((
        normalize_identifier(&quoted_args[0]),
        normalize_identifier(&quoted_args[1]),
    ))
}

fn single_quoted_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\'' {
            continue;
        }
        let mut value = String::new();
        while let Some(arg_ch) = chars.next() {
            if arg_ch == '\'' {
                if chars.peek() == Some(&'\'') {
                    value.push(chars.next().unwrap());
                    continue;
                }
                break;
            }
            value.push(arg_ch);
        }
        args.push(value);
    }
    args
}

fn matching_parenthesized(input: &str, open_offset: usize) -> Option<&str> {
    let mut depth = 0usize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let start = open_offset + 1;

    for (offset, ch) in input
        .char_indices()
        .skip_while(|(offset, _)| *offset < open_offset)
    {
        match ch {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '(' if !in_single_quote && !in_double_quote => depth += 1,
            ')' if !in_single_quote && !in_double_quote => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return input.get(start..offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level_commas(input: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    for ch in input.chars() {
        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                current.push(ch);
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                current.push(ch);
            }
            '(' if !in_single_quote && !in_double_quote => {
                depth += 1;
                current.push(ch);
            }
            ')' if !in_single_quote && !in_double_quote => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if depth == 0 && !in_single_quote && !in_double_quote => {
                let value = current.trim();
                if !value.is_empty() {
                    values.push(value.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let value = current.trim();
    if !value.is_empty() {
        values.push(value.to_string());
    }
    values
}

fn table_after_keyword(statement: &str, keyword: &str) -> Option<String> {
    let mut tokens = statement.split_whitespace();
    while let Some(token) = tokens.next() {
        if token.eq_ignore_ascii_case(keyword) {
            return tokens.next().and_then(first_identifier);
        }
    }
    None
}

fn identifier_after_phrase(input: &str, phrase: &str) -> Option<String> {
    let lower = input.to_ascii_lowercase();
    let offset = lower.find(phrase)? + phrase.len();
    take_identifier_with_remainder(input[offset..].trim_start()).map(|(identifier, _)| identifier)
}

fn first_identifier(input: &str) -> Option<String> {
    take_identifier_with_remainder(input).map(|(identifier, _)| identifier)
}

fn take_identifier_with_remainder(input: &str) -> Option<(String, &str)> {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    let mut end = 0usize;
    let mut in_double_quote = false;
    for (offset, ch) in trimmed.char_indices() {
        if ch == '"' {
            in_double_quote = !in_double_quote;
            end = offset + ch.len_utf8();
            continue;
        }
        if !in_double_quote && (ch.is_whitespace() || matches!(ch, '(' | ')' | ',' | ';')) {
            break;
        }
        end = offset + ch.len_utf8();
    }
    if end == 0 {
        return None;
    }

    let identifier = normalize_identifier(&trimmed[..end]);
    let remainder = &trimmed[end..];
    Some((identifier, remainder))
}

fn where_columns(statement: &str) -> Vec<String> {
    let normalized = statement.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = normalized.to_ascii_lowercase();
    let Some(where_offset) = lower.find(" where ") else {
        return Vec::new();
    };
    let where_clause = &normalized[where_offset + " where ".len()..];
    let mut columns = Vec::new();
    for predicate in where_clause.split(['=', '<', '>', ',', ')', '(']) {
        let Some(identifier) = first_identifier(predicate) else {
            continue;
        };
        let keyword = identifier.to_ascii_lowercase();
        if matches!(
            keyword.as_str(),
            "and" | "or" | "not" | "is" | "null" | "true" | "false" | "select"
        ) || identifier.chars().all(|ch| ch.is_ascii_digit())
        {
            continue;
        }
        let column = identifier
            .rsplit('.')
            .next()
            .unwrap_or(identifier.as_str())
            .to_string();
        columns.push(column);
    }
    columns
}

fn optional_field(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() || value == "-" {
        None
    } else {
        Some(value)
    }
}

fn normalize_identifier(value: &str) -> String {
    let mut value = value
        .trim()
        .trim_end_matches(';')
        .trim_end_matches(',')
        .trim()
        .to_string();
    if let Some(offset) = value.to_ascii_lowercase().find("::regclass") {
        value.truncate(offset);
    }
    value
        .split('.')
        .map(|part| part.trim().trim_matches('"').to_string())
        .collect::<Vec<_>>()
        .join(".")
}

fn unqualified_table(table: &str) -> String {
    table.rsplit('.').next().unwrap_or(table).to_string()
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CitusLspError {
    MissingRequiredField(&'static str),
    InvalidMetadataLine { line: usize, detail: String },
}

impl fmt::Display for CitusLspError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::InvalidMetadataLine { line, detail } => {
                write!(formatter, "metadata line {line} is invalid: {detail}")
            }
        }
    }
}

impl Error for CitusLspError {}

fn missing_distribution_column_diagnostic(
    table: &str,
    columns: &[String],
    tenant_column: Option<&str>,
) -> Result<LspDiagnostic, CitusLspError> {
    let column = tenant_column
        .filter(|column| !column.trim().is_empty())
        .or_else(|| columns.first().map(String::as_str))
        .unwrap_or("");
    validate_required("distribution_column_hint", column)?;

    Ok(LspDiagnostic {
        code: LspDiagnosticCode::MissingDistributionColumn,
        severity: DiagnosticSeverity::Warning,
        message: format!("table {} should declare a Citus distribution column", table),
        quick_fix: Some(LspQuickFix {
            title: "Add distribution column".to_string(),
            action: LspQuickFixAction::AddDistributionColumn {
                table: table.to_string(),
                column: column.to_string(),
            },
        }),
    })
}

fn validate_required(field: &'static str, value: &str) -> Result<(), CitusLspError> {
    if value.trim().is_empty() {
        return Err(CitusLspError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(field: &'static str, values: &[String]) -> Result<(), CitusLspError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(CitusLspError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_colocated_join_reports_diagnostic() {
        let plan = plan_with_all_rules();
        let request = SqlAnalysisRequest {
            uri: "file:///workspace/query.sql".to_string(),
            intents: vec![SqlIntent::Join {
                left_table: "public.orders".to_string(),
                right_table: "public.events".to_string(),
            }],
        };

        let analysis = plan.analyze(&request).unwrap();

        assert_eq!(
            analysis.diagnostics[0].code,
            LspDiagnosticCode::NonColocatedJoin
        );
        assert!(matches!(
            analysis.diagnostics[0].quick_fix.as_ref().unwrap().action,
            LspQuickFixAction::AlignColocation { .. }
        ));
    }

    #[test]
    fn colocated_join_has_no_diagnostic() {
        let plan = plan_with_all_rules();
        let request = SqlAnalysisRequest {
            uri: "file:///workspace/query.sql".to_string(),
            intents: vec![SqlIntent::Join {
                left_table: "public.orders".to_string(),
                right_table: "public.line_items".to_string(),
            }],
        };

        assert!(plan.analyze(&request).unwrap().diagnostics.is_empty());
    }

    #[test]
    fn missing_distribution_column_has_m5_quick_fix() {
        let plan = plan_with_all_rules();
        let request = SqlAnalysisRequest {
            uri: "file:///workspace/schema.sql".to_string(),
            intents: vec![SqlIntent::CreateTable {
                table: "tenant_a.invoices".to_string(),
                columns: vec!["tenant_id".to_string(), "invoice_id".to_string()],
                distribution_column: None,
                tenant_column: Some("tenant_id".to_string()),
            }],
        };

        let analysis = plan.analyze(&request).unwrap();

        assert_eq!(
            analysis.diagnostics[0].quick_fix.as_ref().unwrap().action,
            LspQuickFixAction::AddDistributionColumn {
                table: "tenant_a.invoices".to_string(),
                column: "tenant_id".to_string()
            }
        );
    }

    #[test]
    fn hypertable_on_distributed_table_requires_bridge() {
        let plan = plan_with_all_rules();
        let request = SqlAnalysisRequest {
            uri: "file:///workspace/hypertable.sql".to_string(),
            intents: vec![SqlIntent::CreateHypertable {
                table: "public.events".to_string(),
                time_column: Some("created_at".to_string()),
                uses_distributed_bridge: false,
            }],
        };

        let analysis = plan.analyze(&request).unwrap();

        assert_eq!(
            analysis.diagnostics[0].code,
            LspDiagnosticCode::HypertableInvariant
        );
        assert!(matches!(
            analysis.diagnostics[0].quick_fix.as_ref().unwrap().action,
            LspQuickFixAction::UseDistributedHypertableBridge { .. }
        ));
    }

    #[test]
    fn tenant_table_select_requires_tenant_filter() {
        let plan = plan_with_all_rules();
        let request = SqlAnalysisRequest {
            uri: "file:///workspace/query.sql".to_string(),
            intents: vec![SqlIntent::Select {
                table: "public.orders".to_string(),
                where_columns: vec!["status".to_string()],
            }],
        };

        assert_eq!(
            plan.analyze(&request).unwrap().diagnostics[0].code,
            LspDiagnosticCode::MissingTenantFilter
        );
    }

    #[test]
    fn distribution_column_drop_is_error() {
        let plan = plan_with_all_rules();
        let request = SqlAnalysisRequest {
            uri: "file:///workspace/schema.sql".to_string(),
            intents: vec![SqlIntent::AlterColumn {
                table: "public.orders".to_string(),
                column: "tenant_id".to_string(),
                action: AlterColumnAction::Drop,
            }],
        };

        let diagnostic = &plan.analyze(&request).unwrap().diagnostics[0];
        assert_eq!(diagnostic.code, LspDiagnosticCode::DistributionColumnAlter);
        assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn canonical_plan_reports_all_executable_diagnostic_classes() {
        let analysis = canonical_lsp_plan()
            .unwrap()
            .analyze(&canonical_analysis_request())
            .unwrap();
        let codes = analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>();

        assert_eq!(
            codes,
            vec![
                LspDiagnosticCode::MissingDistributionColumn,
                LspDiagnosticCode::NonColocatedJoin,
                LspDiagnosticCode::DistributionColumnAlter,
                LspDiagnosticCode::HypertableInvariant,
                LspDiagnosticCode::MissingTenantFilter,
                LspDiagnosticCode::MissingSearchAnalyzer,
            ]
        );
        assert!(
            analysis
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.quick_fix.is_some())
                .count()
                >= 5
        );
    }

    #[test]
    fn parses_metadata_tsv_for_file_backed_analysis() {
        let metadata = parse_metadata_tsv(
            "distributed_table\tpublic.orders\ttenant_id\ttenant\ttenant_id\n\
             hypertable\tpublic.events\tcreated_at\tpublic.events\n\
             search_index\torders_search\tpublic.orders\t-\n\
             tenant\ttenant-a\ttenant_a\n",
        )
        .unwrap();

        assert_eq!(metadata.distributed_tables.len(), 1);
        assert_eq!(metadata.hypertables[0].table, "public.events");
        assert_eq!(metadata.search_indexes[0].analyzer, None);
        assert_eq!(metadata.tenants[0].schema, "tenant_a");
    }

    #[test]
    fn parses_sql_document_into_real_diagnostic_intents() {
        let sql = "\
            CREATE TABLE tenant_a.invoices (tenant_id uuid, invoice_id uuid);\n\
            CREATE TABLE public.shipments (tenant_id uuid, shipment_id uuid);\n\
            SELECT create_distributed_table('public.shipments', 'tenant_id');\n\
            SELECT * FROM public.orders JOIN public.events ON orders.device_id = events.device_id;\n\
            ALTER TABLE public.orders DROP COLUMN tenant_id;\n\
            SELECT create_hypertable('public.events', 'created_at');\n\
            SELECT * FROM public.orders WHERE status = 'open';\n\
            CREATE INDEX orders_search ON public.orders USING bm25 (status);\n";
        let request = parse_sql_document("file:///workspace/migration.sql", sql).unwrap();
        let analysis = canonical_lsp_plan().unwrap().analyze(&request).unwrap();
        let codes = analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>();

        assert!(codes.contains(&LspDiagnosticCode::MissingDistributionColumn));
        assert!(codes.contains(&LspDiagnosticCode::NonColocatedJoin));
        assert!(codes.contains(&LspDiagnosticCode::DistributionColumnAlter));
        assert!(codes.contains(&LspDiagnosticCode::HypertableInvariant));
        assert!(codes.contains(&LspDiagnosticCode::MissingTenantFilter));
        assert!(codes.contains(&LspDiagnosticCode::MissingSearchAnalyzer));
        assert_eq!(
            analysis
                .diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code == LspDiagnosticCode::MissingDistributionColumn
                })
                .count(),
            1
        );
    }

    #[test]
    fn distributed_hypertable_bridge_suppresses_hypertable_warning() {
        let sql = "\
            SELECT apply_distribute_hypertable('public.events', 'device_id', 'created_at', '1 day');\n\
            SELECT create_hypertable('public.events', 'created_at');\n";
        let request = parse_sql_document("file:///workspace/hypertable.sql", sql).unwrap();
        let analysis = canonical_lsp_plan().unwrap().analyze(&request).unwrap();

        assert!(!analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == LspDiagnosticCode::HypertableInvariant));
    }

    fn plan_with_all_rules() -> CitusLspPlan {
        canonical_lsp_plan().unwrap()
    }
}
