// FEATURE: D4
// FEATURE: M5
// FEATURE: TS8

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LspMetadataViewPlan {
    pub schema: String,
    pub views: Vec<LspMetadataView>,
}

impl LspMetadataViewPlan {
    pub fn new(
        schema: impl Into<String>,
        views: Vec<LspMetadataView>,
    ) -> Result<Self, LspMetadataError> {
        let plan = Self {
            schema: schema.into(),
            views,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn canonical() -> Self {
        Self {
            schema: "companion".to_string(),
            views: vec![
                LspMetadataView::DistributedTables,
                LspMetadataView::ColocationGroups,
                LspMetadataView::Hypertables,
                LspMetadataView::SearchIndexes,
                LspMetadataView::Tenants,
            ],
        }
    }

    pub fn validate(&self) -> Result<(), LspMetadataError> {
        validate_required("schema", &self.schema)?;
        if self.views.is_empty() {
            return Err(LspMetadataError::MissingRequiredField("views"));
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<LspMetadataSqlPlan, LspMetadataError> {
        self.validate()?;

        let mut commands = vec![format!(
            "CREATE SCHEMA IF NOT EXISTS {};",
            quote_identifier(&self.schema)
        )];
        for view in &self.views {
            commands.push(view.render_sql(&self.schema));
        }

        LspMetadataSqlPlan::new(commands)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LspMetadataView {
    DistributedTables,
    ColocationGroups,
    Hypertables,
    SearchIndexes,
    Tenants,
}

impl LspMetadataView {
    pub fn view_name(self) -> &'static str {
        match self {
            Self::DistributedTables => "distributed_tables",
            Self::ColocationGroups => "colocation_groups",
            Self::Hypertables => "hypertables",
            Self::SearchIndexes => "search_indexes",
            Self::Tenants => "tenants",
        }
    }

    fn render_sql(self, schema: &str) -> String {
        let qualified_name = format!(
            "{}.{}",
            quote_identifier(schema),
            quote_identifier(self.view_name())
        );
        match self {
            Self::DistributedTables => format!(
                "CREATE OR REPLACE VIEW {qualified_name} AS\n\
                 SELECT logicalrelid::text AS table_name,\n\
                        partmethod::text AS partition_method,\n\
                        partkey::text AS partition_key,\n\
                        colocationid::bigint AS colocation_id\n\
                 FROM pg_dist_partition;"
            ),
            Self::ColocationGroups => format!(
                "CREATE OR REPLACE VIEW {qualified_name} AS\n\
                 SELECT colocationid::bigint AS colocation_id,\n\
                        shardcount::integer AS shard_count,\n\
                        replicationfactor::integer AS replication_factor,\n\
                        distributioncolumntype::text AS distribution_column_type\n\
                 FROM pg_dist_colocation;"
            ),
            Self::Hypertables => format!(
                "CREATE OR REPLACE VIEW {qualified_name} AS\n\
                 SELECT hypertable_schema || '.' || hypertable_name AS table_name,\n\
                        num_dimensions::integer AS dimensions,\n\
                        compression_enabled::boolean AS compression_enabled\n\
                 FROM timescaledb_information.hypertables;"
            ),
            Self::SearchIndexes => format!(
                "CREATE OR REPLACE VIEW {qualified_name} AS\n\
                 SELECT schemaname || '.' || indexname AS index_name,\n\
                        schemaname || '.' || tablename AS table_name,\n\
                        indexdef AS definition\n\
                 FROM pg_indexes\n\
                 WHERE indexdef ILIKE '%USING bm25%'\n\
                    OR indexdef ILIKE '%USING hnsw%'\n\
                    OR indexdef ILIKE '%USING ivfflat%';"
            ),
            Self::Tenants => format!(
                "CREATE OR REPLACE VIEW {qualified_name} AS\n\
                 SELECT nspname AS schema_name\n\
                 FROM pg_namespace\n\
                 WHERE nspname NOT LIKE 'pg\\_%' ESCAPE '\\'\n\
                   AND nspname <> 'information_schema'\n\
                   AND nspname !~ '^pg_toast';"
            ),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LspMetadataSqlPlan {
    pub commands: Vec<String>,
}

impl LspMetadataSqlPlan {
    fn new(commands: Vec<String>) -> Result<Self, LspMetadataError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(LspMetadataError::MissingRequiredField("commands"));
        }
        Ok(Self { commands })
    }

    pub fn script(&self) -> String {
        self.commands.join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LspMetadataError {
    MissingRequiredField(&'static str),
}

impl fmt::Display for LspMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for LspMetadataError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), LspMetadataError> {
    if value.trim().is_empty() {
        return Err(LspMetadataError::MissingRequiredField(field));
    }
    Ok(())
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_plan_renders_all_lsp_metadata_views() {
        let plan = LspMetadataViewPlan::canonical().to_sql_plan().unwrap();
        let script = plan.script();

        assert!(script.contains("CREATE SCHEMA IF NOT EXISTS \"companion\";"));
        assert!(script.contains("\"companion\".\"distributed_tables\""));
        assert!(script.contains("\"companion\".\"colocation_groups\""));
        assert!(script.contains("\"companion\".\"hypertables\""));
        assert!(script.contains("\"companion\".\"search_indexes\""));
        assert!(script.contains("\"companion\".\"tenants\""));
    }

    #[test]
    fn plan_requires_at_least_one_view() {
        let plan = LspMetadataViewPlan {
            schema: "companion".to_string(),
            views: Vec::new(),
        };

        assert_eq!(
            plan.validate(),
            Err(LspMetadataError::MissingRequiredField("views"))
        );
    }

    #[test]
    fn identifiers_are_quoted() {
        let plan =
            LspMetadataViewPlan::new("companion-extra", vec![LspMetadataView::DistributedTables])
                .unwrap()
                .to_sql_plan()
                .unwrap();

        assert!(plan.script().contains("\"companion-extra\""));
    }
}
