//! Operator-facing command contracts for citusctl.

// FEATURE: D1
// FEATURE: D2
// FEATURE: M8
// FEATURE: B3
// FEATURE: B5
// FEATURE: WF2

use std::error::Error;
use std::fmt;

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
    InvalidFeatureId,
    InvalidTimestamp,
    MissingRequiredField(&'static str),
    UnknownCommand(String),
    UnknownIntent(String),
    UnknownValue { field: &'static str, value: String },
}

impl fmt::Display for CitusCtlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFeatureId => write!(formatter, "feature_id must be a stable feature id"),
            Self::InvalidTimestamp => {
                write!(formatter, "target_time must be an RFC3339 UTC timestamp")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::UnknownCommand(command) => write!(formatter, "unknown command: {command}"),
            Self::UnknownIntent(intent) => write!(formatter, "unknown intent: {intent}"),
            Self::UnknownValue { field, value } => write!(formatter, "unknown {field}: {value}"),
        }
    }
}

impl Error for CitusCtlError {}

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
}
