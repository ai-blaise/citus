// FEATURE: Search2
// FEATURE: Search3
// FEATURE: Search9

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchIndexDistributedPlan {
    pub table: String,
    pub index_name: String,
    pub columns: Vec<SearchColumnPlan>,
    pub distribution_column: String,
}

impl SearchIndexDistributedPlan {
    pub fn new(
        table: impl Into<String>,
        index_name: impl Into<String>,
        columns: Vec<SearchColumnPlan>,
        distribution_column: impl Into<String>,
    ) -> Result<Self, SearchBridgeError> {
        let plan = Self {
            table: table.into(),
            index_name: index_name.into(),
            columns,
            distribution_column: distribution_column.into(),
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn validate(&self) -> Result<(), SearchBridgeError> {
        validate_required("table", &self.table)?;
        validate_required("index_name", &self.index_name)?;
        validate_required("distribution_column", &self.distribution_column)?;
        validate_columns(&self.columns)?;
        if !self
            .columns
            .iter()
            .any(|column| column.role == SearchColumnRole::Text)
        {
            return Err(SearchBridgeError::MissingTextColumn);
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<SearchSqlPlan, SearchBridgeError> {
        self.validate()?;
        let text_columns = self
            .columns
            .iter()
            .filter(|column| column.role == SearchColumnRole::Text)
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        SearchSqlPlan::new(
            "Search2",
            vec![
                format!(
                    "SELECT companion_internal.ensure_search_workers({}, {});",
                    sql_literal(&self.table),
                    sql_literal(&self.distribution_column)
                ),
                format!(
                    "CREATE INDEX IF NOT EXISTS {} ON {} USING bm25 ({});",
                    self.index_name, self.table, text_columns
                ),
            ],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HybridRankPlan {
    pub table: String,
    pub text_query: String,
    pub vector_column: String,
    pub vector_parameter: String,
    pub text_weight: u8,
    pub vector_weight: u8,
    pub limit: u32,
}

impl HybridRankPlan {
    pub fn validate(&self) -> Result<(), SearchBridgeError> {
        validate_required("table", &self.table)?;
        validate_required("text_query", &self.text_query)?;
        validate_required("vector_column", &self.vector_column)?;
        validate_required("vector_parameter", &self.vector_parameter)?;
        if self.text_weight == 0 || self.vector_weight == 0 {
            return Err(SearchBridgeError::InvalidWeight);
        }
        if self.limit == 0 {
            return Err(SearchBridgeError::InvalidLimit);
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<SearchSqlPlan, SearchBridgeError> {
        self.validate()?;
        SearchSqlPlan::new(
            "Search3",
            vec![format!(
                "SELECT *, ((bm25_score * {}) + (vector_score * {})) AS hybrid_score \
                 FROM companion_internal.hybrid_rank({}, {}, {}, {}) \
                 ORDER BY hybrid_score DESC LIMIT {};",
                self.text_weight,
                self.vector_weight,
                sql_literal(&self.table),
                sql_literal(&self.text_query),
                self.vector_column,
                self.vector_parameter,
                self.limit
            )],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RerankerPlan {
    pub input_view: String,
    pub provider: String,
    pub model: String,
    pub limit: u32,
}

impl RerankerPlan {
    pub fn validate(&self) -> Result<(), SearchBridgeError> {
        validate_required("input_view", &self.input_view)?;
        validate_required("provider", &self.provider)?;
        validate_required("model", &self.model)?;
        if self.limit == 0 {
            return Err(SearchBridgeError::InvalidLimit);
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<SearchSqlPlan, SearchBridgeError> {
        self.validate()?;
        SearchSqlPlan::new(
            "Search9",
            vec![format!(
                "SELECT * FROM companion_internal.rerank_search({}, {}, {}) LIMIT {};",
                sql_literal(&self.input_view),
                sql_literal(&self.provider),
                sql_literal(&self.model),
                self.limit
            )],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchColumnPlan {
    pub name: String,
    pub role: SearchColumnRole,
}

impl SearchColumnPlan {
    pub fn text(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            role: SearchColumnRole::Text,
        }
    }

    pub fn vector(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            role: SearchColumnRole::Vector,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SearchColumnRole {
    Text,
    Vector,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl SearchSqlPlan {
    fn new(feature_id: &'static str, commands: Vec<String>) -> Result<Self, SearchBridgeError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(SearchBridgeError::MissingRequiredField("commands"));
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
pub enum SearchBridgeError {
    InvalidLimit,
    InvalidWeight,
    MissingRequiredField(&'static str),
    MissingTextColumn,
}

impl fmt::Display for SearchBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit => write!(formatter, "limit must be greater than zero"),
            Self::InvalidWeight => write!(formatter, "rank weights must be greater than zero"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::MissingTextColumn => write!(formatter, "at least one text column is required"),
        }
    }
}

impl Error for SearchBridgeError {}

fn validate_columns(columns: &[SearchColumnPlan]) -> Result<(), SearchBridgeError> {
    if columns.is_empty() || columns.iter().any(|column| column.name.trim().is_empty()) {
        return Err(SearchBridgeError::MissingRequiredField("columns"));
    }
    Ok(())
}

fn validate_required(field: &'static str, value: &str) -> Result<(), SearchBridgeError> {
    if value.trim().is_empty() {
        return Err(SearchBridgeError::MissingRequiredField(field));
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
    fn distributed_search_index_renders_worker_fanout() {
        let plan = SearchIndexDistributedPlan::new(
            "public.docs",
            "docs_bm25",
            vec![
                SearchColumnPlan::text("title"),
                SearchColumnPlan::text("body"),
                SearchColumnPlan::vector("embedding"),
            ],
            "tenant_id",
        )
        .unwrap()
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "Search2");
        assert!(plan.script().contains("ensure_search_workers"));
        assert!(plan.script().contains("USING bm25 (title, body)"));
    }

    #[test]
    fn hybrid_rank_requires_positive_limit() {
        let plan = HybridRankPlan {
            table: "public.docs".to_string(),
            text_query: "database".to_string(),
            vector_column: "embedding".to_string(),
            vector_parameter: "$1".to_string(),
            text_weight: 1,
            vector_weight: 1,
            limit: 0,
        };

        assert_eq!(plan.validate(), Err(SearchBridgeError::InvalidLimit));
    }

    #[test]
    fn reranker_renders_provider_model_contract() {
        let plan = RerankerPlan {
            input_view: "companion.docs_hybrid".to_string(),
            provider: "openai".to_string(),
            model: "rerank-small".to_string(),
            limit: 20,
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "Search9");
        assert!(plan.script().contains("'openai'"));
    }
}
