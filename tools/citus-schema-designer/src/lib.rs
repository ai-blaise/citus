//! Schema visualization contracts for citus-schema-designer.

// FEATURE: M9
// FEATURE: D6

use ai_blaise_citus_tool_runtime::{escape_html, ToolRuntimeError, ToolSnapshot};
use std::error::Error;
use std::fmt;
use std::fmt::Write as _;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SchemaDesignerModel {
    pub tables: Vec<DesignerTable>,
    pub relationships: Vec<DesignerRelationship>,
    pub shard_map: Vec<ShardPlacementVisual>,
}

impl SchemaDesignerModel {
    pub fn validate(&self) -> Result<(), SchemaDesignerError> {
        if self.tables.is_empty() {
            return Err(SchemaDesignerError::MissingRequiredField("tables"));
        }
        for table in &self.tables {
            table.validate()?;
        }
        for relationship in &self.relationships {
            relationship.validate()?;
        }
        for placement in &self.shard_map {
            placement.validate()?;
        }
        Ok(())
    }

    pub fn overlay_layers(&self) -> Result<Vec<DesignerOverlayLayer>, SchemaDesignerError> {
        self.validate()?;
        let mut layers = Vec::new();
        for table in &self.tables {
            if let Some(distribution) = &table.distribution {
                layers.push(DesignerOverlayLayer {
                    table: table.name.clone(),
                    kind: DesignerOverlayKind::Distribution,
                    label: format!(
                        "{} / {} shards",
                        distribution.distribution_column, distribution.shard_count
                    ),
                });
            }
            if let Some(hypertable) = &table.hypertable {
                layers.push(DesignerOverlayLayer {
                    table: table.name.clone(),
                    kind: DesignerOverlayKind::Hypertable,
                    label: format!(
                        "{} every {}",
                        hypertable.time_column, hypertable.chunk_interval
                    ),
                });
            }
            if table.search_indexes > 0 {
                layers.push(DesignerOverlayLayer {
                    table: table.name.clone(),
                    kind: DesignerOverlayKind::SearchIndex,
                    label: format!("{} search indexes", table.search_indexes),
                });
            }
            if table.webhook_count > 0 {
                layers.push(DesignerOverlayLayer {
                    table: table.name.clone(),
                    kind: DesignerOverlayKind::Webhook,
                    label: format!("{} webhooks", table.webhook_count),
                });
            }
        }
        for placement in &self.shard_map {
            layers.push(DesignerOverlayLayer {
                table: placement.table.clone(),
                kind: DesignerOverlayKind::ShardPlacement,
                label: format!("shard {} on {}", placement.shard_id, placement.worker),
            });
        }
        Ok(layers)
    }

    pub fn render_svg(&self) -> Result<String, SchemaDesignerError> {
        let layers = self.overlay_layers()?;
        let table_height = 92_i32;
        let table_gap = 34_i32;
        let width = 960_i32;
        let height = 120_i32 + (self.tables.len() as i32 * (table_height + table_gap)).max(180);
        let mut out = String::new();
        let _ = writeln!(
            out,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" data-feature=\"D6 M9\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">"
        );
        out.push_str("<style>text{font-family:monospace;font-size:13px}.table{fill:#f8fafc;stroke:#334155}.overlay{fill:#ecfeff;stroke:#0891b2}.placement{fill:#fef3c7;stroke:#d97706}</style>\n");
        let _ = writeln!(
            out,
            "<text x=\"24\" y=\"32\">citus-schema-designer snapshot render</text>"
        );

        for (index, table) in self.tables.iter().enumerate() {
            let y = 60 + index as i32 * (table_height + table_gap);
            let _ = writeln!(
                out,
                "<g data-table=\"{}\"><rect class=\"table\" x=\"24\" y=\"{y}\" width=\"360\" height=\"{table_height}\" rx=\"6\"/><text x=\"42\" y=\"{}\">{}</text>",
                escape_html(&table.name),
                y + 24,
                escape_html(&table.name)
            );
            for (column_index, column) in table.columns.iter().enumerate() {
                let _ = writeln!(
                    out,
                    "<text x=\"42\" y=\"{}\">{} {}</text>",
                    y + 48 + column_index as i32 * 18,
                    escape_html(&column.name),
                    escape_html(&column.sql_type)
                );
            }
            out.push_str("</g>\n");
        }

        for (index, layer) in layers.iter().enumerate() {
            let y = 60 + index as i32 * 34;
            let class_name = match layer.kind {
                DesignerOverlayKind::ShardPlacement => "placement",
                _ => "overlay",
            };
            let _ = writeln!(
                out,
                "<g data-overlay=\"{:?}\"><rect class=\"{class_name}\" x=\"430\" y=\"{y}\" width=\"490\" height=\"24\" rx=\"4\"/><text x=\"442\" y=\"{}\">{}: {}</text></g>",
                layer.kind,
                y + 17,
                escape_html(&layer.table),
                escape_html(&layer.label)
            );
        }

        out.push_str("</svg>\n");
        Ok(out)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DesignerTable {
    pub name: String,
    pub columns: Vec<DesignerColumn>,
    pub distribution: Option<DistributionOverlay>,
    pub hypertable: Option<HypertableOverlay>,
    pub search_indexes: u32,
    pub webhook_count: u32,
}

impl DesignerTable {
    fn validate(&self) -> Result<(), SchemaDesignerError> {
        validate_required("table.name", &self.name)?;
        if self.columns.is_empty() {
            return Err(SchemaDesignerError::MissingRequiredField("table.columns"));
        }
        for column in &self.columns {
            column.validate()?;
        }
        if let Some(distribution) = &self.distribution {
            distribution.validate()?;
            if !self
                .columns
                .iter()
                .any(|column| column.name == distribution.distribution_column)
            {
                return Err(SchemaDesignerError::DistributionColumnMissing);
            }
        }
        if let Some(hypertable) = &self.hypertable {
            hypertable.validate()?;
            if !self
                .columns
                .iter()
                .any(|column| column.name == hypertable.time_column)
            {
                return Err(SchemaDesignerError::HypertableColumnMissing);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DesignerColumn {
    pub name: String,
    pub sql_type: String,
    pub nullable: bool,
}

impl DesignerColumn {
    fn validate(&self) -> Result<(), SchemaDesignerError> {
        validate_required("column.name", &self.name)?;
        validate_required("column.sql_type", &self.sql_type)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DistributionOverlay {
    pub distribution_column: String,
    pub colocation_group: String,
    pub shard_count: u32,
}

impl DistributionOverlay {
    fn validate(&self) -> Result<(), SchemaDesignerError> {
        validate_required(
            "distribution.distribution_column",
            &self.distribution_column,
        )?;
        validate_required("distribution.colocation_group", &self.colocation_group)?;
        if self.shard_count == 0 {
            return Err(SchemaDesignerError::InvalidShardCount);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HypertableOverlay {
    pub time_column: String,
    pub chunk_interval: String,
}

impl HypertableOverlay {
    fn validate(&self) -> Result<(), SchemaDesignerError> {
        validate_required("hypertable.time_column", &self.time_column)?;
        validate_required("hypertable.chunk_interval", &self.chunk_interval)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DesignerRelationship {
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
}

impl DesignerRelationship {
    fn validate(&self) -> Result<(), SchemaDesignerError> {
        validate_required("relationship.from_table", &self.from_table)?;
        validate_required("relationship.from_column", &self.from_column)?;
        validate_required("relationship.to_table", &self.to_table)?;
        validate_required("relationship.to_column", &self.to_column)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardPlacementVisual {
    pub table: String,
    pub shard_id: u64,
    pub worker: String,
    pub state: PlacementState,
}

impl ShardPlacementVisual {
    fn validate(&self) -> Result<(), SchemaDesignerError> {
        validate_required("placement.table", &self.table)?;
        validate_required("placement.worker", &self.worker)?;
        if self.shard_id == 0 {
            return Err(SchemaDesignerError::InvalidShardId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PlacementState {
    Active,
    Rebalancing,
    Draining,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DesignerOverlayLayer {
    pub table: String,
    pub kind: DesignerOverlayKind,
    pub label: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DesignerOverlayKind {
    Distribution,
    Hypertable,
    SearchIndex,
    Webhook,
    ShardPlacement,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SchemaDesignerError {
    DistributionColumnMissing,
    HypertableColumnMissing,
    InvalidShardCount,
    InvalidShardId,
    MissingRequiredField(&'static str),
    RuntimeSnapshot(String),
}

impl fmt::Display for SchemaDesignerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DistributionColumnMissing => {
                write!(formatter, "distribution column must exist in table columns")
            }
            Self::HypertableColumnMissing => {
                write!(
                    formatter,
                    "hypertable time column must exist in table columns"
                )
            }
            Self::InvalidShardCount => write!(formatter, "shard_count must be greater than zero"),
            Self::InvalidShardId => write!(formatter, "shard_id must be greater than zero"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::RuntimeSnapshot(detail) => {
                write!(formatter, "runtime snapshot invalid: {detail}")
            }
        }
    }
}

impl Error for SchemaDesignerError {}

impl From<ToolRuntimeError> for SchemaDesignerError {
    fn from(error: ToolRuntimeError) -> Self {
        Self::RuntimeSnapshot(error.to_string())
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), SchemaDesignerError> {
    if value.trim().is_empty() {
        return Err(SchemaDesignerError::MissingRequiredField(field));
    }
    Ok(())
}

pub fn schema_designer_model_from_snapshot(
    snapshot: &ToolSnapshot,
) -> Result<SchemaDesignerModel, SchemaDesignerError> {
    snapshot.validate()?;
    let tables = snapshot
        .tables
        .iter()
        .map(|table| {
            let mut columns = vec![DesignerColumn {
                name: table.distribution_column.clone(),
                sql_type: "text".to_string(),
                nullable: false,
            }];
            let hypertable = match (&table.hypertable_time_column, &table.chunk_interval) {
                (Some(time_column), Some(chunk_interval)) => {
                    if !columns.iter().any(|column| column.name == *time_column) {
                        columns.push(DesignerColumn {
                            name: time_column.clone(),
                            sql_type: "timestamptz".to_string(),
                            nullable: false,
                        });
                    }
                    Some(HypertableOverlay {
                        time_column: time_column.clone(),
                        chunk_interval: chunk_interval.clone(),
                    })
                }
                _ => None,
            };
            DesignerTable {
                name: table.name.clone(),
                columns,
                distribution: Some(DistributionOverlay {
                    distribution_column: table.distribution_column.clone(),
                    colocation_group: table.colocation_group.clone(),
                    shard_count: table.shard_count,
                }),
                hypertable,
                search_indexes: table.search_indexes,
                webhook_count: table.webhook_count,
            }
        })
        .collect::<Vec<_>>();

    let shard_map = snapshot
        .shards
        .iter()
        .map(|shard| ShardPlacementVisual {
            table: shard.table.clone(),
            shard_id: shard.shard_id,
            worker: shard.worker.clone(),
            state: match shard.state.as_str() {
                "draining" => PlacementState::Draining,
                "rebalancing" => PlacementState::Rebalancing,
                _ => PlacementState::Active,
            },
        })
        .collect();

    Ok(SchemaDesignerModel {
        tables,
        relationships: Vec::new(),
        shard_map,
    })
}

pub fn canonical_schema_designer_model() -> SchemaDesignerModel {
    SchemaDesignerModel {
        tables: vec![DesignerTable {
            name: "public.events".to_string(),
            columns: vec![
                DesignerColumn {
                    name: "tenant_id".to_string(),
                    sql_type: "uuid".to_string(),
                    nullable: false,
                },
                DesignerColumn {
                    name: "created_at".to_string(),
                    sql_type: "timestamptz".to_string(),
                    nullable: false,
                },
            ],
            distribution: Some(DistributionOverlay {
                distribution_column: "tenant_id".to_string(),
                colocation_group: "tenant".to_string(),
                shard_count: 32,
            }),
            hypertable: Some(HypertableOverlay {
                time_column: "created_at".to_string(),
                chunk_interval: "1 day".to_string(),
            }),
            search_indexes: 1,
            webhook_count: 2,
        }],
        relationships: Vec::new(),
        shard_map: vec![ShardPlacementVisual {
            table: "public.events".to_string(),
            shard_id: 102_008,
            worker: "worker-1".to_string(),
            state: PlacementState::Active,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_blaise_citus_tool_runtime::canonical_snapshot;

    #[test]
    fn schema_designer_outputs_live_overlay_layers() {
        let model = canonical_schema_designer_model();

        let layers = model.overlay_layers().unwrap();
        assert!(layers
            .iter()
            .any(|layer| layer.kind == DesignerOverlayKind::Distribution));
        assert!(layers
            .iter()
            .any(|layer| layer.kind == DesignerOverlayKind::ShardPlacement));
    }

    #[test]
    fn schema_designer_rejects_missing_distribution_column() {
        let table = DesignerTable {
            name: "public.events".to_string(),
            columns: vec![DesignerColumn {
                name: "created_at".to_string(),
                sql_type: "timestamptz".to_string(),
                nullable: false,
            }],
            distribution: Some(DistributionOverlay {
                distribution_column: "tenant_id".to_string(),
                colocation_group: "tenant".to_string(),
                shard_count: 32,
            }),
            hypertable: None,
            search_indexes: 0,
            webhook_count: 0,
        };

        assert_eq!(
            table.validate(),
            Err(SchemaDesignerError::DistributionColumnMissing)
        );
    }

    #[test]
    fn schema_designer_rejects_zero_shard_id() {
        let placement = ShardPlacementVisual {
            table: "public.events".to_string(),
            shard_id: 0,
            worker: "worker-1".to_string(),
            state: PlacementState::Active,
        };

        assert_eq!(
            placement.validate(),
            Err(SchemaDesignerError::InvalidShardId)
        );
    }

    #[test]
    fn schema_designer_renders_snapshot_svg() {
        let model = schema_designer_model_from_snapshot(&canonical_snapshot()).unwrap();

        let svg = model.render_svg().unwrap();

        assert!(svg.contains("<svg"));
        assert!(svg.contains("data-feature=\"D6 M9\""));
        assert!(svg.contains("public.events"));
        assert!(svg.contains("shard 102008 on worker-1"));
    }
}
