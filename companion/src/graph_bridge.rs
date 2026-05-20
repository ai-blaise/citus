// FEATURE: G2
// FEATURE: G3
// FEATURE: API4

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphDistributionPlan {
    pub graph_name: String,
    pub vertex_table: String,
    pub edge_table: String,
    pub vertex_key: String,
    pub edge_source_key: String,
    pub edge_target_key: String,
    pub colocation_group: String,
}

impl GraphDistributionPlan {
    pub fn validate(&self) -> Result<(), GraphBridgeError> {
        validate_required("graph_name", &self.graph_name)?;
        validate_required("vertex_table", &self.vertex_table)?;
        validate_required("edge_table", &self.edge_table)?;
        validate_required("vertex_key", &self.vertex_key)?;
        validate_required("edge_source_key", &self.edge_source_key)?;
        validate_required("edge_target_key", &self.edge_target_key)?;
        validate_required("colocation_group", &self.colocation_group)
    }

    pub fn to_sql_plan(&self) -> Result<GraphSqlPlan, GraphBridgeError> {
        self.validate()?;
        GraphSqlPlan::new(
            "G2",
            vec![
                format!(
                    "SELECT ag_catalog.create_graph({});",
                    sql_literal(&self.graph_name)
                ),
                format!(
                    "SELECT companion_internal.ensure_graph_colocation({}, {}, {}, {});",
                    sql_literal(&self.vertex_table),
                    sql_literal(&self.edge_table),
                    sql_literal(&self.vertex_key),
                    sql_literal(&self.colocation_group)
                ),
                format!(
                    "SELECT companion_internal.register_graphql_distributed_graph({}, {}, {});",
                    sql_literal(&self.graph_name),
                    sql_literal(&self.vertex_table),
                    sql_literal(&self.edge_table)
                ),
            ],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl GraphSqlPlan {
    fn new(feature_id: &'static str, commands: Vec<String>) -> Result<Self, GraphBridgeError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(GraphBridgeError::MissingRequiredField("commands"));
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
pub enum GraphBridgeError {
    MissingRequiredField(&'static str),
}

impl fmt::Display for GraphBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for GraphBridgeError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), GraphBridgeError> {
    if value.trim().is_empty() {
        return Err(GraphBridgeError::MissingRequiredField(field));
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
    fn graph_distribution_renders_age_and_colocation_contract() {
        let plan = GraphDistributionPlan {
            graph_name: "tenant_graph".to_string(),
            vertex_table: "public.vertices".to_string(),
            edge_table: "public.edges".to_string(),
            vertex_key: "tenant_id".to_string(),
            edge_source_key: "source_id".to_string(),
            edge_target_key: "target_id".to_string(),
            colocation_group: "tenant".to_string(),
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "G2");
        assert!(plan.script().contains("create_graph"));
        assert!(plan.script().contains("ensure_graph_colocation"));
        assert!(plan.script().contains("register_graphql_distributed_graph"));
    }
}
