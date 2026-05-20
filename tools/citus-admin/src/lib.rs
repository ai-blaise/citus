// FEATURE: D5

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminUiPlan {
    pub modules: Vec<AdminModule>,
    pub actions: Vec<AdminAction>,
}

impl AdminUiPlan {
    pub fn new(modules: Vec<AdminModule>, actions: Vec<AdminAction>) -> Self {
        Self { modules, actions }
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.modules.is_empty() {
            errors.push("at least one admin module is required".to_string());
        }

        for action in &self.actions {
            action.validate(&mut errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn routes(&self) -> Vec<AdminRoute> {
        self.modules
            .iter()
            .map(|module| AdminRoute {
                path: module.path().to_string(),
                title: module.title().to_string(),
            })
            .collect()
    }

    pub fn required_modules() -> Vec<AdminModule> {
        vec![
            AdminModule::ClusterTopology,
            AdminModule::ShardExplorer,
            AdminModule::TimescaleChunks,
            AdminModule::VectorizerDashboard,
            AdminModule::Branches,
            AdminModule::Tenants,
            AdminModule::BackupBrowser,
            AdminModule::RealtimeDebugger,
        ]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRoute {
    pub path: String,
    pub title: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminModule {
    ClusterTopology,
    ShardExplorer,
    TimescaleChunks,
    VectorizerDashboard,
    Branches,
    Tenants,
    BackupBrowser,
    RealtimeDebugger,
}

impl AdminModule {
    fn path(&self) -> &'static str {
        match self {
            Self::ClusterTopology => "/cluster/topology",
            Self::ShardExplorer => "/cluster/shards",
            Self::TimescaleChunks => "/timescale/chunks",
            Self::VectorizerDashboard => "/ai/vectorizers",
            Self::Branches => "/recovery/branches",
            Self::Tenants => "/tenants",
            Self::BackupBrowser => "/backups",
            Self::RealtimeDebugger => "/realtime/debugger",
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Self::ClusterTopology => "Cluster topology",
            Self::ShardExplorer => "Shard explorer",
            Self::TimescaleChunks => "Timescale chunks",
            Self::VectorizerDashboard => "Vectorizer dashboard",
            Self::Branches => "Branches",
            Self::Tenants => "Tenants",
            Self::BackupBrowser => "Backup browser",
            Self::RealtimeDebugger => "Realtime debugger",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminAction {
    RebalanceShard {
        shard_id: u64,
        confirmation: String,
    },
    MoveTenant {
        tenant: String,
        target_worker: String,
        confirmation: String,
    },
    SuspendBranch {
        branch: String,
        confirmation: String,
    },
    ReplayRealtimeStream {
        tenant: String,
        lsn: String,
    },
}

impl AdminAction {
    fn validate(&self, errors: &mut Vec<String>) {
        match self {
            Self::RebalanceShard {
                shard_id,
                confirmation,
            } => {
                if *shard_id == 0 {
                    errors.push("rebalance shard_id must be positive".to_string());
                }
                validate_confirmation("rebalance", confirmation, errors);
            }
            Self::MoveTenant {
                tenant,
                target_worker,
                confirmation,
            } => {
                if tenant.trim().is_empty() {
                    errors.push("move tenant is required".to_string());
                }
                if target_worker.trim().is_empty() {
                    errors.push("move target_worker is required".to_string());
                }
                validate_confirmation("move", confirmation, errors);
            }
            Self::SuspendBranch {
                branch,
                confirmation,
            } => {
                if branch.trim().is_empty() {
                    errors.push("suspend branch is required".to_string());
                }
                validate_confirmation("suspend", confirmation, errors);
            }
            Self::ReplayRealtimeStream { tenant, lsn } => {
                if tenant.trim().is_empty() {
                    errors.push("replay tenant is required".to_string());
                }
                if lsn.trim().is_empty() {
                    errors.push("replay lsn is required".to_string());
                }
            }
        }
    }
}

fn validate_confirmation(action: &str, confirmation: &str, errors: &mut Vec<String>) {
    if confirmation != "CONFIRM" {
        errors.push(format!("{action} requires CONFIRM"));
    }
}

pub fn canonical_admin_plan() -> AdminUiPlan {
    AdminUiPlan::new(
        AdminUiPlan::required_modules(),
        vec![AdminAction::ReplayRealtimeStream {
            tenant: "tenant-a".to_string(),
            lsn: "0/16B6C50".to_string(),
        }],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_plan_renders_core_routes() {
        let plan = canonical_admin_plan();
        let routes = plan.routes();

        assert_eq!(routes.len(), AdminUiPlan::required_modules().len());
        assert!(routes.iter().any(|route| route.path == "/cluster/topology"));
        assert!(routes.iter().any(|route| route.path == "/ai/vectorizers"));
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn admin_rejects_unconfirmed_destructive_action() {
        let plan = AdminUiPlan::new(
            vec![AdminModule::ShardExplorer],
            vec![AdminAction::RebalanceShard {
                shard_id: 42,
                confirmation: "apply".to_string(),
            }],
        );

        let errors = plan.validate().expect_err("confirmation must be exact");

        assert!(errors.iter().any(|error| error.contains("CONFIRM")));
    }
}
