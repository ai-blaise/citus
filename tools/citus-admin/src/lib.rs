// FEATURE: D5

use ai_blaise_citus_tool_runtime::{escape_html, ToolRuntimeError, ToolSnapshot};
use std::error::Error;
use std::fmt;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRuntime {
    snapshot: ToolSnapshot,
    plan: AdminUiPlan,
}

impl AdminRuntime {
    pub fn new(snapshot: ToolSnapshot) -> Result<Self, AdminRuntimeError> {
        snapshot.validate().map_err(AdminRuntimeError::from)?;
        let plan = canonical_admin_plan();
        if let Err(errors) = plan.validate() {
            return Err(AdminRuntimeError::InvalidPlan(errors.join("; ")));
        }
        Ok(Self { snapshot, plan })
    }

    pub fn render_route(&self, route: &str) -> Result<String, AdminRuntimeError> {
        let title = self
            .plan
            .routes()
            .into_iter()
            .find(|candidate| candidate.path == route)
            .map(|candidate| candidate.title)
            .ok_or_else(|| AdminRuntimeError::UnknownRoute(route.to_string()))?;

        let body = match route {
            "/cluster/topology" => self.render_topology(),
            "/cluster/shards" => self.render_shards(),
            "/timescale/chunks" => self.render_hypertables(),
            "/ai/vectorizers" => self.render_vectorizers(),
            "/recovery/branches" => self.render_branches(),
            "/tenants" => self.render_tenants(),
            "/backups" => self.render_backups(),
            "/realtime/debugger" => self.render_realtime(),
            _ => return Err(AdminRuntimeError::UnknownRoute(route.to_string())),
        };

        Ok(self.render_layout(route, &title, &body))
    }

    pub fn execute_action(
        &self,
        action: AdminRuntimeAction,
    ) -> Result<AdminActionReceipt, AdminRuntimeError> {
        match action {
            AdminRuntimeAction::RebalanceShard {
                shard_id,
                confirmation,
            } => {
                require_confirmation("rebalance-shard", &confirmation)?;
                if !self.snapshot.has_shard(shard_id) {
                    return Err(AdminRuntimeError::ActionRejected(format!(
                        "unknown shard_id {shard_id}"
                    )));
                }
                Ok(AdminActionReceipt::accepted(
                    "rebalance-shard",
                    format!("validated dry-run for shard {shard_id}"),
                ))
            }
            AdminRuntimeAction::MoveTenant {
                tenant,
                target_worker,
                confirmation,
            } => {
                require_confirmation("move-tenant", &confirmation)?;
                if !self.snapshot.has_tenant(&tenant) {
                    return Err(AdminRuntimeError::ActionRejected(format!(
                        "unknown tenant {tenant}"
                    )));
                }
                if !self.snapshot.has_worker(&target_worker) {
                    return Err(AdminRuntimeError::ActionRejected(format!(
                        "unknown worker {target_worker}"
                    )));
                }
                Ok(AdminActionReceipt::accepted(
                    "move-tenant",
                    format!("validated dry-run for {tenant} -> {target_worker}"),
                ))
            }
            AdminRuntimeAction::SuspendBranch {
                branch,
                confirmation,
            } => {
                require_confirmation("suspend-branch", &confirmation)?;
                if !self.snapshot.has_branch(&branch) {
                    return Err(AdminRuntimeError::ActionRejected(format!(
                        "unknown branch {branch}"
                    )));
                }
                Ok(AdminActionReceipt::accepted(
                    "suspend-branch",
                    format!("validated dry-run for {branch}"),
                ))
            }
            AdminRuntimeAction::ReplayRealtimeStream { tenant, lsn } => {
                if !self.snapshot.has_tenant(&tenant) {
                    return Err(AdminRuntimeError::ActionRejected(format!(
                        "unknown tenant {tenant}"
                    )));
                }
                if lsn.trim().is_empty() {
                    return Err(AdminRuntimeError::ActionRejected(
                        "replay lsn is required".to_string(),
                    ));
                }
                Ok(AdminActionReceipt::accepted(
                    "replay-realtime-stream",
                    format!("validated replay for {tenant} from {lsn}"),
                ))
            }
        }
    }

    fn render_layout(&self, route: &str, title: &str, body: &str) -> String {
        let mut nav = String::new();
        for item in self.plan.routes() {
            nav.push_str(&format!(
                "<a href=\"{}\" data-active=\"{}\">{}</a>",
                escape_html(&item.path),
                item.path == route,
                escape_html(&item.title),
            ));
        }

        format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title></head>\
             <body data-tool=\"citus-admin\" data-cluster=\"{}\">\
             <nav>{}</nav><main><section data-route=\"{}\"><h1>{}</h1>\
             <p data-generated-at=\"{}\">snapshot generated at {}</p>{}\
             </section></main></body></html>",
            escape_html(title),
            escape_html(&self.snapshot.cluster_name),
            nav,
            escape_html(route),
            escape_html(title),
            escape_html(&self.snapshot.generated_at),
            escape_html(&self.snapshot.generated_at),
            body,
        )
    }

    fn render_topology(&self) -> String {
        let rows = self
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
            .collect::<Vec<_>>();
        let mut html = html_table(&["worker", "host", "role", "readiness"], rows);
        if let Some(pool) = &self.snapshot.pool {
            html.push_str(&format!(
                "<aside data-pool-state=\"{}\">pool {}: {} active clients, {} waiting, {} upstream errors</aside>",
                escape_html(&pool.state),
                escape_html(&pool.state),
                pool.active_clients,
                pool.waiting_clients,
                pool.upstream_errors,
            ));
        }
        html
    }

    fn render_shards(&self) -> String {
        html_table(
            &["table", "shard", "worker", "state", "bytes"],
            self.snapshot
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
                .collect(),
        )
    }

    fn render_hypertables(&self) -> String {
        html_table(
            &["table", "time_column", "chunk_interval", "shards"],
            self.snapshot
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
                .collect(),
        )
    }

    fn render_vectorizers(&self) -> String {
        html_table(
            &["name", "tenant", "backlog", "budget_tokens", "state"],
            self.snapshot
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
                .collect(),
        )
    }

    fn render_branches(&self) -> String {
        html_table(
            &["branch", "state", "lsn"],
            self.snapshot
                .branches
                .iter()
                .map(|branch| {
                    vec![
                        branch.name.clone(),
                        branch.state.clone(),
                        branch.lsn.clone(),
                    ]
                })
                .collect(),
        )
    }

    fn render_tenants(&self) -> String {
        html_table(
            &["tenant", "state", "home_worker", "shards"],
            self.snapshot
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
                .collect(),
        )
    }

    fn render_backups(&self) -> String {
        html_table(
            &["backup", "state", "completed_at"],
            self.snapshot
                .backups
                .iter()
                .map(|backup| {
                    vec![
                        backup.name.clone(),
                        backup.state.clone(),
                        backup.completed_at.clone(),
                    ]
                })
                .collect(),
        )
    }

    fn render_realtime(&self) -> String {
        html_table(
            &["tenant", "table", "subscribers", "confirmed_lsn"],
            self.snapshot
                .realtime_streams
                .iter()
                .map(|stream| {
                    vec![
                        stream.tenant_id.clone(),
                        stream.table.clone(),
                        stream.subscribers.to_string(),
                        stream.confirmed_lsn.clone(),
                    ]
                })
                .collect(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminRuntimeAction {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminActionReceipt {
    pub action: String,
    pub status: String,
    pub detail: String,
}

impl AdminActionReceipt {
    fn accepted(action: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            status: "accepted".to_string(),
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AdminRuntimeError {
    ActionRejected(String),
    InvalidPlan(String),
    InvalidSnapshot(String),
    UnknownRoute(String),
}

impl From<ToolRuntimeError> for AdminRuntimeError {
    fn from(error: ToolRuntimeError) -> Self {
        Self::InvalidSnapshot(error.to_string())
    }
}

impl fmt::Display for AdminRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActionRejected(detail) => write!(formatter, "admin action rejected: {detail}"),
            Self::InvalidPlan(detail) => write!(formatter, "admin plan invalid: {detail}"),
            Self::InvalidSnapshot(detail) => write!(formatter, "admin snapshot invalid: {detail}"),
            Self::UnknownRoute(route) => write!(formatter, "unknown admin route: {route}"),
        }
    }
}

impl Error for AdminRuntimeError {}

fn require_confirmation(action: &str, confirmation: &str) -> Result<(), AdminRuntimeError> {
    if confirmation != "CONFIRM" {
        return Err(AdminRuntimeError::ActionRejected(format!(
            "{action} requires CONFIRM"
        )));
    }
    Ok(())
}

fn html_table(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let mut html = String::from("<table><thead><tr>");
    for header in headers {
        html.push_str(&format!("<th>{}</th>", escape_html(header)));
    }
    html.push_str("</tr></thead><tbody>");
    for row in rows {
        html.push_str("<tr>");
        for value in row {
            html.push_str(&format!("<td>{}</td>", escape_html(&value)));
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody></table>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_blaise_citus_tool_runtime::canonical_snapshot;

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

    #[test]
    fn admin_runtime_renders_snapshot_route() {
        let runtime = AdminRuntime::new(canonical_snapshot()).unwrap();

        let html = runtime.render_route("/cluster/shards").unwrap();

        assert!(html.contains("data-tool=\"citus-admin\""));
        assert!(html.contains("data-route=\"/cluster/shards\""));
        assert!(html.contains("102008"));
        assert!(html.contains("worker-1"));
    }

    #[test]
    fn admin_runtime_rejects_unconfirmed_rebalance() {
        let runtime = AdminRuntime::new(canonical_snapshot()).unwrap();

        let error = runtime
            .execute_action(AdminRuntimeAction::RebalanceShard {
                shard_id: 102_008,
                confirmation: "apply".to_string(),
            })
            .expect_err("rebalance requires exact confirmation");

        assert!(error.to_string().contains("CONFIRM"));
    }

    #[test]
    fn admin_runtime_accepts_confirmed_rebalance_dry_run() {
        let runtime = AdminRuntime::new(canonical_snapshot()).unwrap();

        let receipt = runtime
            .execute_action(AdminRuntimeAction::RebalanceShard {
                shard_id: 102_008,
                confirmation: "CONFIRM".to_string(),
            })
            .unwrap();

        assert_eq!(receipt.status, "accepted");
    }
}
