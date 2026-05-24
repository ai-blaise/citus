//! Unified live-view contracts for citus-watch.

// FEATURE: O13
// FEATURE: D12

use ai_blaise_citus_tool_runtime::{terminal_table, ToolRuntimeError, ToolSnapshot};
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WatchRuntime {
    snapshot: ToolSnapshot,
    dashboard: WatchDashboardPlan,
}

impl WatchRuntime {
    pub fn new(snapshot: ToolSnapshot) -> Result<Self, WatchRuntimeError> {
        snapshot.validate().map_err(WatchRuntimeError::from)?;
        let dashboard = canonical_watch_dashboard();
        dashboard
            .validate()
            .map_err(|error| WatchRuntimeError::InvalidDashboard(error.to_string()))?;
        Ok(Self {
            snapshot,
            dashboard,
        })
    }

    pub fn signals(&self) -> Vec<WatchSignal> {
        let mut signals = Vec::new();
        match &self.snapshot.pool {
            Some(pool) if pool.is_ready() => signals.push(WatchSignal::ok(
                "pool",
                format!(
                    "{} active clients, {} waiting",
                    pool.active_clients, pool.waiting_clients
                ),
            )),
            Some(pool) => signals.push(WatchSignal::critical(
                "pool",
                format!(
                    "state={} upstream_errors={}",
                    pool.state, pool.upstream_errors
                ),
            )),
            None => signals.push(WatchSignal::warning("pool", "snapshot has no pool row")),
        }

        let backlog = self
            .snapshot
            .vectorizers
            .iter()
            .map(|vectorizer| vectorizer.backlog_jobs)
            .max()
            .unwrap_or(0);
        if backlog > 100 {
            signals.push(WatchSignal::warning(
                "vectorizer-backlog",
                format!("max backlog {backlog} jobs"),
            ));
        } else {
            signals.push(WatchSignal::ok(
                "vectorizer-backlog",
                format!("max backlog {backlog} jobs"),
            ));
        }

        let active_shards = self
            .snapshot
            .shards
            .iter()
            .filter(|shard| shard.state == "active")
            .count();
        signals.push(WatchSignal::ok(
            "shards",
            format!(
                "{active_shards}/{} placements active",
                self.snapshot.shards.len()
            ),
        ));

        signals.push(WatchSignal::ok(
            "tenants",
            format!("{} tenants tracked", self.snapshot.tenants.len()),
        ));
        signals
    }

    pub fn render_frame(&self) -> Result<String, WatchRuntimeError> {
        let queries = self
            .dashboard
            .queries()
            .map_err(|error| WatchRuntimeError::InvalidDashboard(error.to_string()))?;
        let signal_rows = self
            .signals()
            .into_iter()
            .map(|signal| vec![signal.name, signal.level, signal.detail])
            .collect::<Vec<_>>();
        let query_rows = queries
            .iter()
            .map(|query| match query {
                WatchQuery::CompanionSql { name, sql } => {
                    vec![
                        name.to_string(),
                        "companion-sql".to_string(),
                        sql.to_string(),
                    ]
                }
                WatchQuery::Prometheus { name, expr } => {
                    vec![name.to_string(), "prometheus".to_string(), expr.to_string()]
                }
            })
            .collect::<Vec<_>>();

        Ok(format!(
            "citus-watch | cluster={} | refresh={}s | generated_at={}\n{}\n{}",
            self.snapshot.cluster_name,
            self.dashboard.refresh_interval_seconds,
            self.snapshot.generated_at,
            terminal_table(&["signal", "level", "detail"], &signal_rows),
            terminal_table(&["query", "source", "statement"], &query_rows),
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WatchSignal {
    pub name: String,
    pub level: String,
    pub detail: String,
}

impl WatchSignal {
    fn ok(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            level: "ok".to_string(),
            detail: detail.into(),
        }
    }

    fn warning(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            level: "warning".to_string(),
            detail: detail.into(),
        }
    }

    fn critical(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            level: "critical".to_string(),
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WatchRuntimeError {
    InvalidDashboard(String),
    InvalidSnapshot(String),
}

impl From<ToolRuntimeError> for WatchRuntimeError {
    fn from(error: ToolRuntimeError) -> Self {
        Self::InvalidSnapshot(error.to_string())
    }
}

impl fmt::Display for WatchRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDashboard(detail) => {
                write!(formatter, "watch dashboard invalid: {detail}")
            }
            Self::InvalidSnapshot(detail) => write!(formatter, "watch snapshot invalid: {detail}"),
        }
    }
}

impl Error for WatchRuntimeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_blaise_citus_tool_runtime::canonical_snapshot;

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

    #[test]
    fn watch_runtime_renders_snapshot_frame() {
        let runtime = WatchRuntime::new(canonical_snapshot()).unwrap();

        let frame = runtime.render_frame().unwrap();

        assert!(frame.contains("citus-watch | cluster=prod-east"));
        assert!(frame.contains("vectorizer-backlog"));
        assert!(frame.contains("companion.shard_placements"));
    }

    #[test]
    fn watch_runtime_warns_on_vectorizer_backlog() {
        let runtime = WatchRuntime::new(canonical_snapshot()).unwrap();

        assert!(runtime
            .signals()
            .iter()
            .any(|signal| signal.level == "warning"));
    }
}
