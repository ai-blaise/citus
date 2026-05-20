// FEATURE: Geo2
// FEATURE: Geo3

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GeoDistributionPlan {
    pub table: String,
    pub geometry_column: String,
    pub distribution_column: String,
    pub grid: GeoGrid,
    pub shard_count: u32,
}

impl GeoDistributionPlan {
    pub fn validate(&self) -> Result<(), GeoValidationError> {
        validate_required("table", &self.table)?;
        validate_required("geometry_column", &self.geometry_column)?;
        validate_required("distribution_column", &self.distribution_column)?;
        if self.shard_count == 0 {
            return Err(GeoValidationError::InvalidShardCount);
        }
        self.grid.validate()
    }

    pub fn to_sql_plan(&self) -> Result<GeoSqlPlan, GeoValidationError> {
        self.validate()?;
        GeoSqlPlan::new(
            "Geo2",
            vec![
                format!(
                    "SELECT companion_internal.add_geohash_column({}, {}, {}, {});",
                    sql_literal(&self.table),
                    sql_literal(&self.geometry_column),
                    sql_literal(&self.distribution_column),
                    self.grid.precision
                ),
                format!(
                    "SELECT create_distributed_table({}, {}, shard_count => {});",
                    sql_literal(&self.table),
                    sql_literal(&self.distribution_column),
                    self.shard_count
                ),
            ],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GeoPruningPlan {
    pub table: String,
    pub geometry_column: String,
    pub grid: GeoGrid,
}

impl GeoPruningPlan {
    pub fn validate(&self) -> Result<(), GeoValidationError> {
        validate_required("table", &self.table)?;
        validate_required("geometry_column", &self.geometry_column)?;
        self.grid.validate()
    }

    pub fn to_sql_plan(&self) -> Result<GeoSqlPlan, GeoValidationError> {
        self.validate()?;
        GeoSqlPlan::new(
            "Geo3",
            vec![format!(
                "SELECT companion_internal.enable_geo_shard_pruning({}, {}, {});",
                sql_literal(&self.table),
                sql_literal(&self.geometry_column),
                self.grid.precision
            )],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GeoGrid {
    pub precision: u8,
    pub srid: u32,
}

impl GeoGrid {
    fn validate(&self) -> Result<(), GeoValidationError> {
        if self.precision == 0 || self.precision > 12 {
            return Err(GeoValidationError::InvalidGridPrecision);
        }
        if self.srid == 0 {
            return Err(GeoValidationError::InvalidSrid);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GeoSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl GeoSqlPlan {
    fn new(feature_id: &'static str, commands: Vec<String>) -> Result<Self, GeoValidationError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(GeoValidationError::MissingRequiredField("commands"));
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
pub enum GeoValidationError {
    InvalidGridPrecision,
    InvalidShardCount,
    InvalidSrid,
    MissingRequiredField(&'static str),
}

impl fmt::Display for GeoValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGridPrecision => write!(formatter, "grid precision must be 1..=12"),
            Self::InvalidShardCount => write!(formatter, "shard_count must be greater than zero"),
            Self::InvalidSrid => write!(formatter, "srid must be greater than zero"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for GeoValidationError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), GeoValidationError> {
    if value.trim().is_empty() {
        return Err(GeoValidationError::MissingRequiredField(field));
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
    fn geo_distribution_renders_geohash_and_distribution_sql() {
        let plan = GeoDistributionPlan {
            table: "public.places".to_string(),
            geometry_column: "geom".to_string(),
            distribution_column: "geo_hash".to_string(),
            grid: GeoGrid {
                precision: 7,
                srid: 4326,
            },
            shard_count: 32,
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "Geo2");
        assert!(plan.script().contains("add_geohash_column"));
        assert!(plan.script().contains("create_distributed_table"));
    }

    #[test]
    fn geo_pruning_requires_valid_precision() {
        let plan = GeoPruningPlan {
            table: "public.places".to_string(),
            geometry_column: "geom".to_string(),
            grid: GeoGrid {
                precision: 13,
                srid: 4326,
            },
        };

        assert_eq!(
            plan.validate(),
            Err(GeoValidationError::InvalidGridPrecision)
        );
    }
}
