// FEATURE: S2

//! Shared execution boundary contract for kube-rs controllers.
//!
//! The current operator controllers are allowed to validate specs and render
//! deterministic plans. Kubernetes apply, direct SQL execution, and `.status`
//! mutation remain explicit alpha operations until each path has a real runner,
//! RBAC, idempotency, and live evidence.

use kube::runtime::controller::Action;
use std::env;
use std::error::Error;
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use super::ControllerError;

pub const EXECUTION_MODE_ENV: &str = "AI_BLAISE_OPERATOR_EXECUTION_MODE";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ExecutionMode {
    DryRun,
    Apply,
}

impl ExecutionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::Apply => "apply",
        }
    }
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::DryRun
    }
}

impl FromStr for ExecutionMode {
    type Err = BoundaryError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw.trim() {
            "" | "dry-run" | "dry_run" | "dryrun" => Ok(Self::DryRun),
            "apply" => Ok(Self::Apply),
            other => Err(BoundaryError::InvalidExecutionMode(other.to_string())),
        }
    }
}

pub fn execution_mode_from_env() -> Result<ExecutionMode, BoundaryError> {
    match env::var(EXECUTION_MODE_ENV) {
        Ok(value) => ExecutionMode::from_str(&value),
        Err(env::VarError::NotPresent) => Ok(ExecutionMode::DryRun),
        Err(env::VarError::NotUnicode(value)) => Err(BoundaryError::InvalidExecutionMode(
            value.to_string_lossy().to_string(),
        )),
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BoundaryOperationKind {
    RenderPlan,
    KubernetesApply,
    DirectSql,
    StatusMutation,
}

impl BoundaryOperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RenderPlan => "render-plan",
            Self::KubernetesApply => "kubernetes-apply",
            Self::DirectSql => "direct-sql",
            Self::StatusMutation => "status-mutation",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BoundaryOperation {
    pub name: String,
    pub kind: BoundaryOperationKind,
    pub implemented: bool,
}

impl BoundaryOperation {
    pub fn render_plan(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: BoundaryOperationKind::RenderPlan,
            implemented: true,
        }
    }

    pub fn alpha(name: impl Into<String>, kind: BoundaryOperationKind) -> Self {
        Self {
            name: name.into(),
            kind,
            implemented: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConditionType {
    SpecAccepted,
    PlanRendered,
    DryRun,
    KubernetesApplyAlpha,
    DirectSqlAlpha,
    StatusMutationAlpha,
}

impl ConditionType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SpecAccepted => "SpecAccepted",
            Self::PlanRendered => "PlanRendered",
            Self::DryRun => "DryRun",
            Self::KubernetesApplyAlpha => "KubernetesApplyAlpha",
            Self::DirectSqlAlpha => "DirectSqlAlpha",
            Self::StatusMutationAlpha => "StatusMutationAlpha",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConditionStatus {
    True,
    False,
    Unknown,
}

impl ConditionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::True => "True",
            Self::False => "False",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ControllerCondition {
    pub condition_type: ConditionType,
    pub status: ConditionStatus,
    pub reason: String,
    pub message: String,
}

impl ControllerCondition {
    fn new(
        condition_type: ConditionType,
        status: ConditionStatus,
        reason: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            condition_type,
            status,
            reason: reason.into(),
            message: message.into(),
        }
    }

    fn render_summary(&self) -> String {
        format!(
            "{}={}:{}",
            self.condition_type.as_str(),
            self.status.as_str(),
            self.reason
        )
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RetryClass {
    None,
    SpecTerminal,
    Transient,
    AlphaBlocked,
}

impl RetryClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::SpecTerminal => "spec-terminal",
            Self::Transient => "transient",
            Self::AlphaBlocked => "alpha-blocked",
        }
    }

    pub fn action(self, default_requeue: Duration) -> Action {
        match self {
            Self::Transient => Action::requeue(default_requeue),
            Self::None | Self::SpecTerminal | Self::AlphaBlocked => Action::await_change(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ControllerBoundaryPlan {
    pub controller: String,
    pub resource: String,
    pub mode: ExecutionMode,
    pub operations: Vec<BoundaryOperation>,
    pub conditions: Vec<ControllerCondition>,
    pub retry_class: RetryClass,
    pub requeue_seconds: u64,
}

impl ControllerBoundaryPlan {
    pub fn try_new(
        controller: impl Into<String>,
        resource: impl Into<String>,
        mode: ExecutionMode,
        operations: Vec<BoundaryOperation>,
        requeue_after: Duration,
    ) -> Result<Self, BoundaryError> {
        let controller = controller.into();
        let resource = resource.into();
        validate_non_empty("controller", &controller)?;
        validate_non_empty("resource", &resource)?;
        if operations.is_empty() {
            return Err(BoundaryError::NoOperations);
        }
        for operation in &operations {
            validate_non_empty("operation.name", &operation.name)?;
            if matches!(mode, ExecutionMode::Apply) && !operation.implemented {
                return Err(BoundaryError::ApplyRequiresImplementedOperation {
                    operation: operation.name.clone(),
                    kind: operation.kind,
                });
            }
        }

        let conditions = render_conditions(mode, &operations);
        let retry_class = if conditions.iter().any(|condition| {
            matches!(
                condition.condition_type,
                ConditionType::KubernetesApplyAlpha
                    | ConditionType::DirectSqlAlpha
                    | ConditionType::StatusMutationAlpha
            )
        }) {
            RetryClass::AlphaBlocked
        } else {
            RetryClass::None
        };

        Ok(Self {
            controller,
            resource,
            mode,
            operations,
            conditions,
            retry_class,
            requeue_seconds: requeue_after.as_secs(),
        })
    }

    pub fn render_tsv_header() -> &'static str {
        "controller\tresource\tmode\trender_plan\tkubernetes_apply\tdirect_sql\tstatus_mutation\tconditions\tretry_class\trequeue_seconds"
    }

    pub fn render_tsv(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.controller,
            self.resource,
            self.mode.as_str(),
            self.count(BoundaryOperationKind::RenderPlan),
            self.count(BoundaryOperationKind::KubernetesApply),
            self.count(BoundaryOperationKind::DirectSql),
            self.count(BoundaryOperationKind::StatusMutation),
            self.conditions_summary(),
            self.retry_class.as_str(),
            self.requeue_seconds
        )
    }

    fn count(&self, kind: BoundaryOperationKind) -> usize {
        self.operations
            .iter()
            .filter(|operation| operation.kind == kind)
            .count()
    }

    fn conditions_summary(&self) -> String {
        self.conditions
            .iter()
            .map(ControllerCondition::render_summary)
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn render_conditions(
    mode: ExecutionMode,
    operations: &[BoundaryOperation],
) -> Vec<ControllerCondition> {
    let mut conditions = vec![
        ControllerCondition::new(
            ConditionType::SpecAccepted,
            ConditionStatus::True,
            "Validated",
            "spec validation passed before boundary rendering",
        ),
        ControllerCondition::new(
            ConditionType::PlanRendered,
            ConditionStatus::True,
            "Rendered",
            format!("{} operation(s) rendered", operations.len()),
        ),
    ];

    if matches!(mode, ExecutionMode::DryRun) {
        conditions.push(ControllerCondition::new(
            ConditionType::DryRun,
            ConditionStatus::True,
            "NoMutations",
            "dry-run mode records intent without applying Kubernetes objects, SQL, or status",
        ));
    }

    push_alpha_condition(
        &mut conditions,
        operations,
        BoundaryOperationKind::KubernetesApply,
        ConditionType::KubernetesApplyAlpha,
        "Kubernetes apply is alpha until the controller has a real apply runner and RBAC evidence",
    );
    push_alpha_condition(
        &mut conditions,
        operations,
        BoundaryOperationKind::DirectSql,
        ConditionType::DirectSqlAlpha,
        "direct SQL execution is alpha until an idempotent SQL runner is implemented",
    );
    push_alpha_condition(
        &mut conditions,
        operations,
        BoundaryOperationKind::StatusMutation,
        ConditionType::StatusMutationAlpha,
        "status mutation is alpha until status subresource updates are implemented",
    );

    conditions
}

fn push_alpha_condition(
    conditions: &mut Vec<ControllerCondition>,
    operations: &[BoundaryOperation],
    kind: BoundaryOperationKind,
    condition_type: ConditionType,
    message: &'static str,
) {
    if operations
        .iter()
        .any(|operation| operation.kind == kind && !operation.implemented)
    {
        conditions.push(ControllerCondition::new(
            condition_type,
            ConditionStatus::False,
            "AlphaNotImplemented",
            message,
        ));
    }
}

pub fn retry_class_for_error(error: &ControllerError) -> RetryClass {
    match error {
        ControllerError::InvalidSpec(_) => RetryClass::SpecTerminal,
        ControllerError::Boundary(BoundaryError::ApplyRequiresImplementedOperation { .. }) => {
            RetryClass::AlphaBlocked
        }
        ControllerError::Boundary(_) => RetryClass::SpecTerminal,
        ControllerError::Kube(_) | ControllerError::Companion(_) => RetryClass::Transient,
    }
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), BoundaryError> {
    if value.trim().is_empty() {
        return Err(BoundaryError::MissingRequiredField(field));
    }
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BoundaryError {
    InvalidExecutionMode(String),
    MissingRequiredField(&'static str),
    NoOperations,
    ApplyRequiresImplementedOperation {
        operation: String,
        kind: BoundaryOperationKind,
    },
}

impl fmt::Display for BoundaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExecutionMode(mode) => write!(
                formatter,
                "{EXECUTION_MODE_ENV} must be dry-run or apply, got {mode}"
            ),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::NoOperations => write!(
                formatter,
                "boundary plan must include at least one operation"
            ),
            Self::ApplyRequiresImplementedOperation { operation, kind } => write!(
                formatter,
                "apply mode blocked: operation {operation} ({}) is alpha and not implemented",
                kind.as_str()
            ),
        }
    }
}

impl Error for BoundaryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_renders_alpha_conditions_without_failing() {
        let plan = ControllerBoundaryPlan::try_new(
            "Hypertable",
            "metrics",
            ExecutionMode::DryRun,
            vec![
                BoundaryOperation::render_plan("render_hypertable_plan"),
                BoundaryOperation::alpha(
                    "execute_hypertable_sql",
                    BoundaryOperationKind::DirectSql,
                ),
                BoundaryOperation::alpha(
                    "patch_hypertable_status",
                    BoundaryOperationKind::StatusMutation,
                ),
            ],
            Duration::from_secs(30),
        )
        .expect("dry-run boundary plan");

        assert_eq!(plan.retry_class, RetryClass::AlphaBlocked);
        assert_eq!(
            plan.render_tsv(),
            "Hypertable	metrics	dry-run	1	0	1	1	SpecAccepted=True:Validated,PlanRendered=True:Rendered,DryRun=True:NoMutations,DirectSqlAlpha=False:AlphaNotImplemented,StatusMutationAlpha=False:AlphaNotImplemented	alpha-blocked	30"
        );
    }

    #[test]
    fn apply_mode_fails_closed_for_alpha_operations() {
        let error = ControllerBoundaryPlan::try_new(
            "Migration",
            "users-display-name",
            ExecutionMode::Apply,
            vec![
                BoundaryOperation::render_plan("render_migration_plan"),
                BoundaryOperation::alpha(
                    "patch_migration_status",
                    BoundaryOperationKind::StatusMutation,
                ),
            ],
            Duration::from_secs(30),
        )
        .expect_err("apply must be blocked");

        assert_eq!(
            error,
            BoundaryError::ApplyRequiresImplementedOperation {
                operation: "patch_migration_status".to_string(),
                kind: BoundaryOperationKind::StatusMutation,
            }
        );
    }

    #[test]
    fn invalid_execution_mode_is_rejected() {
        assert_eq!(
            ExecutionMode::from_str("status-patch"),
            Err(BoundaryError::InvalidExecutionMode(
                "status-patch".to_string()
            ))
        );
    }
}
