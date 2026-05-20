// FEATURE: D3

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
    fn name(&self) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
