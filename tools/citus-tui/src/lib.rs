// FEATURE: D3

use ai_blaise_citus_tool_runtime::{terminal_table, ToolRuntimeError, ToolSnapshot};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiSessionPlan {
    pub connection_name: String,
    pub panels: Vec<TuiPanel>,
    pub actions: Vec<TuiAction>,
    pub safe_mode: bool,
}

impl TuiSessionPlan {
    pub fn new(
        connection_name: impl Into<String>,
        panels: Vec<TuiPanel>,
        actions: Vec<TuiAction>,
    ) -> Self {
        Self {
            connection_name: connection_name.into(),
            panels,
            actions,
            safe_mode: true,
        }
    }

    pub fn with_safe_mode(mut self, safe_mode: bool) -> Self {
        self.safe_mode = safe_mode;
        self
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.connection_name.trim().is_empty() {
            errors.push("connection_name is required".to_string());
        }
        if self.panels.is_empty() {
            errors.push("at least one panel is required".to_string());
        }
        if self.actions.is_empty() {
            errors.push("at least one action is required".to_string());
        }

        for action in &self.actions {
            if self.safe_mode && action.is_destructive() {
                errors.push(format!("safe_mode blocks {}", action.name()));
            }
            action.validate(&mut errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn required_panels() -> Vec<TuiPanel> {
        vec![
            TuiPanel::Cluster,
            TuiPanel::Shards,
            TuiPanel::Hypertables,
            TuiPanel::CitusExplain,
            TuiPanel::Rebalance,
            TuiPanel::VectorizerBacklog,
            TuiPanel::SearchIndexes,
            TuiPanel::Tenants,
            TuiPanel::Branches,
        ]
    }

    pub fn covers_required_panels(&self) -> bool {
        Self::required_panels()
            .iter()
            .all(|panel| self.panels.contains(panel))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TuiPanel {
    Cluster,
    Shards,
    Hypertables,
    CitusExplain,
    Rebalance,
    VectorizerBacklog,
    SearchIndexes,
    Tenants,
    Branches,
}

impl TuiPanel {
    pub fn as_name(&self) -> &'static str {
        match self {
            Self::Cluster => "cluster",
            Self::Shards => "shards",
            Self::Hypertables => "hypertables",
            Self::CitusExplain => "citus-explain",
            Self::Rebalance => "rebalance",
            Self::VectorizerBacklog => "vectorizer-backlog",
            Self::SearchIndexes => "search-indexes",
            Self::Tenants => "tenants",
            Self::Branches => "branches",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "cluster" => Some(Self::Cluster),
            "shards" => Some(Self::Shards),
            "hypertables" => Some(Self::Hypertables),
            "citus-explain" => Some(Self::CitusExplain),
            "rebalance" => Some(Self::Rebalance),
            "vectorizer-backlog" => Some(Self::VectorizerBacklog),
            "search-indexes" => Some(Self::SearchIndexes),
            "tenants" => Some(Self::Tenants),
            "branches" => Some(Self::Branches),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TuiAction {
    ExplainQuery {
        sql: String,
    },
    RebalanceDryRun {
        shard_id: u64,
    },
    RebalanceApply {
        shard_id: u64,
    },
    TenantMove {
        tenant: String,
        target_worker: String,
    },
    BranchPromote {
        branch: String,
    },
}

impl TuiAction {
    pub fn name(&self) -> &'static str {
        match self {
            Self::ExplainQuery { .. } => "explain-query",
            Self::RebalanceDryRun { .. } => "rebalance-dry-run",
            Self::RebalanceApply { .. } => "rebalance-apply",
            Self::TenantMove { .. } => "tenant-move",
            Self::BranchPromote { .. } => "branch-promote",
        }
    }

    fn is_destructive(&self) -> bool {
        matches!(
            self,
            Self::RebalanceApply { .. } | Self::TenantMove { .. } | Self::BranchPromote { .. }
        )
    }

    fn validate(&self, errors: &mut Vec<String>) {
        match self {
            Self::ExplainQuery { sql } if sql.trim().is_empty() => {
                errors.push("explain-query sql is required".to_string());
            }
            Self::RebalanceDryRun { shard_id } | Self::RebalanceApply { shard_id }
                if *shard_id == 0 =>
            {
                errors.push(format!("{} shard_id must be positive", self.name()));
            }
            Self::TenantMove {
                tenant,
                target_worker,
            } => {
                if tenant.trim().is_empty() {
                    errors.push("tenant-move tenant is required".to_string());
                }
                if target_worker.trim().is_empty() {
                    errors.push("tenant-move target_worker is required".to_string());
                }
            }
            Self::BranchPromote { branch } if branch.trim().is_empty() => {
                errors.push("branch-promote branch is required".to_string());
            }
            _ => {}
        }
    }
}

pub fn canonical_tui_session() -> TuiSessionPlan {
    TuiSessionPlan::new(
        "production-readonly",
        TuiSessionPlan::required_panels(),
        vec![
            TuiAction::ExplainQuery {
                sql: "select * from events where tenant_id = $1".to_string(),
            },
            TuiAction::RebalanceDryRun { shard_id: 1024 },
        ],
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiRuntime {
    snapshot: ToolSnapshot,
    session: TuiSessionPlan,
}

impl TuiRuntime {
    pub fn new(snapshot: ToolSnapshot) -> Result<Self, TuiRuntimeError> {
        snapshot.validate().map_err(TuiRuntimeError::from)?;
        let session = canonical_tui_session();
        if let Err(errors) = session.validate() {
            return Err(TuiRuntimeError::InvalidSession(errors.join("; ")));
        }
        Ok(Self { snapshot, session })
    }

    pub fn render_panel(&self, panel: TuiPanel) -> Result<String, TuiRuntimeError> {
        if !self.session.panels.contains(&panel) {
            return Err(TuiRuntimeError::UnknownPanel(panel.as_name().to_string()));
        }
        let table = match panel {
            TuiPanel::Cluster => terminal_table(
                &["worker", "host", "role", "readiness"],
                &self
                    .snapshot
                    .workers
                    .iter()
                    .map(|worker| {
                        vec![
                            worker.name.clone(),
                            worker.host.clone(),
                            worker.role.clone(),
                            worker.readiness.clone(),
                        ]
                    })
                    .collect::<Vec<_>>(),
            ),
            TuiPanel::Shards => terminal_table(
                &["table", "shard", "worker", "state", "bytes"],
                &self
                    .snapshot
                    .shards
                    .iter()
                    .map(|shard| {
                        vec![
                            shard.table.clone(),
                            shard.shard_id.to_string(),
                            shard.worker.clone(),
                            shard.state.clone(),
                            shard.bytes.to_string(),
                        ]
                    })
                    .collect::<Vec<_>>(),
            ),
            TuiPanel::Hypertables => terminal_table(
                &["table", "time", "interval", "shards"],
                &self
                    .snapshot
                    .tables
                    .iter()
                    .filter(|table| table.hypertable_time_column.is_some())
                    .map(|table| {
                        vec![
                            table.name.clone(),
                            table.hypertable_time_column.clone().unwrap_or_default(),
                            table.chunk_interval.clone().unwrap_or_default(),
                            table.shard_count.to_string(),
                        ]
                    })
                    .collect::<Vec<_>>(),
            ),
            TuiPanel::CitusExplain => terminal_table(
                &["query", "mode", "guard"],
                &[vec![
                    "select * from events where tenant_id = $1".to_string(),
                    "distributed-plan".to_string(),
                    "read-only".to_string(),
                ]],
            ),
            TuiPanel::Rebalance => terminal_table(
                &["shard", "state", "worker", "action"],
                &self
                    .snapshot
                    .shards
                    .iter()
                    .map(|shard| {
                        vec![
                            shard.shard_id.to_string(),
                            shard.state.clone(),
                            shard.worker.clone(),
                            "dry-run".to_string(),
                        ]
                    })
                    .collect::<Vec<_>>(),
            ),
            TuiPanel::VectorizerBacklog => terminal_table(
                &["name", "tenant", "backlog", "budget", "state"],
                &self
                    .snapshot
                    .vectorizers
                    .iter()
                    .map(|vectorizer| {
                        vec![
                            vectorizer.name.clone(),
                            vectorizer.tenant_id.clone(),
                            vectorizer.backlog_jobs.to_string(),
                            vectorizer.budget_remaining_tokens.to_string(),
                            vectorizer.state.clone(),
                        ]
                    })
                    .collect::<Vec<_>>(),
            ),
            TuiPanel::SearchIndexes => terminal_table(
                &["table", "index", "state", "method"],
                &self
                    .snapshot
                    .search_indexes
                    .iter()
                    .map(|index| {
                        vec![
                            index.table.clone(),
                            index.name.clone(),
                            index.state.clone(),
                            index.method.clone(),
                        ]
                    })
                    .collect::<Vec<_>>(),
            ),
            TuiPanel::Tenants => terminal_table(
                &["tenant", "state", "home_worker", "shards"],
                &self
                    .snapshot
                    .tenants
                    .iter()
                    .map(|tenant| {
                        vec![
                            tenant.tenant_id.clone(),
                            tenant.state.clone(),
                            tenant.home_worker.clone(),
                            tenant.shard_count.to_string(),
                        ]
                    })
                    .collect::<Vec<_>>(),
            ),
            TuiPanel::Branches => terminal_table(
                &["branch", "state", "lsn"],
                &self
                    .snapshot
                    .branches
                    .iter()
                    .map(|branch| {
                        vec![
                            branch.name.clone(),
                            branch.state.clone(),
                            branch.lsn.clone(),
                        ]
                    })
                    .collect::<Vec<_>>(),
            ),
        };

        Ok(format!(
            "citus-tui | cluster={} | panel={} | safe_mode={} | generated_at={}\n{}",
            self.snapshot.cluster_name,
            panel.as_name(),
            self.session.safe_mode,
            self.snapshot.generated_at,
            table
        ))
    }

    pub fn preview_action(
        &self,
        action: TuiAction,
        unsafe_allow_mutation: bool,
        confirmation: Option<&str>,
    ) -> Result<TuiActionReceipt, TuiRuntimeError> {
        let mut errors = Vec::new();
        action.validate(&mut errors);
        if !errors.is_empty() {
            return Err(TuiRuntimeError::ActionRejected(errors.join("; ")));
        }
        if action.is_destructive() && self.session.safe_mode && !unsafe_allow_mutation {
            return Err(TuiRuntimeError::ActionRejected(format!(
                "safe_mode blocks {}",
                action.name()
            )));
        }
        if action.is_destructive() && confirmation != Some("CONFIRM") {
            return Err(TuiRuntimeError::ActionRejected(format!(
                "{} requires CONFIRM",
                action.name()
            )));
        }

        match &action {
            TuiAction::RebalanceDryRun { shard_id } | TuiAction::RebalanceApply { shard_id } => {
                if !self.snapshot.has_shard(*shard_id) {
                    return Err(TuiRuntimeError::ActionRejected(format!(
                        "unknown shard_id {shard_id}"
                    )));
                }
            }
            TuiAction::TenantMove {
                tenant,
                target_worker,
            } => {
                if !self.snapshot.has_tenant(tenant) {
                    return Err(TuiRuntimeError::ActionRejected(format!(
                        "unknown tenant {tenant}"
                    )));
                }
                if !self.snapshot.has_worker(target_worker) {
                    return Err(TuiRuntimeError::ActionRejected(format!(
                        "unknown worker {target_worker}"
                    )));
                }
            }
            TuiAction::BranchPromote { branch } => {
                if !self.snapshot.has_branch(branch) {
                    return Err(TuiRuntimeError::ActionRejected(format!(
                        "unknown branch {branch}"
                    )));
                }
            }
            TuiAction::ExplainQuery { .. } => {}
        }

        Ok(TuiActionReceipt {
            action: action.name().to_string(),
            status: "accepted".to_string(),
            detail: "validated preview".to_string(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiActionReceipt {
    pub action: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TuiRuntimeError {
    ActionRejected(String),
    InvalidSession(String),
    InvalidSnapshot(String),
    UnknownPanel(String),
}

impl From<ToolRuntimeError> for TuiRuntimeError {
    fn from(error: ToolRuntimeError) -> Self {
        Self::InvalidSnapshot(error.to_string())
    }
}

impl fmt::Display for TuiRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActionRejected(detail) => write!(formatter, "TUI action rejected: {detail}"),
            Self::InvalidSession(detail) => write!(formatter, "TUI session invalid: {detail}"),
            Self::InvalidSnapshot(detail) => write!(formatter, "TUI snapshot invalid: {detail}"),
            Self::UnknownPanel(panel) => write!(formatter, "unknown TUI panel: {panel}"),
        }
    }
}

impl Error for TuiRuntimeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_blaise_citus_tool_runtime::canonical_snapshot;

    #[test]
    fn tui_session_covers_required_panels() {
        let plan = canonical_tui_session();

        assert!(plan.covers_required_panels());
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn tui_safe_mode_blocks_mutations() {
        let plan = TuiSessionPlan::new(
            "prod",
            vec![TuiPanel::Rebalance],
            vec![TuiAction::RebalanceApply { shard_id: 7 }],
        );

        let errors = plan.validate().expect_err("safe mode must reject apply");

        assert!(errors.iter().any(|error| error.contains("safe_mode")));
    }

    #[test]
    fn tui_runtime_renders_shard_panel() {
        let runtime = TuiRuntime::new(canonical_snapshot()).unwrap();

        let frame = runtime.render_panel(TuiPanel::Shards).unwrap();

        assert!(frame.contains("panel=shards"));
        assert!(frame.contains("102008"));
        assert!(frame.contains("worker-1"));
    }

    #[test]
    fn tui_runtime_safe_mode_rejects_mutation() {
        let runtime = TuiRuntime::new(canonical_snapshot()).unwrap();

        let error = runtime
            .preview_action(
                TuiAction::TenantMove {
                    tenant: "tenant-a".to_string(),
                    target_worker: "worker-2".to_string(),
                },
                false,
                None,
            )
            .expect_err("safe mode blocks tenant moves");

        assert!(error.to_string().contains("safe_mode"));
    }

    #[test]
    fn tui_runtime_allows_confirmed_mutation_preview() {
        let runtime = TuiRuntime::new(canonical_snapshot()).unwrap();

        let receipt = runtime
            .preview_action(
                TuiAction::TenantMove {
                    tenant: "tenant-a".to_string(),
                    target_worker: "worker-2".to_string(),
                },
                true,
                Some("CONFIRM"),
            )
            .unwrap();

        assert_eq!(receipt.status, "accepted");
    }
}
