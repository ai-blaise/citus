//! Operator-facing command contracts for citusctl.

// FEATURE: D1
// FEATURE: D2
// FEATURE: M8
// FEATURE: B3
// FEATURE: B5
// FEATURE: WF2

use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const K8S_MANIFEST_EVIDENCE_BOUNDARY: &str = "live-kubernetes-manifest-apply";
const TIME_TRAVEL_EVIDENCE_BOUNDARY: &str = "time-travel-intent-validation-only";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CitusCtlRequest {
    pub intent: ExecutionIntent,
    pub command: CitusCtlCommand,
}

impl CitusCtlRequest {
    pub fn plan(&self) -> Result<CitusCtlPlan, CitusCtlError> {
        self.intent.validate()?;
        self.command.validate()?;

        let mut steps = vec![CitusCtlStep::ValidateInput, CitusCtlStep::RenderDiff];
        if self.command.requires_cluster_preflight() {
            steps.push(CitusCtlStep::RunPreflight);
        }
        if self.intent.is_apply() {
            steps.push(CitusCtlStep::Execute);
            steps.push(CitusCtlStep::WriteAuditRecord);
        }

        Ok(CitusCtlPlan {
            command_name: self.command.name(),
            destructive: self.command.is_destructive(),
            requires_plan_id: self.command.is_destructive()
                || self.command.requires_cluster_preflight(),
            steps,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExecutionIntent {
    Plan,
    Apply { plan_id: String },
}

impl ExecutionIntent {
    fn validate(&self) -> Result<(), CitusCtlError> {
        match self {
            Self::Plan => Ok(()),
            Self::Apply { plan_id } => validate_plan_id(plan_id),
        }
    }

    fn is_apply(&self) -> bool {
        matches!(self, Self::Apply { .. })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CitusCtlCommand {
    Init {
        cluster: String,
    },
    Apply {
        manifest_path: String,
    },
    Upgrade {
        target_version: String,
    },
    Dev {
        action: DevAction,
    },
    Bench {
        profile: String,
    },
    Inspect {
        target: InspectTarget,
    },
    Vectorizer {
        action: NamedAction,
    },
    Branch {
        action: NamedAction,
    },
    Migrate {
        manifest_path: String,
    },
    Dump {
        target: String,
    },
    Restore {
        source_uri: String,
    },
    RestorePitr {
        cluster: String,
        target_time: String,
    },
    Tenant {
        action: NamedAction,
    },
    Webhook {
        action: NamedAction,
    },
    Function {
        action: NamedAction,
    },
    Search {
        action: NamedAction,
    },
    Backup {
        action: NamedAction,
    },
    TimeTravel {
        target_time: String,
    },
    WalReplay {
        source_uri: String,
        target_time: String,
    },
    NewFeature {
        feature_id: String,
    },
}

impl CitusCtlCommand {
    fn validate(&self) -> Result<(), CitusCtlError> {
        match self {
            Self::Init { cluster } => validate_required("cluster", cluster),
            Self::Apply { manifest_path } | Self::Migrate { manifest_path } => {
                validate_required("manifest_path", manifest_path)
            }
            Self::Upgrade { target_version } => validate_required("target_version", target_version),
            Self::Dev { .. } => Ok(()),
            Self::Bench { profile } => validate_required("profile", profile),
            Self::Inspect { .. } => Ok(()),
            Self::Vectorizer { action }
            | Self::Branch { action }
            | Self::Tenant { action }
            | Self::Webhook { action }
            | Self::Function { action }
            | Self::Search { action }
            | Self::Backup { action } => action.validate(),
            Self::Dump { target } => validate_required("target", target),
            Self::Restore { source_uri } => validate_required("source_uri", source_uri),
            Self::RestorePitr {
                cluster,
                target_time,
            } => {
                validate_required("cluster", cluster)?;
                validate_timestamp(target_time)
            }
            Self::TimeTravel { target_time } => validate_timestamp(target_time),
            Self::WalReplay {
                source_uri,
                target_time,
            } => {
                validate_required("source_uri", source_uri)?;
                validate_timestamp(target_time)
            }
            Self::NewFeature { feature_id } => validate_feature_id(feature_id),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Init { .. } => "init",
            Self::Apply { .. } => "apply",
            Self::Upgrade { .. } => "upgrade",
            Self::Dev { .. } => "dev",
            Self::Bench { .. } => "bench",
            Self::Inspect { .. } => "inspect",
            Self::Vectorizer { .. } => "vectorizer",
            Self::Branch { .. } => "branch",
            Self::Migrate { .. } => "migrate",
            Self::Dump { .. } => "dump",
            Self::Restore { .. } => "restore",
            Self::RestorePitr { .. } => "restore-pitr",
            Self::Tenant { .. } => "tenant",
            Self::Webhook { .. } => "webhook",
            Self::Function { .. } => "function",
            Self::Search { .. } => "search",
            Self::Backup { .. } => "backup",
            Self::TimeTravel { .. } => "time-travel",
            Self::WalReplay { .. } => "wal-replay",
            Self::NewFeature { .. } => "new-feature",
        }
    }

    fn is_destructive(&self) -> bool {
        matches!(
            self,
            Self::Apply { .. }
                | Self::Upgrade { .. }
                | Self::Migrate { .. }
                | Self::Restore { .. }
                | Self::RestorePitr { .. }
                | Self::WalReplay { .. }
                | Self::Tenant {
                    action: NamedAction {
                        verb: ActionVerb::Delete | ActionVerb::Archive | ActionVerb::Move,
                        ..
                    }
                }
                | Self::Branch {
                    action: NamedAction {
                        verb: ActionVerb::Promote | ActionVerb::Delete,
                        ..
                    }
                }
        )
    }

    fn requires_cluster_preflight(&self) -> bool {
        !matches!(self, Self::NewFeature { .. })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DevAction {
    Up,
    Down,
}

impl DevAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum InspectTarget {
    Cluster,
    Shards,
    Hypertables,
    Tenants,
    Branches,
    Backups,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NamedAction {
    pub verb: ActionVerb,
    pub name: String,
}

impl NamedAction {
    fn validate(&self) -> Result<(), CitusCtlError> {
        validate_required("name", &self.name)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ActionVerb {
    Create,
    Apply,
    Delete,
    Archive,
    Move,
    Promote,
    Suspend,
    Resume,
    Inspect,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DevLifecycleRuntime {
    cluster_name: String,
    state_dir: PathBuf,
}

impl DevLifecycleRuntime {
    pub fn new(
        cluster_name: impl Into<String>,
        state_dir: impl Into<PathBuf>,
    ) -> Result<Self, CitusCtlError> {
        let cluster_name = cluster_name.into();
        validate_required("cluster_name", &cluster_name)?;
        let state_dir = state_dir.into();
        validate_state_dir(&state_dir)?;
        Ok(Self {
            cluster_name,
            state_dir,
        })
    }

    pub fn state_path(&self) -> PathBuf {
        self.state_dir.join("dev-lifecycle.state")
    }

    pub fn audit_path(&self) -> PathBuf {
        self.state_dir.join("dev-lifecycle.audit.tsv")
    }

    pub fn plan(&self, action: DevAction) -> Result<DevLifecyclePlan, CitusCtlError> {
        self.build_plan(action, None, true)
    }

    pub fn apply(
        &self,
        action: DevAction,
        plan_id: impl Into<String>,
    ) -> Result<DevLifecycleReport, CitusCtlError> {
        let plan_id = plan_id.into();
        validate_plan_id(&plan_id)?;
        let plan = self.build_plan(action, Some(plan_id), false)?;
        let before = plan.before.status;
        let after = plan.after.status;
        let changed = before != after;
        let mut state_written = false;
        let mut state_removed = false;

        fs::create_dir_all(&self.state_dir)?;
        match action {
            DevAction::Up if changed => {
                write_dev_state(&self.state_path(), &plan.after)?;
                state_written = true;
            }
            DevAction::Down if changed => match fs::remove_file(self.state_path()) {
                Ok(()) => state_removed = true,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            },
            _ => {}
        }
        append_dev_audit_record(
            &self.audit_path(),
            &plan,
            changed,
            state_written,
            state_removed,
        )?;

        Ok(DevLifecycleReport {
            plan,
            changed,
            state_written,
            state_removed,
            audit_record_written: true,
            evidence_boundary: "local-state-file-only",
        })
    }

    fn build_plan(
        &self,
        action: DevAction,
        plan_id: Option<String>,
        dry_run: bool,
    ) -> Result<DevLifecyclePlan, CitusCtlError> {
        let before = read_dev_state(&self.state_path())?.unwrap_or_else(|| DevLifecycleState {
            cluster_name: self.cluster_name.clone(),
            status: DevClusterStatus::Absent,
            generation: 0,
            last_plan_id: None,
        });
        if before.cluster_name != self.cluster_name && before.status != DevClusterStatus::Absent {
            return Err(CitusCtlError::CorruptState(format!(
                "state belongs to cluster {}",
                before.cluster_name
            )));
        }
        let after = match action {
            DevAction::Up => {
                if before.status == DevClusterStatus::Running {
                    before.clone()
                } else {
                    DevLifecycleState {
                        cluster_name: self.cluster_name.clone(),
                        status: DevClusterStatus::Running,
                        generation: before.generation.saturating_add(1).max(1),
                        last_plan_id: plan_id.clone(),
                    }
                }
            }
            DevAction::Down => DevLifecycleState {
                cluster_name: self.cluster_name.clone(),
                status: DevClusterStatus::Absent,
                generation: before.generation,
                last_plan_id: plan_id.clone(),
            },
        };
        Ok(DevLifecyclePlan {
            action,
            plan_id,
            state_dir: self.state_dir.to_string_lossy().to_string(),
            state_path: self.state_path().to_string_lossy().to_string(),
            before,
            after,
            dry_run,
            steps: dev_lifecycle_steps(action, dry_run),
            cleanup_guard: "state-file-only-no-recursive-delete",
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DevLifecyclePlan {
    pub action: DevAction,
    pub plan_id: Option<String>,
    pub state_dir: String,
    pub state_path: String,
    pub before: DevLifecycleState,
    pub after: DevLifecycleState,
    pub dry_run: bool,
    pub steps: Vec<DevLifecycleStep>,
    pub cleanup_guard: &'static str,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DevLifecycleReport {
    pub plan: DevLifecyclePlan,
    pub changed: bool,
    pub state_written: bool,
    pub state_removed: bool,
    pub audit_record_written: bool,
    pub evidence_boundary: &'static str,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DevLifecycleState {
    pub cluster_name: String,
    pub status: DevClusterStatus,
    pub generation: u64,
    pub last_plan_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DevClusterStatus {
    Absent,
    Running,
}

impl DevClusterStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Running => "running",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DevLifecycleStep {
    ValidateStateDir,
    ReadState,
    RenderPlan,
    VerifyPlanId,
    CreateStateDir,
    WriteState,
    RemoveStateFile,
    WriteAuditRecord,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DevLifecycleCanonicalReport {
    pub state_dir: String,
    pub plan_up_steps: usize,
    pub apply_up_changed: bool,
    pub idempotent_up_changed: bool,
    pub apply_down_changed: bool,
    pub idempotent_down_changed: bool,
    pub final_state_present: bool,
    pub cleanup_guard: &'static str,
    pub evidence_boundary: &'static str,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum DevLifecycleOutputFormat {
    Tsv,
    Json,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DevLifecycleCliOptions {
    cluster_name: Option<String>,
    state_dir: Option<PathBuf>,
    format: DevLifecycleOutputFormat,
}

impl DevLifecycleCliOptions {
    fn parse(args: &[String]) -> Result<Self, CitusCtlError> {
        let mut cluster_name = None;
        let mut state_dir = None;
        let mut format = DevLifecycleOutputFormat::Tsv;
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--cluster" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(CitusCtlError::MissingRequiredField("cluster_name"));
                    };
                    validate_required("cluster_name", value)?;
                    cluster_name = Some(value.clone());
                    index += 2;
                }
                "--state-dir" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(CitusCtlError::MissingRequiredField("state_dir"));
                    };
                    state_dir = Some(PathBuf::from(value));
                    index += 2;
                }
                "--format" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(CitusCtlError::MissingRequiredField("format"));
                    };
                    format = match value.as_str() {
                        "tsv" => DevLifecycleOutputFormat::Tsv,
                        "json" => DevLifecycleOutputFormat::Json,
                        other => {
                            return Err(CitusCtlError::UnknownValue {
                                field: "format",
                                value: other.to_string(),
                            })
                        }
                    };
                    index += 2;
                }
                value if value.starts_with("--") => {
                    return Err(CitusCtlError::UnknownValue {
                        field: "dev_lifecycle_option",
                        value: value.to_string(),
                    })
                }
                value => {
                    return Err(CitusCtlError::UnknownValue {
                        field: "argument",
                        value: value.to_string(),
                    })
                }
            }
        }

        Ok(Self {
            cluster_name,
            state_dir,
            format,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DevLifecycleCliReport {
    mode: &'static str,
    cluster_name: String,
    action: DevAction,
    state_path: String,
    plan_id: Option<String>,
    dry_run: bool,
    changed: bool,
    state_written: bool,
    state_removed: bool,
    audit_record_written: bool,
    before_status: DevClusterStatus,
    after_status: DevClusterStatus,
    before_generation: u64,
    after_generation: u64,
    steps: usize,
    cleanup_guard: &'static str,
    evidence_boundary: &'static str,
}

impl DevLifecycleCliReport {
    fn from_plan(plan: &DevLifecyclePlan) -> Self {
        Self {
            mode: "plan",
            cluster_name: plan.after.cluster_name.clone(),
            action: plan.action,
            state_path: plan.state_path.clone(),
            plan_id: plan.plan_id.clone(),
            dry_run: plan.dry_run,
            changed: plan.before.status != plan.after.status,
            state_written: false,
            state_removed: false,
            audit_record_written: false,
            before_status: plan.before.status,
            after_status: plan.after.status,
            before_generation: plan.before.generation,
            after_generation: plan.after.generation,
            steps: plan.steps.len(),
            cleanup_guard: plan.cleanup_guard,
            evidence_boundary: "local-state-file-only",
        }
    }

    fn from_apply(report: &DevLifecycleReport) -> Self {
        Self {
            mode: "apply",
            cluster_name: report.plan.after.cluster_name.clone(),
            action: report.plan.action,
            state_path: report.plan.state_path.clone(),
            plan_id: report.plan.plan_id.clone(),
            dry_run: report.plan.dry_run,
            changed: report.changed,
            state_written: report.state_written,
            state_removed: report.state_removed,
            audit_record_written: report.audit_record_written,
            before_status: report.plan.before.status,
            after_status: report.plan.after.status,
            before_generation: report.plan.before.generation,
            after_generation: report.plan.after.generation,
            steps: report.plan.steps.len(),
            cleanup_guard: report.plan.cleanup_guard,
            evidence_boundary: report.evidence_boundary,
        }
    }

    fn tsv_header() -> &'static str {
        "mode\tcluster\taction\tstate_path\tplan_id\tdry_run\tchanged\tstate_written\tstate_removed\taudit_record_written\tbefore_status\tafter_status\tbefore_generation\tafter_generation\tsteps\tcleanup_guard\tevidence_boundary"
    }

    fn to_tsv(&self) -> String {
        format!(
            "{}\n{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            Self::tsv_header(),
            self.mode,
            self.cluster_name,
            self.action.as_str(),
            self.state_path,
            self.plan_id.as_deref().unwrap_or(""),
            self.dry_run,
            self.changed,
            self.state_written,
            self.state_removed,
            self.audit_record_written,
            self.before_status.as_str(),
            self.after_status.as_str(),
            self.before_generation,
            self.after_generation,
            self.steps,
            self.cleanup_guard,
            self.evidence_boundary
        )
    }

    fn to_json(&self) -> String {
        let plan_id = self
            .plan_id
            .as_deref()
            .map(|value| format!("\"{}\"", json_escape(value)))
            .unwrap_or_else(|| "null".to_string());
        format!(
            "{{\"action\":\"{}\",\"after_generation\":{},\"after_status\":\"{}\",\"audit_record_written\":{},\"before_generation\":{},\"before_status\":\"{}\",\"changed\":{},\"cleanup_guard\":\"{}\",\"cluster\":\"{}\",\"dry_run\":{},\"evidence_boundary\":\"{}\",\"mode\":\"{}\",\"plan_id\":{},\"state_path\":\"{}\",\"state_removed\":{},\"state_written\":{},\"steps\":{}}}",
            self.action.as_str(),
            self.after_generation,
            self.after_status.as_str(),
            self.audit_record_written,
            self.before_generation,
            self.before_status.as_str(),
            self.changed,
            self.cleanup_guard,
            json_escape(&self.cluster_name),
            self.dry_run,
            self.evidence_boundary,
            self.mode,
            plan_id,
            json_escape(&self.state_path),
            self.state_removed,
            self.state_written,
            self.steps,
        )
    }

    fn render(&self, format: DevLifecycleOutputFormat) -> String {
        match format {
            DevLifecycleOutputFormat::Tsv => self.to_tsv(),
            DevLifecycleOutputFormat::Json => self.to_json(),
        }
    }
}

pub fn render_dev_lifecycle_cli_report_from_args(
    args: &[String],
) -> Result<Option<String>, CitusCtlError> {
    let Some(first) = args.first() else {
        return Ok(None);
    };
    let (plan_id, rest) = match first.as_str() {
        "plan" => (None, &args[1..]),
        "apply" => {
            let Some(plan_id) = args.get(1) else {
                return Ok(None);
            };
            (Some(plan_id.clone()), &args[2..])
        }
        _ => return Ok(None),
    };

    if rest.first().map(String::as_str) != Some("dev") {
        return Ok(None);
    }
    let Some(action_value) = rest.get(1) else {
        return Ok(None);
    };
    let option_args = &rest[2..];
    if !option_args.iter().any(|arg| arg.starts_with("--")) {
        return Ok(None);
    }

    let action = parse_dev_action(action_value)?;
    let options = DevLifecycleCliOptions::parse(option_args)?;
    let state_dir = options
        .state_dir
        .ok_or(CitusCtlError::MissingRequiredField("state_dir"))?;
    let cluster_name = options
        .cluster_name
        .unwrap_or_else(|| "dev-citus".to_string());
    let runtime = DevLifecycleRuntime::new(cluster_name, state_dir)?;
    let report = match plan_id {
        None => DevLifecycleCliReport::from_plan(&runtime.plan(action)?),
        Some(plan_id) => DevLifecycleCliReport::from_apply(&runtime.apply(action, plan_id)?),
    };

    Ok(Some(report.render(options.format)))
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct K8sManifestRuntime {
    manifest_path: PathBuf,
    namespace: String,
    state_dir: PathBuf,
    kubectl: String,
    context: Option<String>,
}

impl K8sManifestRuntime {
    pub fn new(
        manifest_path: impl Into<PathBuf>,
        namespace: impl Into<String>,
        state_dir: impl Into<PathBuf>,
    ) -> Result<Self, CitusCtlError> {
        let namespace = namespace.into();
        validate_k8s_namespace(&namespace)?;
        let state_dir = state_dir.into();
        validate_state_dir(&state_dir)?;
        Ok(Self {
            manifest_path: manifest_path.into(),
            namespace,
            state_dir,
            kubectl: "kubectl".to_string(),
            context: None,
        })
    }

    fn with_kubectl(mut self, kubectl: impl Into<String>) -> Result<Self, CitusCtlError> {
        let kubectl = kubectl.into();
        validate_required("kubectl", &kubectl)?;
        self.kubectl = kubectl;
        Ok(self)
    }

    fn with_context(mut self, context: Option<String>) -> Result<Self, CitusCtlError> {
        if let Some(context) = &context {
            validate_required("context", context)?;
        }
        self.context = context;
        Ok(self)
    }

    pub fn audit_path(&self) -> PathBuf {
        self.state_dir.join("k8s-manifest-apply.audit.tsv")
    }

    pub fn plan(&self) -> Result<K8sManifestPlan, CitusCtlError> {
        let manifest = K8sManifest::load(&self.manifest_path)?;
        let plan_id = k8s_manifest_plan_id(&manifest, &self.namespace);
        let resources = self.kubectl_apply_server_dry_run()?;
        Ok(K8sManifestPlan {
            plan_id,
            manifest_path: self.manifest_path.to_string_lossy().to_string(),
            manifest_hash: manifest.content_hash,
            namespace: self.namespace.clone(),
            dry_run: true,
            resources,
            steps: vec![
                K8sManifestStep::ValidateManifest,
                K8sManifestStep::RenderPlan,
                K8sManifestStep::RunServerDryRun,
            ],
            evidence_boundary: K8S_MANIFEST_EVIDENCE_BOUNDARY,
        })
    }

    pub fn apply(&self, plan_id: impl Into<String>) -> Result<K8sManifestReport, CitusCtlError> {
        let plan_id = plan_id.into();
        validate_plan_id(&plan_id)?;
        let plan = self.plan()?;
        if plan.plan_id != plan_id {
            return Err(CitusCtlError::PlanIdMismatch);
        }

        let apply_output = self.kubectl_apply_live()?;
        let resources = self.kubectl_get_manifest()?;
        let changed = apply_output.lines().any(|line| {
            line.ends_with(" created")
                || line.ends_with(" configured")
                || line.ends_with(" patched")
        });
        fs::create_dir_all(&self.state_dir)?;
        append_k8s_manifest_audit_record(&self.audit_path(), &plan, changed, &resources)?;

        Ok(K8sManifestReport {
            plan: K8sManifestPlan { resources, ..plan },
            changed,
            applied: true,
            audit_record_written: true,
            evidence_boundary: K8S_MANIFEST_EVIDENCE_BOUNDARY,
        })
    }

    fn kubectl_apply_server_dry_run(&self) -> Result<Vec<String>, CitusCtlError> {
        let output = self.run_kubectl(&[
            "apply".to_string(),
            "-f".to_string(),
            self.manifest_path.to_string_lossy().to_string(),
            "-n".to_string(),
            self.namespace.clone(),
            "--dry-run=server".to_string(),
            "-o".to_string(),
            "name".to_string(),
        ])?;
        parse_kubectl_resources(&output)
    }

    fn kubectl_apply_live(&self) -> Result<String, CitusCtlError> {
        self.run_kubectl(&[
            "apply".to_string(),
            "-f".to_string(),
            self.manifest_path.to_string_lossy().to_string(),
            "-n".to_string(),
            self.namespace.clone(),
        ])
    }

    fn kubectl_get_manifest(&self) -> Result<Vec<String>, CitusCtlError> {
        let output = self.run_kubectl(&[
            "get".to_string(),
            "-f".to_string(),
            self.manifest_path.to_string_lossy().to_string(),
            "-n".to_string(),
            self.namespace.clone(),
            "-o".to_string(),
            "name".to_string(),
        ])?;
        parse_kubectl_resources(&output)
    }

    fn run_kubectl(&self, arguments: &[String]) -> Result<String, CitusCtlError> {
        let mut command = Command::new(&self.kubectl);
        if let Some(context) = &self.context {
            command.arg("--context").arg(context);
        }
        let output = command.args(arguments).output().map_err(|error| {
            CitusCtlError::KubernetesCommand(format!(
                "kubectl {} spawn failed: {}",
                arguments.join(" "),
                error
            ))
        })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(CitusCtlError::KubernetesCommand(format!(
                "kubectl {} failed: {}",
                arguments.join(" "),
                stderr
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct K8sManifestPlan {
    pub plan_id: String,
    pub manifest_path: String,
    pub manifest_hash: String,
    pub namespace: String,
    pub dry_run: bool,
    pub resources: Vec<String>,
    pub steps: Vec<K8sManifestStep>,
    pub evidence_boundary: &'static str,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct K8sManifestReport {
    pub plan: K8sManifestPlan,
    pub changed: bool,
    pub applied: bool,
    pub audit_record_written: bool,
    pub evidence_boundary: &'static str,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum K8sManifestStep {
    ValidateManifest,
    RenderPlan,
    RunServerDryRun,
    ExecuteKubectlApply,
    VerifyResources,
    WriteAuditRecord,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum K8sManifestOutputFormat {
    Tsv,
    Json,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct K8sManifestCliOptions {
    namespace: Option<String>,
    state_dir: Option<PathBuf>,
    format: K8sManifestOutputFormat,
    kubectl: String,
    context: Option<String>,
}

impl K8sManifestCliOptions {
    fn parse(args: &[String]) -> Result<Self, CitusCtlError> {
        let mut namespace = None;
        let mut state_dir = None;
        let mut format = K8sManifestOutputFormat::Tsv;
        let mut kubectl = "kubectl".to_string();
        let mut context = None;
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--namespace" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(CitusCtlError::MissingRequiredField("namespace"));
                    };
                    validate_k8s_namespace(value)?;
                    namespace = Some(value.clone());
                    index += 2;
                }
                "--state-dir" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(CitusCtlError::MissingRequiredField("state_dir"));
                    };
                    state_dir = Some(PathBuf::from(value));
                    index += 2;
                }
                "--format" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(CitusCtlError::MissingRequiredField("format"));
                    };
                    format = match value.as_str() {
                        "tsv" => K8sManifestOutputFormat::Tsv,
                        "json" => K8sManifestOutputFormat::Json,
                        other => {
                            return Err(CitusCtlError::UnknownValue {
                                field: "format",
                                value: other.to_string(),
                            })
                        }
                    };
                    index += 2;
                }
                "--kubectl" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(CitusCtlError::MissingRequiredField("kubectl"));
                    };
                    validate_required("kubectl", value)?;
                    kubectl = value.clone();
                    index += 2;
                }
                "--context" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(CitusCtlError::MissingRequiredField("context"));
                    };
                    validate_required("context", value)?;
                    context = Some(value.clone());
                    index += 2;
                }
                value if value.starts_with("--") => {
                    return Err(CitusCtlError::UnknownValue {
                        field: "k8s_manifest_option",
                        value: value.to_string(),
                    })
                }
                value => {
                    return Err(CitusCtlError::UnknownValue {
                        field: "argument",
                        value: value.to_string(),
                    })
                }
            }
        }

        Ok(Self {
            namespace,
            state_dir,
            format,
            kubectl,
            context,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct K8sManifestCliReport {
    mode: &'static str,
    plan_id: String,
    manifest_path: String,
    manifest_hash: String,
    namespace: String,
    dry_run: bool,
    changed: bool,
    applied: bool,
    audit_record_written: bool,
    resources: Vec<String>,
    steps: usize,
    evidence_boundary: &'static str,
}

impl K8sManifestCliReport {
    fn from_plan(plan: K8sManifestPlan) -> Self {
        Self {
            mode: "plan",
            plan_id: plan.plan_id,
            manifest_path: plan.manifest_path,
            manifest_hash: plan.manifest_hash,
            namespace: plan.namespace,
            dry_run: plan.dry_run,
            changed: false,
            applied: false,
            audit_record_written: false,
            resources: plan.resources,
            steps: plan.steps.len(),
            evidence_boundary: plan.evidence_boundary,
        }
    }

    fn from_apply(report: K8sManifestReport) -> Self {
        Self {
            mode: "apply",
            plan_id: report.plan.plan_id,
            manifest_path: report.plan.manifest_path,
            manifest_hash: report.plan.manifest_hash,
            namespace: report.plan.namespace,
            dry_run: false,
            changed: report.changed,
            applied: report.applied,
            audit_record_written: report.audit_record_written,
            resources: report.plan.resources,
            steps: 6,
            evidence_boundary: report.evidence_boundary,
        }
    }

    fn tsv_header() -> &'static str {
        "mode\tplan_id\tmanifest_path\tmanifest_hash\tnamespace\tdry_run\tchanged\tapplied\taudit_record_written\tresources\tsteps\tevidence_boundary"
    }

    fn to_tsv(&self) -> String {
        format!(
            "{}\n{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            Self::tsv_header(),
            self.mode,
            self.plan_id,
            self.manifest_path,
            self.manifest_hash,
            self.namespace,
            self.dry_run,
            self.changed,
            self.applied,
            self.audit_record_written,
            self.resources.join(","),
            self.steps,
            self.evidence_boundary,
        )
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"applied\":{},\"audit_record_written\":{},\"changed\":{},\"dry_run\":{},\"evidence_boundary\":\"{}\",\"manifest_hash\":\"{}\",\"manifest_path\":\"{}\",\"mode\":\"{}\",\"namespace\":\"{}\",\"plan_id\":\"{}\",\"resources\":{},\"steps\":{}}}",
            self.applied,
            self.audit_record_written,
            self.changed,
            self.dry_run,
            self.evidence_boundary,
            json_escape(&self.manifest_hash),
            json_escape(&self.manifest_path),
            self.mode,
            json_escape(&self.namespace),
            json_escape(&self.plan_id),
            json_string_array(&self.resources),
            self.steps,
        )
    }

    fn render(&self, format: K8sManifestOutputFormat) -> String {
        match format {
            K8sManifestOutputFormat::Tsv => self.to_tsv(),
            K8sManifestOutputFormat::Json => self.to_json(),
        }
    }
}

pub fn render_k8s_manifest_cli_report_from_args(
    args: &[String],
) -> Result<Option<String>, CitusCtlError> {
    let Some(first) = args.first() else {
        return Ok(None);
    };
    let (apply_plan_id, rest) = match first.as_str() {
        "plan" => (None, &args[1..]),
        "apply" => {
            let Some(plan_id) = args.get(1) else {
                return Ok(None);
            };
            (Some(plan_id.clone()), &args[2..])
        }
        _ => return Ok(None),
    };

    if rest.first().map(String::as_str) != Some("apply") {
        return Ok(None);
    }
    let Some(manifest_path) = rest.get(1) else {
        return Ok(None);
    };
    let option_args = &rest[2..];
    if !option_args.iter().any(|arg| arg.starts_with("--")) {
        return Ok(None);
    }

    let options = K8sManifestCliOptions::parse(option_args)?;
    let namespace = options
        .namespace
        .ok_or(CitusCtlError::MissingRequiredField("namespace"))?;
    let state_dir = options
        .state_dir
        .ok_or(CitusCtlError::MissingRequiredField("state_dir"))?;
    let runtime = K8sManifestRuntime::new(manifest_path, namespace, state_dir)?
        .with_kubectl(options.kubectl)?
        .with_context(options.context)?;

    let report = match apply_plan_id {
        None => K8sManifestCliReport::from_plan(runtime.plan()?),
        Some(plan_id) => K8sManifestCliReport::from_apply(runtime.apply(plan_id)?),
    };

    Ok(Some(report.render(options.format)))
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TimeTravelIntentRuntime {
    target_time: String,
    now: String,
    max_staleness_seconds: u64,
    state_dir: PathBuf,
}

impl TimeTravelIntentRuntime {
    pub fn new(
        target_time: impl Into<String>,
        now: impl Into<String>,
        max_staleness_seconds: u64,
        state_dir: impl Into<PathBuf>,
    ) -> Result<Self, CitusCtlError> {
        let target_time = target_time.into();
        let now = now.into();
        validate_timestamp(&target_time)?;
        validate_timestamp(&now)?;
        if max_staleness_seconds == 0 {
            return Err(CitusCtlError::UnknownValue {
                field: "max_staleness_seconds",
                value: "0".to_string(),
            });
        }
        let state_dir = state_dir.into();
        validate_state_dir(&state_dir)?;
        let runtime = Self {
            target_time,
            now,
            max_staleness_seconds,
            state_dir,
        };
        runtime.validate_window()?;
        Ok(runtime)
    }

    pub fn audit_path(&self) -> PathBuf {
        self.state_dir.join("time-travel-intent.audit.tsv")
    }

    pub fn plan(&self) -> Result<TimeTravelIntentPlan, CitusCtlError> {
        let age_seconds = self.age_seconds()?;
        Ok(TimeTravelIntentPlan {
            plan_id: self.plan_id()?,
            target_time: self.target_time.clone(),
            now: self.now.clone(),
            age_seconds,
            max_staleness_seconds: self.max_staleness_seconds,
            accepted: true,
            dry_run: true,
            steps: vec![
                TimeTravelIntentStep::ValidateUtcTimestamp,
                TimeTravelIntentStep::ValidateStalenessWindow,
                TimeTravelIntentStep::RenderIntentPlan,
            ],
            evidence_boundary: TIME_TRAVEL_EVIDENCE_BOUNDARY,
        })
    }

    pub fn apply(
        &self,
        plan_id: impl Into<String>,
    ) -> Result<TimeTravelIntentReport, CitusCtlError> {
        let plan_id = plan_id.into();
        validate_plan_id(&plan_id)?;
        let plan = self.plan()?;
        if plan.plan_id != plan_id {
            return Err(CitusCtlError::TimeTravelPlanIdMismatch);
        }
        fs::create_dir_all(&self.state_dir)?;
        append_time_travel_intent_audit_record(&self.audit_path(), &plan)?;
        Ok(TimeTravelIntentReport {
            plan,
            accepted: true,
            audit_record_written: true,
            evidence_boundary: TIME_TRAVEL_EVIDENCE_BOUNDARY,
        })
    }

    fn validate_window(&self) -> Result<(), CitusCtlError> {
        let _ = self.age_seconds()?;
        Ok(())
    }

    fn age_seconds(&self) -> Result<u64, CitusCtlError> {
        let target_epoch = utc_timestamp_epoch_seconds(&self.target_time)?;
        let now_epoch = utc_timestamp_epoch_seconds(&self.now)?;
        if target_epoch > now_epoch {
            return Err(CitusCtlError::UnknownValue {
                field: "target_time",
                value: "must not be in the future".to_string(),
            });
        }
        let age_seconds = (now_epoch - target_epoch) as u64;
        if age_seconds > self.max_staleness_seconds {
            return Err(CitusCtlError::UnknownValue {
                field: "target_time",
                value: format!(
                    "older than max_staleness_seconds {}",
                    self.max_staleness_seconds
                ),
            });
        }
        Ok(age_seconds)
    }

    fn plan_id(&self) -> Result<String, CitusCtlError> {
        let material = format!(
            "{}\0{}\0{}",
            self.target_time, self.now, self.max_staleness_seconds
        );
        Ok(format!("time-travel-{:016x}", fnv1a64(material.as_bytes())))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TimeTravelIntentPlan {
    pub plan_id: String,
    pub target_time: String,
    pub now: String,
    pub age_seconds: u64,
    pub max_staleness_seconds: u64,
    pub accepted: bool,
    pub dry_run: bool,
    pub steps: Vec<TimeTravelIntentStep>,
    pub evidence_boundary: &'static str,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TimeTravelIntentReport {
    pub plan: TimeTravelIntentPlan,
    pub accepted: bool,
    pub audit_record_written: bool,
    pub evidence_boundary: &'static str,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TimeTravelIntentStep {
    ValidateUtcTimestamp,
    ValidateStalenessWindow,
    RenderIntentPlan,
    WriteAuditRecord,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum TimeTravelIntentOutputFormat {
    Tsv,
    Json,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct TimeTravelIntentCliOptions {
    now: Option<String>,
    max_staleness_seconds: Option<u64>,
    state_dir: Option<PathBuf>,
    format: TimeTravelIntentOutputFormat,
}

impl TimeTravelIntentCliOptions {
    fn parse(args: &[String]) -> Result<Self, CitusCtlError> {
        let mut now = None;
        let mut max_staleness_seconds = None;
        let mut state_dir = None;
        let mut format = TimeTravelIntentOutputFormat::Tsv;
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--now" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(CitusCtlError::MissingRequiredField("now"));
                    };
                    validate_timestamp(value)?;
                    now = Some(value.clone());
                    index += 2;
                }
                "--max-staleness-seconds" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(CitusCtlError::MissingRequiredField("max_staleness_seconds"));
                    };
                    let parsed = value.parse::<u64>().ok().filter(|value| *value > 0).ok_or(
                        CitusCtlError::UnknownValue {
                            field: "max_staleness_seconds",
                            value: value.clone(),
                        },
                    )?;
                    max_staleness_seconds = Some(parsed);
                    index += 2;
                }
                "--state-dir" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(CitusCtlError::MissingRequiredField("state_dir"));
                    };
                    state_dir = Some(PathBuf::from(value));
                    index += 2;
                }
                "--format" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(CitusCtlError::MissingRequiredField("format"));
                    };
                    format = match value.as_str() {
                        "tsv" => TimeTravelIntentOutputFormat::Tsv,
                        "json" => TimeTravelIntentOutputFormat::Json,
                        other => {
                            return Err(CitusCtlError::UnknownValue {
                                field: "format",
                                value: other.to_string(),
                            })
                        }
                    };
                    index += 2;
                }
                value if value.starts_with("--") => {
                    return Err(CitusCtlError::UnknownValue {
                        field: "time_travel_option",
                        value: value.to_string(),
                    })
                }
                value => {
                    return Err(CitusCtlError::UnknownValue {
                        field: "argument",
                        value: value.to_string(),
                    })
                }
            }
        }

        Ok(Self {
            now,
            max_staleness_seconds,
            state_dir,
            format,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct TimeTravelIntentCliReport {
    mode: &'static str,
    plan_id: String,
    target_time: String,
    now: String,
    age_seconds: u64,
    max_staleness_seconds: u64,
    accepted: bool,
    dry_run: bool,
    audit_record_written: bool,
    steps: usize,
    evidence_boundary: &'static str,
}

impl TimeTravelIntentCliReport {
    fn from_plan(plan: TimeTravelIntentPlan) -> Self {
        Self {
            mode: "plan",
            plan_id: plan.plan_id,
            target_time: plan.target_time,
            now: plan.now,
            age_seconds: plan.age_seconds,
            max_staleness_seconds: plan.max_staleness_seconds,
            accepted: plan.accepted,
            dry_run: plan.dry_run,
            audit_record_written: false,
            steps: plan.steps.len(),
            evidence_boundary: plan.evidence_boundary,
        }
    }

    fn from_apply(report: TimeTravelIntentReport) -> Self {
        Self {
            mode: "apply",
            plan_id: report.plan.plan_id,
            target_time: report.plan.target_time,
            now: report.plan.now,
            age_seconds: report.plan.age_seconds,
            max_staleness_seconds: report.plan.max_staleness_seconds,
            accepted: report.accepted,
            dry_run: false,
            audit_record_written: report.audit_record_written,
            steps: 4,
            evidence_boundary: report.evidence_boundary,
        }
    }

    fn tsv_header() -> &'static str {
        "mode\tplan_id\ttarget_time\tnow\tage_seconds\tmax_staleness_seconds\taccepted\tdry_run\taudit_record_written\tsteps\tevidence_boundary"
    }

    fn to_tsv(&self) -> String {
        format!(
            "{}\n{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            Self::tsv_header(),
            self.mode,
            self.plan_id,
            self.target_time,
            self.now,
            self.age_seconds,
            self.max_staleness_seconds,
            self.accepted,
            self.dry_run,
            self.audit_record_written,
            self.steps,
            self.evidence_boundary,
        )
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"accepted\":{},\"age_seconds\":{},\"audit_record_written\":{},\"dry_run\":{},\"evidence_boundary\":\"{}\",\"max_staleness_seconds\":{},\"mode\":\"{}\",\"now\":\"{}\",\"plan_id\":\"{}\",\"steps\":{},\"target_time\":\"{}\"}}",
            self.accepted,
            self.age_seconds,
            self.audit_record_written,
            self.dry_run,
            self.evidence_boundary,
            self.max_staleness_seconds,
            self.mode,
            json_escape(&self.now),
            json_escape(&self.plan_id),
            self.steps,
            json_escape(&self.target_time),
        )
    }

    fn render(&self, format: TimeTravelIntentOutputFormat) -> String {
        match format {
            TimeTravelIntentOutputFormat::Tsv => self.to_tsv(),
            TimeTravelIntentOutputFormat::Json => self.to_json(),
        }
    }
}

pub fn render_time_travel_intent_cli_report_from_args(
    args: &[String],
) -> Result<Option<String>, CitusCtlError> {
    let Some(first) = args.first() else {
        return Ok(None);
    };
    let (apply_plan_id, rest) = match first.as_str() {
        "plan" => (None, &args[1..]),
        "apply" => {
            let Some(plan_id) = args.get(1) else {
                return Ok(None);
            };
            (Some(plan_id.clone()), &args[2..])
        }
        _ => return Ok(None),
    };

    if rest.first().map(String::as_str) != Some("time-travel") {
        return Ok(None);
    }
    let Some(target_time) = rest.get(1) else {
        return Ok(None);
    };
    let option_args = &rest[2..];
    if !option_args.iter().any(|arg| arg.starts_with("--")) {
        return Ok(None);
    }

    let options = TimeTravelIntentCliOptions::parse(option_args)?;
    let now = options
        .now
        .ok_or(CitusCtlError::MissingRequiredField("now"))?;
    let max_staleness_seconds = options
        .max_staleness_seconds
        .ok_or(CitusCtlError::MissingRequiredField("max_staleness_seconds"))?;
    let state_dir = options
        .state_dir
        .ok_or(CitusCtlError::MissingRequiredField("state_dir"))?;
    let runtime = TimeTravelIntentRuntime::new(target_time, now, max_staleness_seconds, state_dir)?;
    let report = match apply_plan_id {
        None => TimeTravelIntentCliReport::from_plan(runtime.plan()?),
        Some(plan_id) => TimeTravelIntentCliReport::from_apply(runtime.apply(plan_id)?),
    };

    Ok(Some(report.render(options.format)))
}

pub fn canonical_dev_lifecycle_report() -> Result<DevLifecycleCanonicalReport, CitusCtlError> {
    let state_dir = std::env::temp_dir().join(format!(
        "ai-blaise-citusctl-dev-lifecycle-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&state_dir);
    let runtime = DevLifecycleRuntime::new("dev-citus", &state_dir)?;
    let plan_up = runtime.plan(DevAction::Up)?;
    let apply_up = runtime.apply(DevAction::Up, "plan-dev-up-1")?;
    let idempotent_up = runtime.apply(DevAction::Up, "plan-dev-up-2")?;
    let apply_down = runtime.apply(DevAction::Down, "plan-dev-down-1")?;
    let idempotent_down = runtime.apply(DevAction::Down, "plan-dev-down-2")?;
    let final_state_present = runtime.state_path().exists();
    let _ = fs::remove_dir_all(&state_dir);

    Ok(DevLifecycleCanonicalReport {
        state_dir: state_dir.to_string_lossy().to_string(),
        plan_up_steps: plan_up.steps.len(),
        apply_up_changed: apply_up.changed,
        idempotent_up_changed: idempotent_up.changed,
        apply_down_changed: apply_down.changed,
        idempotent_down_changed: idempotent_down.changed,
        final_state_present,
        cleanup_guard: apply_down.plan.cleanup_guard,
        evidence_boundary: apply_down.evidence_boundary,
    })
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CitusCtlPlan {
    pub command_name: &'static str,
    pub destructive: bool,
    pub requires_plan_id: bool,
    pub steps: Vec<CitusCtlStep>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CitusCtlStep {
    ValidateInput,
    RenderDiff,
    RunPreflight,
    Execute,
    WriteAuditRecord,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CitusCtlError {
    CorruptState(String),
    InvalidFeatureId,
    InvalidPlanId,
    InvalidManifest(String),
    InvalidTimestamp,
    MissingRequiredField(&'static str),
    KubernetesCommand(String),
    PlanIdMismatch,
    StateIo(String),
    TimeTravelPlanIdMismatch,
    UnknownCommand(String),
    UnknownIntent(String),
    UnknownValue { field: &'static str, value: String },
    UnsafeStateDir(String),
}

impl fmt::Display for CitusCtlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CorruptState(detail) => {
                write!(formatter, "corrupt dev lifecycle state: {detail}")
            }
            Self::InvalidFeatureId => write!(formatter, "feature_id must be a stable feature id"),
            Self::InvalidPlanId => write!(formatter, "plan_id must be stable ascii and non-empty"),
            Self::InvalidManifest(detail) => {
                write!(formatter, "invalid Kubernetes manifest: {detail}")
            }
            Self::InvalidTimestamp => {
                write!(formatter, "target_time must be an RFC3339 UTC timestamp")
            }
            Self::KubernetesCommand(detail) => {
                write!(formatter, "Kubernetes command failed: {detail}")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::PlanIdMismatch => {
                write!(
                    formatter,
                    "plan_id does not match current Kubernetes manifest plan"
                )
            }
            Self::StateIo(error) => write!(formatter, "dev lifecycle state io failed: {error}"),
            Self::TimeTravelPlanIdMismatch => {
                write!(
                    formatter,
                    "plan_id does not match current time-travel intent plan"
                )
            }
            Self::UnknownCommand(command) => write!(formatter, "unknown command: {command}"),
            Self::UnknownIntent(intent) => write!(formatter, "unknown intent: {intent}"),
            Self::UnknownValue { field, value } => write!(formatter, "unknown {field}: {value}"),
            Self::UnsafeStateDir(path) => {
                write!(formatter, "unsafe dev lifecycle state dir: {path}")
            }
        }
    }
}

impl Error for CitusCtlError {}

impl From<std::io::Error> for CitusCtlError {
    fn from(error: std::io::Error) -> Self {
        Self::StateIo(error.to_string())
    }
}

pub fn parse_request(args: &[String]) -> Result<CitusCtlRequest, CitusCtlError> {
    let (intent, rest) = parse_intent(args)?;
    let command = parse_command(rest)?;
    Ok(CitusCtlRequest { intent, command })
}

pub fn v2_command_catalog() -> &'static [&'static str] {
    &[
        "init",
        "apply",
        "upgrade",
        "dev up",
        "dev down",
        "bench",
        "inspect",
        "vectorizer",
        "branch",
        "migrate",
        "dump",
        "restore",
        "restore-pitr",
        "tenant",
        "webhook",
        "function",
        "search",
        "backup",
        "time-travel",
        "wal-replay",
        "new-feature",
    ]
}

fn parse_intent(args: &[String]) -> Result<(ExecutionIntent, &[String]), CitusCtlError> {
    let Some(first) = args.first() else {
        return Err(CitusCtlError::MissingRequiredField("intent"));
    };

    match first.as_str() {
        "plan" => Ok((ExecutionIntent::Plan, &args[1..])),
        "apply" => {
            let Some(plan_id) = args.get(1) else {
                return Err(CitusCtlError::MissingRequiredField("plan_id"));
            };
            Ok((
                ExecutionIntent::Apply {
                    plan_id: plan_id.clone(),
                },
                &args[2..],
            ))
        }
        other => Err(CitusCtlError::UnknownIntent(other.to_string())),
    }
}

fn parse_command(args: &[String]) -> Result<CitusCtlCommand, CitusCtlError> {
    let Some(command) = args.first() else {
        return Err(CitusCtlError::MissingRequiredField("command"));
    };

    let value = |index: usize, field: &'static str| {
        args.get(index)
            .cloned()
            .ok_or(CitusCtlError::MissingRequiredField(field))
    };

    match command.as_str() {
        "init" => {
            expect_arg_count(args, 2, "init")?;
            Ok(CitusCtlCommand::Init {
                cluster: value(1, "cluster")?,
            })
        }
        "apply" => {
            expect_arg_count(args, 2, "apply")?;
            Ok(CitusCtlCommand::Apply {
                manifest_path: value(1, "manifest_path")?,
            })
        }
        "upgrade" => {
            expect_arg_count(args, 2, "upgrade")?;
            Ok(CitusCtlCommand::Upgrade {
                target_version: value(1, "target_version")?,
            })
        }
        "dev" => {
            expect_arg_count(args, 2, "dev")?;
            Ok(CitusCtlCommand::Dev {
                action: parse_dev_action(&value(1, "dev_action")?)?,
            })
        }
        "bench" => {
            expect_arg_count(args, 2, "bench")?;
            Ok(CitusCtlCommand::Bench {
                profile: value(1, "profile")?,
            })
        }
        "inspect" => {
            expect_arg_count(args, 2, "inspect")?;
            Ok(CitusCtlCommand::Inspect {
                target: parse_inspect_target(&value(1, "inspect_target")?)?,
            })
        }
        "vectorizer" => Ok(CitusCtlCommand::Vectorizer {
            action: parse_named_action(args, "vectorizer_name")?,
        }),
        "branch" => Ok(CitusCtlCommand::Branch {
            action: parse_named_action(args, "branch_name")?,
        }),
        "migrate" => {
            expect_arg_count(args, 2, "migrate")?;
            Ok(CitusCtlCommand::Migrate {
                manifest_path: value(1, "manifest_path")?,
            })
        }
        "dump" => {
            expect_arg_count(args, 2, "dump")?;
            Ok(CitusCtlCommand::Dump {
                target: value(1, "target")?,
            })
        }
        "restore" => {
            expect_arg_count(args, 2, "restore")?;
            Ok(CitusCtlCommand::Restore {
                source_uri: value(1, "source_uri")?,
            })
        }
        "restore-pitr" => {
            expect_arg_count(args, 3, "restore-pitr")?;
            Ok(CitusCtlCommand::RestorePitr {
                cluster: value(1, "cluster")?,
                target_time: value(2, "target_time")?,
            })
        }
        "tenant" => Ok(CitusCtlCommand::Tenant {
            action: parse_named_action(args, "tenant_name")?,
        }),
        "webhook" => Ok(CitusCtlCommand::Webhook {
            action: parse_named_action(args, "webhook_name")?,
        }),
        "function" => Ok(CitusCtlCommand::Function {
            action: parse_named_action(args, "function_name")?,
        }),
        "search" => Ok(CitusCtlCommand::Search {
            action: parse_named_action(args, "search_name")?,
        }),
        "backup" => Ok(CitusCtlCommand::Backup {
            action: parse_named_action(args, "backup_name")?,
        }),
        "time-travel" => {
            expect_arg_count(args, 2, "time-travel")?;
            Ok(CitusCtlCommand::TimeTravel {
                target_time: value(1, "target_time")?,
            })
        }
        "wal-replay" => {
            expect_arg_count(args, 3, "wal-replay")?;
            Ok(CitusCtlCommand::WalReplay {
                source_uri: value(1, "source_uri")?,
                target_time: value(2, "target_time")?,
            })
        }
        "new-feature" => {
            expect_arg_count(args, 2, "new-feature")?;
            Ok(CitusCtlCommand::NewFeature {
                feature_id: value(1, "feature_id")?,
            })
        }
        other => Err(CitusCtlError::UnknownCommand(other.to_string())),
    }
}

fn parse_named_action(
    args: &[String],
    name_field: &'static str,
) -> Result<NamedAction, CitusCtlError> {
    let command_name = args.first().map(String::as_str).unwrap_or("action");
    expect_arg_count(args, 3, command_name)?;
    let verb = args
        .get(1)
        .ok_or(CitusCtlError::MissingRequiredField("verb"))
        .and_then(|verb| parse_action_verb(verb))?;
    let name = args
        .get(2)
        .cloned()
        .ok_or(CitusCtlError::MissingRequiredField(name_field))?;
    Ok(NamedAction { verb, name })
}

fn expect_arg_count(
    args: &[String],
    expected: usize,
    command_name: &str,
) -> Result<(), CitusCtlError> {
    if args.len() == expected {
        return Ok(());
    }
    Err(CitusCtlError::UnknownValue {
        field: "argument",
        value: format!(
            "{command_name} expected {} argument(s), got {}",
            expected.saturating_sub(1),
            args.len().saturating_sub(1)
        ),
    })
}

fn parse_dev_action(value: &str) -> Result<DevAction, CitusCtlError> {
    match value {
        "up" => Ok(DevAction::Up),
        "down" => Ok(DevAction::Down),
        other => Err(CitusCtlError::UnknownValue {
            field: "dev_action",
            value: other.to_string(),
        }),
    }
}

fn parse_inspect_target(value: &str) -> Result<InspectTarget, CitusCtlError> {
    match value {
        "cluster" => Ok(InspectTarget::Cluster),
        "shards" => Ok(InspectTarget::Shards),
        "hypertables" => Ok(InspectTarget::Hypertables),
        "tenants" => Ok(InspectTarget::Tenants),
        "branches" => Ok(InspectTarget::Branches),
        "backups" => Ok(InspectTarget::Backups),
        other => Err(CitusCtlError::UnknownValue {
            field: "inspect_target",
            value: other.to_string(),
        }),
    }
}

fn parse_action_verb(value: &str) -> Result<ActionVerb, CitusCtlError> {
    match value {
        "create" => Ok(ActionVerb::Create),
        "apply" => Ok(ActionVerb::Apply),
        "delete" => Ok(ActionVerb::Delete),
        "archive" => Ok(ActionVerb::Archive),
        "move" => Ok(ActionVerb::Move),
        "promote" => Ok(ActionVerb::Promote),
        "suspend" => Ok(ActionVerb::Suspend),
        "resume" => Ok(ActionVerb::Resume),
        "inspect" => Ok(ActionVerb::Inspect),
        other => Err(CitusCtlError::UnknownValue {
            field: "verb",
            value: other.to_string(),
        }),
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), CitusCtlError> {
    if value.trim().is_empty() {
        return Err(CitusCtlError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_state_dir(path: &Path) -> Result<(), CitusCtlError> {
    let rendered = path.to_string_lossy();
    if rendered.trim().is_empty()
        || rendered.contains('\0')
        || path == Path::new("/")
        || path.components().count() <= 1
    {
        return Err(CitusCtlError::UnsafeStateDir(rendered.to_string()));
    }
    Ok(())
}

fn validate_k8s_namespace(value: &str) -> Result<(), CitusCtlError> {
    validate_required("namespace", value)?;
    let bytes = value.as_bytes();
    let valid = bytes.len() <= 63
        && bytes
            .first()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes
            .last()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(CitusCtlError::UnknownValue {
            field: "namespace",
            value: value.to_string(),
        })
    }
}

fn validate_plan_id(value: &str) -> Result<(), CitusCtlError> {
    validate_required("plan_id", value)?;
    if value.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || character == '-'
            || character == '_'
            || character == '.'
    }) {
        Ok(())
    } else {
        Err(CitusCtlError::InvalidPlanId)
    }
}

fn dev_lifecycle_steps(action: DevAction, dry_run: bool) -> Vec<DevLifecycleStep> {
    let mut steps = vec![
        DevLifecycleStep::ValidateStateDir,
        DevLifecycleStep::ReadState,
        DevLifecycleStep::RenderPlan,
    ];
    if !dry_run {
        steps.push(DevLifecycleStep::VerifyPlanId);
        steps.push(DevLifecycleStep::CreateStateDir);
        match action {
            DevAction::Up => steps.push(DevLifecycleStep::WriteState),
            DevAction::Down => steps.push(DevLifecycleStep::RemoveStateFile),
        }
        steps.push(DevLifecycleStep::WriteAuditRecord);
    }
    steps
}

fn append_dev_audit_record(
    path: &Path,
    plan: &DevLifecyclePlan,
    changed: bool,
    state_written: bool,
    state_removed: bool,
) -> Result<(), CitusCtlError> {
    let write_header = !path.exists();
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    if write_header {
        writeln!(
            file,
            "plan_id\tcluster\taction\tbefore_status\tafter_status\tchanged\tstate_written\tstate_removed\tevidence_boundary"
        )?;
    }
    writeln!(
        file,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\tlocal-state-file-only",
        plan.plan_id.as_deref().unwrap_or(""),
        plan.after.cluster_name,
        plan.action.as_str(),
        plan.before.status.as_str(),
        plan.after.status.as_str(),
        changed,
        state_written,
        state_removed
    )?;
    Ok(())
}

fn read_dev_state(path: &Path) -> Result<Option<DevLifecycleState>, CitusCtlError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut cluster_name = None;
    let mut status = None;
    let mut generation = None;
    let mut last_plan_id = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            return Err(CitusCtlError::CorruptState(line.to_string()));
        };
        match key {
            "cluster" => cluster_name = Some(value.to_string()),
            "status" => {
                status = Some(match value {
                    "running" => DevClusterStatus::Running,
                    "absent" => DevClusterStatus::Absent,
                    other => return Err(CitusCtlError::CorruptState(other.to_string())),
                });
            }
            "generation" => {
                generation = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| CitusCtlError::CorruptState(value.to_string()))?,
                );
            }
            "last_plan_id" if !value.is_empty() => last_plan_id = Some(value.to_string()),
            "last_plan_id" => {}
            other => return Err(CitusCtlError::CorruptState(other.to_string())),
        }
    }
    Ok(Some(DevLifecycleState {
        cluster_name: cluster_name
            .ok_or(CitusCtlError::CorruptState("missing cluster".to_string()))?,
        status: status.ok_or(CitusCtlError::CorruptState("missing status".to_string()))?,
        generation: generation.ok_or(CitusCtlError::CorruptState(
            "missing generation".to_string(),
        ))?,
        last_plan_id,
    }))
}

fn write_dev_state(path: &Path, state: &DevLifecycleState) -> Result<(), CitusCtlError> {
    let last_plan_id = state.last_plan_id.as_deref().unwrap_or("");
    fs::write(
        path,
        format!(
            "cluster={}
status={}
generation={}
last_plan_id={}
",
            state.cluster_name,
            state.status.as_str(),
            state.generation,
            last_plan_id
        ),
    )?;
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct K8sManifest {
    content_hash: String,
}

impl K8sManifest {
    fn load(path: &Path) -> Result<Self, CitusCtlError> {
        validate_required("manifest_path", &path.to_string_lossy())?;
        let content = fs::read_to_string(path).map_err(|error| {
            CitusCtlError::InvalidManifest(format!("{}: {}", path.to_string_lossy(), error))
        })?;
        validate_k8s_manifest_text(&content)?;
        Ok(Self {
            content_hash: format!("fnv1a64:{:016x}", fnv1a64(content.as_bytes())),
        })
    }
}

fn validate_k8s_manifest_text(content: &str) -> Result<(), CitusCtlError> {
    if content.trim().is_empty() {
        return Err(CitusCtlError::InvalidManifest(
            "manifest is empty".to_string(),
        ));
    }
    for required in ["apiVersion:", "kind:", "metadata:", "name:"] {
        if !content
            .lines()
            .any(|line| line.trim_start().starts_with(required))
        {
            return Err(CitusCtlError::InvalidManifest(format!(
                "missing required YAML field {required}"
            )));
        }
    }
    Ok(())
}

fn k8s_manifest_plan_id(manifest: &K8sManifest, namespace: &str) -> String {
    let mut material = Vec::new();
    material.extend_from_slice(manifest.content_hash.as_bytes());
    material.push(0);
    material.extend_from_slice(namespace.as_bytes());
    format!("k8s-apply-{:016x}", fnv1a64(&material))
}

fn parse_kubectl_resources(output: &str) -> Result<Vec<String>, CitusCtlError> {
    let resources: Vec<String> = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect();
    if resources.is_empty() {
        return Err(CitusCtlError::KubernetesCommand(
            "kubectl returned no resource names".to_string(),
        ));
    }
    Ok(resources)
}

fn append_k8s_manifest_audit_record(
    path: &Path,
    plan: &K8sManifestPlan,
    changed: bool,
    resources: &[String],
) -> Result<(), CitusCtlError> {
    let write_header = !path.exists();
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    if write_header {
        writeln!(
            file,
            "plan_id\tnamespace\tmanifest_path\tmanifest_hash\tchanged\tresources\tevidence_boundary"
        )?;
    }
    writeln!(
        file,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}",
        plan.plan_id,
        plan.namespace,
        plan.manifest_path,
        plan.manifest_hash,
        changed,
        resources.join(","),
        K8S_MANIFEST_EVIDENCE_BOUNDARY
    )?;
    Ok(())
}

fn append_time_travel_intent_audit_record(
    path: &Path,
    plan: &TimeTravelIntentPlan,
) -> Result<(), CitusCtlError> {
    let write_header = !path.exists();
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    if write_header {
        writeln!(
            file,
            "plan_id\ttarget_time\tnow\tage_seconds\tmax_staleness_seconds\tevidence_boundary"
        )?;
    }
    writeln!(
        file,
        "{}\t{}\t{}\t{}\t{}\t{}",
        plan.plan_id,
        plan.target_time,
        plan.now,
        plan.age_seconds,
        plan.max_staleness_seconds,
        TIME_TRAVEL_EVIDENCE_BOUNDARY
    )?;
    Ok(())
}

fn validate_timestamp(value: &str) -> Result<(), CitusCtlError> {
    validate_required("target_time", value)?;
    utc_timestamp_epoch_seconds(value).map(|_| ())
}

fn utc_timestamp_epoch_seconds(value: &str) -> Result<i64, CitusCtlError> {
    if value.len() != 20 {
        return Err(CitusCtlError::InvalidTimestamp);
    }
    let bytes = value.as_bytes();
    if bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
        || bytes.get(19) != Some(&b'Z')
    {
        return Err(CitusCtlError::InvalidTimestamp);
    }
    let year = parse_fixed_digits(&value[0..4])? as i32;
    let month = parse_fixed_digits(&value[5..7])? as u32;
    let day = parse_fixed_digits(&value[8..10])? as u32;
    let hour = parse_fixed_digits(&value[11..13])? as u32;
    let minute = parse_fixed_digits(&value[14..16])? as u32;
    let second = parse_fixed_digits(&value[17..19])? as u32;

    if !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return Err(CitusCtlError::InvalidTimestamp);
    }
    let max_day = days_in_month(year, month);
    if day == 0 || day > max_day {
        return Err(CitusCtlError::InvalidTimestamp);
    }

    let days = days_from_civil(year, month, day);
    Ok(days * 86_400 + i64::from(hour * 3_600 + minute * 60 + second))
}

fn parse_fixed_digits(value: &str) -> Result<u32, CitusCtlError> {
    if !value.chars().all(|character| character.is_ascii_digit()) {
        return Err(CitusCtlError::InvalidTimestamp);
    }
    value
        .parse::<u32>()
        .map_err(|_| CitusCtlError::InvalidTimestamp)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let adjusted_year = year - i32::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let month_for_formula = month as i32 + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_for_formula + 2) / 5 + day as i32 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    i64::from(era * 146_097 + day_of_era - 719_468)
}

fn validate_feature_id(value: &str) -> Result<(), CitusCtlError> {
    validate_required("feature_id", value)?;
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '.')
    {
        Ok(())
    } else {
        Err(CitusCtlError::InvalidFeatureId)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CitusCtlCanonicalReport {
    pub plans: Vec<CitusCtlPlan>,
    pub catalog_count: usize,
}

impl CitusCtlCanonicalReport {
    pub fn destructive_count(&self) -> usize {
        self.plans.iter().filter(|plan| plan.destructive).count()
    }

    pub fn preflight_count(&self) -> usize {
        self.plans
            .iter()
            .filter(|plan| plan.steps.contains(&CitusCtlStep::RunPreflight))
            .count()
    }

    pub fn execute_count(&self) -> usize {
        self.plans
            .iter()
            .filter(|plan| plan.steps.contains(&CitusCtlStep::Execute))
            .count()
    }

    pub fn total_steps(&self) -> usize {
        self.plans.iter().map(|plan| plan.steps.len()).sum()
    }
}

pub fn canonical_citusctl_report() -> Result<CitusCtlCanonicalReport, CitusCtlError> {
    let examples = [
        &["plan", "dev", "up"][..],
        &["apply", "plan-123", "apply", "deploy/k8s/values-prod.yaml"][..],
        &["plan", "inspect", "cluster"][..],
        &["plan", "time-travel", "2026-05-21T10:00:00Z"][..],
        &[
            "plan",
            "wal-replay",
            "s3://citus-wal/prod",
            "2026-05-21T10:00:00Z",
        ][..],
    ];
    let plans = examples
        .iter()
        .map(|example| parse_request(&args(example)).and_then(|request| request.plan()))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CitusCtlCanonicalReport {
        plans,
        catalog_count: v2_command_catalog().len(),
    })
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WalReplayDebugPlan {
    pub source_uri: String,
    pub target_time: String,
    pub timeline: String,
    pub start_time: String,
    pub end_time: String,
    pub segments: u32,
}

impl WalReplayDebugPlan {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"actions\":[\"validate_source\",\"inspect_fixture\",\"bound_target_time\",\"render_replay_plan\"],\"end_time\":\"{}\",\"segments\":{},\"source_uri\":\"{}\",\"start_time\":\"{}\",\"target_time\":\"{}\",\"timeline\":\"{}\"}}",
            json_escape(&self.end_time),
            self.segments,
            json_escape(&self.source_uri),
            json_escape(&self.start_time),
            json_escape(&self.target_time),
            json_escape(&self.timeline)
        )
    }
}

pub fn wal_replay_debug_plan_from_args(
    args: &[String],
) -> Result<WalReplayDebugPlan, CitusCtlError> {
    let options = WalReplayDebugOptions::parse(args)?;
    if !options.json {
        return Err(CitusCtlError::MissingRequiredField("json"));
    }
    let fixture_path = options
        .fixture_path
        .as_deref()
        .ok_or(CitusCtlError::MissingRequiredField("fixture_path"))?;
    let request = parse_request(&options.command_args)?;
    if !matches!(request.intent, ExecutionIntent::Plan) {
        return Err(CitusCtlError::UnknownIntent(
            "wal-replay-debug-apply".to_string(),
        ));
    }

    let CitusCtlCommand::WalReplay {
        source_uri,
        target_time,
    } = request.command
    else {
        return Err(CitusCtlError::UnknownCommand(
            "wal-replay-debug-json".to_string(),
        ));
    };

    validate_wal_replay_source(&source_uri)?;
    validate_timestamp(&target_time)?;

    let fixture_text =
        std::fs::read_to_string(fixture_path).map_err(|_| CitusCtlError::UnknownValue {
            field: "fixture_path",
            value: fixture_path.to_string(),
        })?;
    let fixture = WalReplayFixture::parse(&fixture_text)?;
    fixture.validate_for(&source_uri, &target_time)?;

    Ok(WalReplayDebugPlan {
        source_uri,
        target_time,
        timeline: fixture.timeline,
        start_time: fixture.start_time,
        end_time: fixture.end_time,
        segments: fixture.segments,
    })
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct WalReplayDebugOptions {
    command_args: Vec<String>,
    fixture_path: Option<String>,
    json: bool,
}

impl WalReplayDebugOptions {
    fn parse(args: &[String]) -> Result<Self, CitusCtlError> {
        let mut command_args = Vec::new();
        let mut fixture_path = None;
        let mut json = false;
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--json" => {
                    json = true;
                    index += 1;
                }
                "--fixture" => {
                    let Some(path) = args.get(index + 1) else {
                        return Err(CitusCtlError::MissingRequiredField("fixture_path"));
                    };
                    fixture_path = Some(path.clone());
                    index += 2;
                }
                value if value.starts_with("--") => {
                    return Err(CitusCtlError::UnknownValue {
                        field: "wal_replay_option",
                        value: value.to_string(),
                    });
                }
                value => {
                    command_args.push(value.to_string());
                    index += 1;
                }
            }
        }

        Ok(Self {
            command_args,
            fixture_path,
            json,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct WalReplayFixture {
    source_uri: String,
    timeline: String,
    start_time: String,
    end_time: String,
    segments: u32,
}

impl WalReplayFixture {
    fn parse(text: &str) -> Result<Self, CitusCtlError> {
        let mut source_uri = None;
        let mut timeline = None;
        let mut start_time = None;
        let mut end_time = None;
        let mut segments = None;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(CitusCtlError::UnknownValue {
                    field: "wal_replay_fixture",
                    value: line.to_string(),
                });
            };
            let value = value.trim().to_string();
            match key.trim() {
                "source_uri" if source_uri.is_none() => source_uri = Some(value),
                "timeline" if timeline.is_none() => timeline = Some(value),
                "start_time" if start_time.is_none() => start_time = Some(value),
                "end_time" if end_time.is_none() => end_time = Some(value),
                "segments" if segments.is_none() => {
                    segments = Some(value.parse::<u32>().ok().filter(|count| *count > 0).ok_or(
                        CitusCtlError::UnknownValue {
                            field: "segments",
                            value,
                        },
                    )?)
                }
                _ => {
                    return Err(CitusCtlError::UnknownValue {
                        field: "wal_replay_fixture",
                        value: key.trim().to_string(),
                    })
                }
            }
        }

        let fixture = Self {
            source_uri: source_uri.ok_or(CitusCtlError::MissingRequiredField("source_uri"))?,
            timeline: timeline.ok_or(CitusCtlError::MissingRequiredField("timeline"))?,
            start_time: start_time.ok_or(CitusCtlError::MissingRequiredField("start_time"))?,
            end_time: end_time.ok_or(CitusCtlError::MissingRequiredField("end_time"))?,
            segments: segments.ok_or(CitusCtlError::MissingRequiredField("segments"))?,
        };
        validate_wal_replay_source(&fixture.source_uri)?;
        validate_required("timeline", &fixture.timeline)?;
        validate_timestamp(&fixture.start_time)?;
        validate_timestamp(&fixture.end_time)?;
        if fixture.start_time > fixture.end_time {
            return Err(CitusCtlError::UnknownValue {
                field: "wal_replay_fixture",
                value: "start_time after end_time".to_string(),
            });
        }
        Ok(fixture)
    }

    fn validate_for(&self, source_uri: &str, target_time: &str) -> Result<(), CitusCtlError> {
        if self.source_uri != source_uri {
            return Err(CitusCtlError::UnknownValue {
                field: "source_uri",
                value: "must match fixture source_uri".to_string(),
            });
        }
        if target_time < self.start_time.as_str() || target_time > self.end_time.as_str() {
            return Err(CitusCtlError::UnknownValue {
                field: "target_time",
                value: "outside fixture range".to_string(),
            });
        }
        Ok(())
    }
}

fn validate_wal_replay_source(value: &str) -> Result<(), CitusCtlError> {
    validate_required("source_uri", value)?;
    if value.starts_with("s3://") || value.starts_with("gs://") || value.starts_with("file://") {
        Ok(())
    } else {
        Err(CitusCtlError::UnknownValue {
            field: "source_uri",
            value: value.to_string(),
        })
    }
}

fn json_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            _ => vec![character],
        })
        .collect()
}

fn json_string_array(values: &[String]) -> String {
    let mut rendered = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            rendered.push(',');
        }
        rendered.push('"');
        rendered.push_str(&json_escape(value));
        rendered.push('"');
    }
    rendered.push(']');
    rendered
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_up_plan_is_non_destructive_and_preflighted() {
        let request = parse_request(&args(&["plan", "dev", "up"])).expect("parse dev up");
        let plan = request.plan().expect("dev up plan");

        assert_eq!(plan.command_name, "dev");
        assert!(!plan.destructive);
        assert_eq!(
            plan.steps,
            vec![
                CitusCtlStep::ValidateInput,
                CitusCtlStep::RenderDiff,
                CitusCtlStep::RunPreflight
            ]
        );
    }

    #[test]
    fn dev_lifecycle_dry_run_does_not_write_state() {
        let dir = temp_state_dir("dry-run");
        let _ = fs::remove_dir_all(&dir);
        let runtime = DevLifecycleRuntime::new("dev-citus", &dir).expect("runtime");
        let plan = runtime.plan(DevAction::Up).expect("plan");

        assert!(plan.dry_run);
        assert_eq!(plan.before.status, DevClusterStatus::Absent);
        assert_eq!(plan.after.status, DevClusterStatus::Running);
        assert!(!runtime.state_path().exists());
    }

    #[test]
    fn dev_lifecycle_apply_is_idempotent_and_down_removes_state_file_only() {
        let dir = temp_state_dir("apply");
        let _ = fs::remove_dir_all(&dir);
        let runtime = DevLifecycleRuntime::new("dev-citus", &dir).expect("runtime");

        let first_up = runtime.apply(DevAction::Up, "plan-up-1").expect("up");
        assert!(first_up.changed);
        assert!(first_up.state_written);
        assert!(runtime.state_path().exists());
        assert!(runtime.audit_path().exists());

        let second_up = runtime
            .apply(DevAction::Up, "plan-up-2")
            .expect("up idempotent");
        assert!(!second_up.changed);
        assert!(!second_up.state_written);

        let first_down = runtime.apply(DevAction::Down, "plan-down-1").expect("down");
        assert!(first_down.changed);
        assert!(first_down.state_removed);
        assert!(first_down.audit_record_written);
        assert!(!runtime.state_path().exists());
        assert!(runtime.audit_path().exists());
        assert_eq!(
            first_down.plan.cleanup_guard,
            "state-file-only-no-recursive-delete"
        );

        let second_down = runtime
            .apply(DevAction::Down, "plan-down-2")
            .expect("down idempotent");
        assert!(!second_down.changed);
        assert!(!second_down.state_removed);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dev_lifecycle_rejects_unsafe_state_dir() {
        assert_eq!(
            DevLifecycleRuntime::new("dev-citus", PathBuf::from("/")),
            Err(CitusCtlError::UnsafeStateDir("/".to_string()))
        );
    }

    #[test]
    fn dev_lifecycle_apply_rejects_unstable_plan_id() {
        let dir = temp_state_dir("bad-plan");
        let runtime = DevLifecycleRuntime::new("dev-citus", &dir).expect("runtime");

        assert_eq!(
            runtime.apply(DevAction::Up, "not ok"),
            Err(CitusCtlError::InvalidPlanId)
        );
    }

    #[test]
    fn apply_intent_requires_plan_id() {
        let error = parse_request(&args(&["apply"])).expect_err("missing plan id");
        assert_eq!(error, CitusCtlError::MissingRequiredField("plan_id"));
    }

    #[test]
    fn apply_intent_rejects_unstable_plan_id() {
        let request = parse_request(&args(&["apply", "not ok", "inspect", "cluster"]))
            .expect("parse invalid plan id request");
        assert_eq!(request.plan(), Err(CitusCtlError::InvalidPlanId));
    }

    #[test]
    fn parser_rejects_trailing_arguments() {
        let error = parse_request(&args(&["plan", "inspect", "cluster", "ignored"]))
            .expect_err("trailing argument rejected");
        assert_eq!(
            error,
            CitusCtlError::UnknownValue {
                field: "argument",
                value: "inspect expected 1 argument(s), got 2".to_string(),
            }
        );
    }

    #[test]
    fn destructive_apply_renders_execute_steps() {
        let request = parse_request(&args(&[
            "apply", "plan-123", "tenant", "archive", "tenant-a",
        ]))
        .expect("parse tenant archive");
        let plan = request.plan().expect("tenant archive plan");

        assert!(plan.destructive);
        assert!(plan.requires_plan_id);
        assert!(plan.steps.contains(&CitusCtlStep::Execute));
        assert!(plan.steps.contains(&CitusCtlStep::WriteAuditRecord));
    }

    #[test]
    fn restore_pitr_rejects_non_utc_timestamp() {
        let request = parse_request(&args(&[
            "plan",
            "restore-pitr",
            "prod",
            "2026-05-19 12:00:00",
        ]))
        .expect("parse restore-pitr");

        assert_eq!(request.plan(), Err(CitusCtlError::InvalidTimestamp));
    }

    #[test]
    fn catalog_covers_v2_commands() {
        let catalog = v2_command_catalog();

        for command in [
            "init",
            "apply",
            "upgrade",
            "dev up",
            "dev down",
            "bench",
            "inspect",
            "vectorizer",
            "branch",
            "migrate",
            "dump",
            "restore",
            "restore-pitr",
            "tenant",
            "webhook",
            "function",
            "search",
            "backup",
            "time-travel",
            "wal-replay",
            "new-feature",
        ] {
            assert!(catalog.contains(&command), "missing {command}");
        }
    }

    #[test]
    fn wal_replay_requires_utc_target_time() {
        let request = parse_request(&args(&[
            "plan",
            "wal-replay",
            "s3://citus-wal/prod",
            "2026-05-19 12:00:00",
        ]))
        .expect("parse wal replay");

        assert_eq!(request.plan(), Err(CitusCtlError::InvalidTimestamp));
    }

    #[test]
    fn wal_replay_debug_plan_reads_fixture_and_emits_json() {
        let fixture_path = std::env::temp_dir().join(format!(
            "ai-blaise-citusctl-wal-fixture-{}.txt",
            std::process::id()
        ));
        std::fs::write(
            &fixture_path,
            "source_uri=s3://citus-wal/prod\ntimeline=0000000100000000000000A1\nstart_time=2026-05-21T09:00:00Z\nend_time=2026-05-21T11:00:00Z\nsegments=3\n",
        )
        .expect("write fixture");

        let plan = wal_replay_debug_plan_from_args(&args(&[
            "plan",
            "wal-replay",
            "s3://citus-wal/prod",
            "2026-05-21T10:00:00Z",
            "--fixture",
            fixture_path.to_str().expect("fixture path"),
            "--json",
        ]))
        .expect("debug plan");
        std::fs::remove_file(&fixture_path).expect("remove fixture");

        assert_eq!(plan.segments, 3);
        assert_eq!(
            plan.to_json(),
            "{\"actions\":[\"validate_source\",\"inspect_fixture\",\"bound_target_time\",\"render_replay_plan\"],\"end_time\":\"2026-05-21T11:00:00Z\",\"segments\":3,\"source_uri\":\"s3://citus-wal/prod\",\"start_time\":\"2026-05-21T09:00:00Z\",\"target_time\":\"2026-05-21T10:00:00Z\",\"timeline\":\"0000000100000000000000A1\"}"
        );
    }

    #[test]
    fn wal_replay_debug_plan_fails_closed_outside_fixture_range() {
        let fixture = WalReplayFixture::parse(
            "source_uri=file:///tmp/wal\ntimeline=0000000100000000000000A1\nstart_time=2026-05-21T09:00:00Z\nend_time=2026-05-21T11:00:00Z\nsegments=3\n",
        )
        .expect("fixture");

        assert_eq!(
            fixture.validate_for("file:///tmp/wal", "2026-05-21T12:00:00Z"),
            Err(CitusCtlError::UnknownValue {
                field: "target_time",
                value: "outside fixture range".to_string(),
            })
        );
    }

    #[test]
    fn dev_lifecycle_cli_reports_json_tsv_and_audit_append() {
        let dir = temp_state_dir("cli-report");
        let _ = fs::remove_dir_all(&dir);
        let state_dir = dir.to_str().expect("state dir");
        let state_path = dir.join("dev-lifecycle.state");
        let audit_path = dir.join("dev-lifecycle.audit.tsv");

        let plan_json = render_dev_lifecycle_cli_report_from_args(&args(&[
            "plan",
            "dev",
            "up",
            "--state-dir",
            state_dir,
            "--format",
            "json",
        ]))
        .expect("plan json")
        .expect("dev lifecycle output");
        assert!(plan_json.contains("\"mode\":\"plan\""));
        assert!(plan_json.contains("\"dry_run\":true"));
        assert!(plan_json.contains("\"plan_id\":null"));
        assert!(plan_json.contains(&format!(
            "\"state_path\":\"{}\"",
            json_escape(&state_path.to_string_lossy())
        )));
        assert!(!state_path.exists());
        assert!(!audit_path.exists());

        let apply_tsv = render_dev_lifecycle_cli_report_from_args(&args(&[
            "apply",
            "plan-dev-up-1",
            "dev",
            "up",
            "--state-dir",
            state_dir,
            "--format",
            "tsv",
        ]))
        .expect("apply tsv")
        .expect("dev lifecycle output");
        assert!(apply_tsv.starts_with(DevLifecycleCliReport::tsv_header()));
        assert!(apply_tsv.contains("apply\tdev-citus\tup\t"));
        assert!(apply_tsv.contains("\tplan-dev-up-1\tfalse\ttrue\ttrue\tfalse\ttrue\t"));
        assert!(state_path.exists());
        assert!(audit_path.exists());
        let audit = fs::read_to_string(&audit_path).expect("audit");
        assert_eq!(audit.lines().count(), 2);
        assert!(audit.contains("plan-dev-up-1\tdev-citus\tup\tabsent\trunning\ttrue\ttrue\tfalse"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn k8s_manifest_cli_reports_live_boundary_with_fake_kubectl() {
        let dir = temp_state_dir("k8s-apply");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("state dir");
        let manifest_path = dir.join("manifest.yaml");
        fs::write(
            &manifest_path,
            "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: ai-blaise-citusctl-live\ndata:\n  feature: M8\n",
        )
        .expect("manifest");
        let kubectl = fake_kubectl(&dir);
        let state_dir = dir.join("state");
        let manifest = manifest_path.to_str().expect("manifest path");
        let kubectl_path = kubectl.to_str().expect("kubectl path");
        let state = state_dir.to_str().expect("state dir");

        let plan_json = render_k8s_manifest_cli_report_from_args(&args(&[
            "plan",
            "apply",
            manifest,
            "--namespace",
            "ai-blaise-m8",
            "--state-dir",
            state,
            "--kubectl",
            kubectl_path,
            "--format",
            "json",
        ]))
        .expect("plan json")
        .expect("k8s plan output");
        assert!(plan_json.contains("\"mode\":\"plan\""));
        assert!(plan_json.contains("\"dry_run\":true"));
        assert!(plan_json.contains("\"audit_record_written\":false"));
        assert!(plan_json.contains("\"resources\":[\"configmap/ai-blaise-citusctl-live\"]"));
        assert!(plan_json.contains(K8S_MANIFEST_EVIDENCE_BOUNDARY));

        let runtime = K8sManifestRuntime::new(manifest, "ai-blaise-m8", state)
            .expect("runtime")
            .with_kubectl(kubectl_path)
            .expect("kubectl");
        let plan = runtime.plan().expect("computed plan");
        let apply_tsv = render_k8s_manifest_cli_report_from_args(&args(&[
            "apply",
            &plan.plan_id,
            "apply",
            manifest,
            "--namespace",
            "ai-blaise-m8",
            "--state-dir",
            state,
            "--kubectl",
            kubectl_path,
            "--format",
            "tsv",
        ]))
        .expect("apply tsv")
        .expect("k8s apply output");
        assert!(apply_tsv.starts_with(K8sManifestCliReport::tsv_header()));
        assert!(
            apply_tsv.contains("\tfalse\ttrue\ttrue\ttrue\tconfigmap/ai-blaise-citusctl-live\t6\t")
        );
        assert!(runtime.audit_path().exists());
        let audit = fs::read_to_string(runtime.audit_path()).expect("audit");
        assert_eq!(audit.lines().count(), 2);
        assert!(audit.contains(K8S_MANIFEST_EVIDENCE_BOUNDARY));
        let log = fs::read_to_string(dir.join("kubectl.log")).expect("kubectl log");
        assert!(log.contains("apply -f"));
        assert!(log.contains("--dry-run=server"));
        assert!(log.contains("get -f"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn k8s_manifest_apply_rejects_mismatched_plan_id() {
        let dir = temp_state_dir("k8s-bad-plan");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("state dir");
        let manifest_path = dir.join("manifest.yaml");
        fs::write(
            &manifest_path,
            "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: ai-blaise-citusctl-live\ndata:\n  feature: M8\n",
        )
        .expect("manifest");
        let kubectl = fake_kubectl(&dir);

        let error = render_k8s_manifest_cli_report_from_args(&args(&[
            "apply",
            "wrong-plan-id",
            "apply",
            manifest_path.to_str().expect("manifest path"),
            "--namespace",
            "ai-blaise-m8",
            "--state-dir",
            dir.join("state").to_str().expect("state dir"),
            "--kubectl",
            kubectl.to_str().expect("kubectl path"),
            "--format",
            "json",
        ]))
        .expect_err("mismatched plan id");
        assert_eq!(error, CitusCtlError::PlanIdMismatch);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn k8s_manifest_rejects_missing_metadata_name() {
        let dir = temp_state_dir("k8s-bad-manifest");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("state dir");
        let manifest_path = dir.join("manifest.yaml");
        fs::write(
            &manifest_path,
            "apiVersion: v1\nkind: ConfigMap\nmetadata:\n",
        )
        .expect("manifest");
        let runtime = K8sManifestRuntime::new(&manifest_path, "ai-blaise-m8", dir.join("state"))
            .expect("runtime")
            .with_kubectl(fake_kubectl(&dir).to_str().expect("kubectl path"))
            .expect("kubectl");

        assert_eq!(
            runtime.plan(),
            Err(CitusCtlError::InvalidManifest(
                "missing required YAML field name:".to_string()
            ))
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn k8s_manifest_rejects_invalid_namespace() {
        let error = K8sManifestRuntime::new("manifest.yaml", "Not_A_Namespace", "/tmp/m8-state")
            .expect_err("invalid namespace");

        assert_eq!(
            error,
            CitusCtlError::UnknownValue {
                field: "namespace",
                value: "Not_A_Namespace".to_string(),
            }
        );
    }

    #[test]
    fn time_travel_intent_reports_json_tsv_and_audit_append() {
        let dir = temp_state_dir("time-travel");
        let _ = fs::remove_dir_all(&dir);
        let state_dir = dir.to_str().expect("state dir");
        let plan_json = render_time_travel_intent_cli_report_from_args(&args(&[
            "plan",
            "time-travel",
            "2026-05-24T00:00:00Z",
            "--now",
            "2026-05-24T00:00:30Z",
            "--max-staleness-seconds",
            "60",
            "--state-dir",
            state_dir,
            "--format",
            "json",
        ]))
        .expect("plan json")
        .expect("time travel plan");
        assert!(plan_json.contains("\"mode\":\"plan\""));
        assert!(plan_json.contains("\"age_seconds\":30"));
        assert!(plan_json.contains("\"accepted\":true"));
        assert!(plan_json.contains("\"audit_record_written\":false"));
        assert!(plan_json.contains(TIME_TRAVEL_EVIDENCE_BOUNDARY));

        let runtime = TimeTravelIntentRuntime::new(
            "2026-05-24T00:00:00Z",
            "2026-05-24T00:00:30Z",
            60,
            state_dir,
        )
        .expect("runtime");
        let plan = runtime.plan().expect("plan");
        let apply_tsv = render_time_travel_intent_cli_report_from_args(&args(&[
            "apply",
            &plan.plan_id,
            "time-travel",
            "2026-05-24T00:00:00Z",
            "--now",
            "2026-05-24T00:00:30Z",
            "--max-staleness-seconds",
            "60",
            "--state-dir",
            state_dir,
            "--format",
            "tsv",
        ]))
        .expect("apply tsv")
        .expect("time travel apply");
        assert!(apply_tsv.starts_with(TimeTravelIntentCliReport::tsv_header()));
        assert!(apply_tsv.contains("\t30\t60\ttrue\tfalse\ttrue\t4\t"));
        let audit = fs::read_to_string(runtime.audit_path()).expect("audit");
        assert_eq!(audit.lines().count(), 2);
        assert!(audit.contains(TIME_TRAVEL_EVIDENCE_BOUNDARY));
        assert_eq!(
            runtime.apply("wrong-plan"),
            Err(CitusCtlError::TimeTravelPlanIdMismatch)
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn time_travel_intent_rejects_out_of_window_and_future_targets() {
        assert_eq!(
            TimeTravelIntentRuntime::new(
                "2026-05-23T23:59:00Z",
                "2026-05-24T00:00:30Z",
                60,
                "/tmp/time-travel-state",
            ),
            Err(CitusCtlError::UnknownValue {
                field: "target_time",
                value: "older than max_staleness_seconds 60".to_string(),
            })
        );
        assert_eq!(
            TimeTravelIntentRuntime::new(
                "2026-05-24T00:00:31Z",
                "2026-05-24T00:00:30Z",
                60,
                "/tmp/time-travel-state",
            ),
            Err(CitusCtlError::UnknownValue {
                field: "target_time",
                value: "must not be in the future".to_string(),
            })
        );
    }

    #[test]
    fn strict_utc_timestamp_rejects_invalid_calendar_dates() {
        for value in [
            "2026-02-29T00:00:00Z",
            "2026-05-24T24:00:00Z",
            "2026-05-24T00:00:60Z",
            "2026-05-24 00:00:00",
        ] {
            assert_eq!(
                validate_timestamp(value),
                Err(CitusCtlError::InvalidTimestamp)
            );
        }
        assert!(validate_timestamp("2024-02-29T00:00:00Z").is_ok());
    }

    #[test]
    fn canonical_report_covers_cli_contract_examples() {
        let report = canonical_citusctl_report().expect("canonical report");

        assert_eq!(report.catalog_count, 21);
        assert_eq!(report.plans.len(), 5);
        assert_eq!(report.destructive_count(), 2);
        assert_eq!(report.preflight_count(), 5);
        assert_eq!(report.execute_count(), 1);
        assert_eq!(report.total_steps(), 17);
    }

    #[test]
    fn canonical_dev_lifecycle_report_is_deterministic() {
        let report = canonical_dev_lifecycle_report().expect("dev lifecycle report");

        assert_eq!(report.plan_up_steps, 3);
        assert!(report.apply_up_changed);
        assert!(!report.idempotent_up_changed);
        assert!(report.apply_down_changed);
        assert!(!report.idempotent_down_changed);
        assert!(!report.final_state_present);
        assert_eq!(report.cleanup_guard, "state-file-only-no-recursive-delete");
        assert_eq!(report.evidence_boundary, "local-state-file-only");
    }

    fn temp_state_dir(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ai-blaise-citusctl-test-{suffix}-{}",
            std::process::id()
        ))
    }

    fn fake_kubectl(dir: &Path) -> PathBuf {
        let path = dir.join("kubectl");
        let log_path = dir.join("kubectl.log");
        let script = format!(
            "#!/usr/bin/env sh\n\
printf '%s\\n' \"$*\" >> '{}'\n\
case \"$1\" in\n\
  apply)\n\
    case \" $* \" in\n\
      *' --dry-run=server '*) printf '%s\\n' 'configmap/ai-blaise-citusctl-live' ;;\n\
      *) printf '%s\\n' 'configmap/ai-blaise-citusctl-live created' ;;\n\
    esac\n\
    ;;\n\
  get)\n\
    printf '%s\\n' 'configmap/ai-blaise-citusctl-live'\n\
    ;;\n\
  *)\n\
    printf '%s\\n' \"unexpected kubectl command: $*\" >&2\n\
    exit 1\n\
    ;;\n\
esac\n",
            log_path.to_string_lossy()
        );
        fs::write(&path, script).expect("fake kubectl");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&path).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).expect("permissions");
        }
        path
    }
}
