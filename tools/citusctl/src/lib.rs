//! Operator-facing command contracts for citusctl.

// FEATURE: D1
// FEATURE: D2
// FEATURE: M8
// FEATURE: B3
// FEATURE: B5
// FEATURE: WF2

use std::error::Error;
use std::fmt;
use std::fs;
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
            Self::Apply { plan_id } => validate_required("plan_id", plan_id),
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

        Ok(DevLifecycleReport {
            plan,
            changed,
            state_written,
            state_removed,
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
        "init" => Ok(CitusCtlCommand::Init {
            cluster: value(1, "cluster")?,
        }),
        "apply" => Ok(CitusCtlCommand::Apply {
            manifest_path: value(1, "manifest_path")?,
        }),
        "upgrade" => Ok(CitusCtlCommand::Upgrade {
            target_version: value(1, "target_version")?,
        }),
        "dev" => Ok(CitusCtlCommand::Dev {
            action: parse_dev_action(&value(1, "dev_action")?)?,
        }),
        "bench" => Ok(CitusCtlCommand::Bench {
            profile: value(1, "profile")?,
        }),
        "inspect" => Ok(CitusCtlCommand::Inspect {
            target: parse_inspect_target(&value(1, "inspect_target")?)?,
        }),
        "vectorizer" => Ok(CitusCtlCommand::Vectorizer {
            action: parse_named_action(args, "vectorizer_name")?,
        }),
        "branch" => Ok(CitusCtlCommand::Branch {
            action: parse_named_action(args, "branch_name")?,
        }),
        "migrate" => Ok(CitusCtlCommand::Migrate {
            manifest_path: value(1, "manifest_path")?,
        }),
        "dump" => Ok(CitusCtlCommand::Dump {
            target: value(1, "target")?,
        }),
        "restore" => Ok(CitusCtlCommand::Restore {
            source_uri: value(1, "source_uri")?,
        }),
        "restore-pitr" => Ok(CitusCtlCommand::RestorePitr {
            cluster: value(1, "cluster")?,
            target_time: value(2, "target_time")?,
        }),
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
        "time-travel" => Ok(CitusCtlCommand::TimeTravel {
            target_time: value(1, "target_time")?,
        }),
        "wal-replay" => Ok(CitusCtlCommand::WalReplay {
            source_uri: value(1, "source_uri")?,
            target_time: value(2, "target_time")?,
        }),
        "new-feature" => Ok(CitusCtlCommand::NewFeature {
            feature_id: value(1, "feature_id")?,
        }),
        other => Err(CitusCtlError::UnknownCommand(other.to_string())),
    }
}

fn parse_named_action(
    args: &[String],
    name_field: &'static str,
) -> Result<NamedAction, CitusCtlError> {
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

        let second_up = runtime
            .apply(DevAction::Up, "plan-up-2")
            .expect("up idempotent");
        assert!(!second_up.changed);
        assert!(!second_up.state_written);

        let first_down = runtime.apply(DevAction::Down, "plan-down-1").expect("down");
        assert!(first_down.changed);
        assert!(first_down.state_removed);
        assert!(!runtime.state_path().exists());
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
