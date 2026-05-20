//! Unified live-view contracts for citus-watch.

// FEATURE: O13
// FEATURE: D12

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WatchDashboardPlan {
    pub refresh_interval_seconds: u32,
    pub data_sources: Vec<WatchDataSource>,
    pub panels: Vec<WatchPanel>,
}

impl WatchDashboardPlan {
    pub fn validate(&self) -> Result<(), WatchError> {
        if self.refresh_interval_seconds == 0 {
            return Err(WatchError::InvalidRefreshInterval);
        }
        if self.data_sources.is_empty() {
            return Err(WatchError::MissingRequiredField("data_sources"));
        }
        for source in &self.data_sources {
            source.validate()?;
        }
        if self.panels.is_empty() {
            return Err(WatchError::MissingRequiredField("panels"));
        }
        for panel in &self.panels {
            panel.validate()?;
        }
        Ok(())
    }

    pub fn queries(&self) -> Result<Vec<WatchQuery>, WatchError> {
        self.validate()?;
        Ok(self.panels.iter().map(WatchPanel::query).collect())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WatchDataSource {
    Prometheus { base_url: String },
    CompanionView { view_name: String },
    PoolMetrics { uds_path: String },
}

impl WatchDataSource {
    fn validate(&self) -> Result<(), WatchError> {
        match self {
            Self::Prometheus { base_url } => {
                validate_required("prometheus.base_url", base_url)?;
                if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
                    return Err(WatchError::InvalidPrometheusUrl);
                }
                Ok(())
            }
            Self::CompanionView { view_name } => {
                validate_required("companion_view.view_name", view_name)
            }
            Self::PoolMetrics { uds_path } => {
                validate_required("pool_metrics.uds_path", uds_path)?;
                if !uds_path.starts_with('/') {
                    return Err(WatchError::InvalidUdsPath);
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WatchPanel {
    ClusterTopology,
    Shards { max_rows: u32 },
    Hypertables { max_rows: u32 },
    CitusExplain,
    Rebalance,
    VectorizerBacklog,
    SearchIndexes,
    Tenants,
    Branches,
}

impl WatchPanel {
    fn validate(&self) -> Result<(), WatchError> {
        match self {
            Self::Shards { max_rows } | Self::Hypertables { max_rows } if *max_rows == 0 => {
                Err(WatchError::InvalidRowLimit)
            }
            _ => Ok(()),
        }
    }

    fn query(&self) -> WatchQuery {
        match self {
            Self::ClusterTopology => WatchQuery::CompanionSql {
                name: "cluster-topology",
                sql: "SELECT * FROM companion.distributed_tables",
            },
            Self::Shards { .. } => WatchQuery::CompanionSql {
                name: "shards",
                sql: "SELECT * FROM companion.shard_placements",
            },
            Self::Hypertables { .. } => WatchQuery::CompanionSql {
                name: "hypertables",
                sql: "SELECT * FROM companion.hypertables",
            },
            Self::CitusExplain => WatchQuery::CompanionSql {
                name: "citus-explain",
                sql: "SELECT * FROM companion.recent_distributed_explain",
            },
            Self::Rebalance => WatchQuery::CompanionSql {
                name: "rebalance",
                sql: "SELECT * FROM companion.rebalance_status",
            },
            Self::VectorizerBacklog => WatchQuery::Prometheus {
                name: "vectorizer-backlog",
                expr: "sum(ai_blaise_citus_vectorizer_backlog_jobs) by (tenant)",
            },
            Self::SearchIndexes => WatchQuery::CompanionSql {
                name: "search-indexes",
                sql: "SELECT * FROM companion.search_indexes",
            },
            Self::Tenants => WatchQuery::CompanionSql {
                name: "tenants",
                sql: "SELECT * FROM companion.tenants",
            },
            Self::Branches => WatchQuery::CompanionSql {
                name: "branches",
                sql: "SELECT * FROM companion.branches",
            },
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WatchQuery {
    CompanionSql {
        name: &'static str,
        sql: &'static str,
    },
    Prometheus {
        name: &'static str,
        expr: &'static str,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WatchError {
    InvalidPrometheusUrl,
    InvalidRefreshInterval,
    InvalidRowLimit,
    InvalidUdsPath,
    MissingRequiredField(&'static str),
}

impl fmt::Display for WatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrometheusUrl => {
                write!(formatter, "prometheus base_url must be http or https")
            }
            Self::InvalidRefreshInterval => {
                write!(
                    formatter,
                    "refresh_interval_seconds must be greater than zero"
                )
            }
            Self::InvalidRowLimit => write!(formatter, "max_rows must be greater than zero"),
            Self::InvalidUdsPath => write!(formatter, "uds_path must be absolute"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for WatchError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), WatchError> {
    if value.trim().is_empty() {
        return Err(WatchError::MissingRequiredField(field));
    }
    Ok(())
}

pub fn canonical_watch_dashboard() -> WatchDashboardPlan {
    WatchDashboardPlan {
        refresh_interval_seconds: 5,
        data_sources: vec![
            WatchDataSource::Prometheus {
                base_url: "http://prometheus.monitoring.svc:9090".to_string(),
            },
            WatchDataSource::CompanionView {
                view_name: "companion.distributed_tables".to_string(),
            },
            WatchDataSource::PoolMetrics {
                uds_path: "/var/run/ai-blaise/pool.sock".to_string(),
            },
        ],
        panels: vec![
            WatchPanel::ClusterTopology,
            WatchPanel::Shards { max_rows: 200 },
            WatchPanel::Hypertables { max_rows: 100 },
            WatchPanel::CitusExplain,
            WatchPanel::Rebalance,
            WatchPanel::VectorizerBacklog,
            WatchPanel::SearchIndexes,
            WatchPanel::Tenants,
            WatchPanel::Branches,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_plan_covers_unified_live_view() {
        let plan = canonical_watch_dashboard();

        let queries = plan.queries().unwrap();
        assert_eq!(queries.len(), 9);
        assert!(queries.contains(&WatchQuery::Prometheus {
            name: "vectorizer-backlog",
            expr: "sum(ai_blaise_citus_vectorizer_backlog_jobs) by (tenant)",
        }));
    }

    #[test]
    fn dashboard_requires_absolute_pool_socket() {
        let plan = WatchDashboardPlan {
            refresh_interval_seconds: 5,
            data_sources: vec![WatchDataSource::PoolMetrics {
                uds_path: "pool.sock".to_string(),
            }],
            panels: vec![WatchPanel::ClusterTopology],
        };

        assert_eq!(plan.validate(), Err(WatchError::InvalidUdsPath));
    }

    #[test]
    fn dashboard_rejects_empty_row_limit() {
        let panel = WatchPanel::Shards { max_rows: 0 };

        assert_eq!(panel.validate(), Err(WatchError::InvalidRowLimit));
    }
}
