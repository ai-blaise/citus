//! Citus-aware language-server diagnostic contracts.

// FEATURE: D4
// FEATURE: M5
// FEATURE: TS8

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
                } if self.rule_enabled(LspRule::MissingDistributionColumnQuickFix) => {
                    if distribution_column
                        .as_deref()
                        .unwrap_or("")
                        .trim()
                        .is_empty()
                    {
                        diagnostics.push(missing_distribution_column_diagnostic(
                            table,
                            columns,
                            tenant_column.as_deref(),
                        )?);
                    }
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
                } if self.rule_enabled(LspRule::MissingSearchAnalyzer) => {
                    if analyzer.as_deref().unwrap_or("").trim().is_empty() {
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CitusLspError {
    MissingRequiredField(&'static str),
}

impl fmt::Display for CitusLspError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
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

    fn plan_with_all_rules() -> CitusLspPlan {
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
            vec![
                LspRule::NonColocatedJoin,
                LspRule::DistributionColumnAlter,
                LspRule::HypertableInvariant,
                LspRule::MissingTenantFilter,
                LspRule::MissingSearchAnalyzer,
                LspRule::MissingDistributionColumnQuickFix,
            ],
        )
        .unwrap()
    }
}
