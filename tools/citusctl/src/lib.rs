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
    InvalidTimestamp,
    MissingRequiredField(&'static str),
    StateIo(String),
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
            Self::InvalidTimestamp => {
                write!(formatter, "target_time must be an RFC3339 UTC timestamp")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::StateIo(error) => write!(formatter, "dev lifecycle state io failed: {error}"),
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

fn validate_timestamp(value: &str) -> Result<(), CitusCtlError> {
    validate_required("target_time", value)?;
    if value.len() >= 20 && value.contains('T') && value.ends_with('Z') {
        Ok(())
    } else {
        Err(CitusCtlError::InvalidTimestamp)
    }
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
}
