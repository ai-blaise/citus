// FEATURE: PM3
// FEATURE: PM4

use crate::{PlanFreezeError, PlanFreezePlan, PlanRegressionSample};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanRuntimeConfig {
    pub max_attempts: u8,
    pub retry_backoff_ms: u64,
}

impl PlanRuntimeConfig {
    pub fn validate(&self) -> Result<(), PlanRuntimeError> {
        if self.max_attempts == 0 {
            return Err(PlanRuntimeError::InvalidConfig("max_attempts"));
        }
        if self.retry_backoff_ms == 0 {
            return Err(PlanRuntimeError::InvalidConfig("retry_backoff_ms"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanRuntimeRequest {
    pub idempotency_key: String,
    pub command: PlanRuntimeCommand,
}

impl PlanRuntimeRequest {
    pub fn validate(&self) -> Result<(), PlanRuntimeError> {
        validate_required("idempotency_key", &self.idempotency_key)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlanRuntimeCommand {
    RegisterPlan {
        plan: PlanFreezePlan,
    },
    RecordObservation {
        sample: PlanRegressionSample,
        executions: u32,
        stable_days: u32,
    },
    EvaluatePromotion {
        query_hash: String,
    },
    EvaluateRegression {
        sample: PlanRegressionSample,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanRuntimeRecord {
    pub plan: PlanFreezePlan,
    pub executions: u32,
    pub stable_days: u32,
    pub baseline_p95_ms: u64,
    pub baseline_cost: u64,
    pub promoted: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanRuntimeOutcome {
    pub query_hash: String,
    pub action: &'static str,
    pub accepted: bool,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanRuntimeAuditEvent {
    pub sequence: u64,
    pub idempotency_key: String,
    pub query_hash: String,
    pub action: &'static str,
    pub outcome: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanRuntimeReport {
    pub records: usize,
    pub promoted: usize,
    pub observations: usize,
    pub audit_events: usize,
    pub idempotent_replays: usize,
    pub retry_attempts: usize,
    pub failed_commands: usize,
    pub regression_violations: usize,
    pub sql_contract_commands: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanRuntimeSqlPlan {
    pub feature_ids: Vec<&'static str>,
    pub commands: Vec<String>,
}

impl PlanRuntimeSqlPlan {
    pub fn canonical() -> Self {
        Self {
            feature_ids: vec!["PM3", "PM4"],
            commands: vec![
                "CREATE TABLE IF NOT EXISTS companion_internal.plan_runtime_commands (idempotency_key text PRIMARY KEY, query_hash text NOT NULL, command_kind text NOT NULL, outcome text NOT NULL, attempts integer NOT NULL, updated_at timestamptz NOT NULL DEFAULT now());".to_string(),
                "CREATE TABLE IF NOT EXISTS companion_internal.plan_runtime_audit (sequence bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY, idempotency_key text NOT NULL, query_hash text NOT NULL, action text NOT NULL, outcome text NOT NULL, detail text NOT NULL, created_at timestamptz NOT NULL DEFAULT now());".to_string(),
                "CREATE UNIQUE INDEX IF NOT EXISTS plan_runtime_commands_query_idempotency_idx ON companion_internal.plan_runtime_commands (query_hash, idempotency_key);".to_string(),
                "CREATE VIEW companion_plan_runtime_audit AS SELECT sequence, idempotency_key, query_hash, action, outcome, detail, created_at FROM companion_internal.plan_runtime_audit;".to_string(),
                "CREATE FUNCTION companion_internal.plan_runtime_retry_due(attempts integer, max_attempts integer) RETURNS boolean LANGUAGE sql IMMUTABLE AS $$ SELECT attempts < max_attempts $$;".to_string(),
            ],
        }
    }

    pub fn validate(&self) -> Result<(), PlanRuntimeError> {
        if self.feature_ids != ["PM3", "PM4"] {
            return Err(PlanRuntimeError::InvalidSqlContract("feature_ids"));
        }
        if self.commands.len() != 5
            || self
                .commands
                .iter()
                .any(|command| command.trim().is_empty())
        {
            return Err(PlanRuntimeError::InvalidSqlContract("commands"));
        }
        Ok(())
    }

    pub fn script(&self) -> String {
        self.commands.join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanRuntime {
    config: PlanRuntimeConfig,
    records: BTreeMap<String, PlanRuntimeRecord>,
    idempotency: BTreeMap<String, PlanRuntimeOutcome>,
    audit: Vec<PlanRuntimeAuditEvent>,
    observations: usize,
    idempotent_replays: usize,
    retry_attempts: usize,
    failed_commands: usize,
    regression_violations: usize,
}

impl PlanRuntime {
    pub fn new(config: PlanRuntimeConfig) -> Result<Self, PlanRuntimeError> {
        config.validate()?;
        Ok(Self {
            config,
            records: BTreeMap::new(),
            idempotency: BTreeMap::new(),
            audit: Vec::new(),
            observations: 0,
            idempotent_replays: 0,
            retry_attempts: 0,
            failed_commands: 0,
            regression_violations: 0,
        })
    }

    pub fn execute(
        &mut self,
        request: PlanRuntimeRequest,
    ) -> Result<PlanRuntimeOutcome, PlanRuntimeError> {
        self.execute_with_retry(request, 0)
    }

    pub fn execute_with_retry(
        &mut self,
        request: PlanRuntimeRequest,
        transient_failures: u8,
    ) -> Result<PlanRuntimeOutcome, PlanRuntimeError> {
        request.validate()?;
        if let Some(outcome) = self.idempotency.get(&request.idempotency_key).cloned() {
            self.idempotent_replays += 1;
            let mut replay = outcome;
            replay.idempotent_replay = true;
            self.record_audit(
                &request.idempotency_key,
                &replay.query_hash,
                replay.action,
                "replayed",
                "idempotency key returned the original durable outcome".to_string(),
            );
            return Ok(replay);
        }

        let query_hash = command_query_hash(&request.command);
        for attempt in 1..=self.config.max_attempts {
            if attempt <= transient_failures {
                self.retry_attempts += 1;
                self.record_audit(
                    &request.idempotency_key,
                    &query_hash,
                    "retry",
                    "scheduled",
                    format!(
                        "transient failure before attempt {attempt}; retry_backoff_ms={}",
                        self.config.retry_backoff_ms
                    ),
                );
                continue;
            }

            let outcome = self.apply_command(&request.command);
            match outcome {
                Ok(outcome) => {
                    self.idempotency
                        .insert(request.idempotency_key.clone(), outcome.clone());
                    self.record_audit(
                        &request.idempotency_key,
                        &outcome.query_hash,
                        outcome.action,
                        "applied",
                        "command durably applied".to_string(),
                    );
                    return Ok(outcome);
                }
                Err(error) => {
                    self.failed_commands += 1;
                    self.record_audit(
                        &request.idempotency_key,
                        &query_hash,
                        "failed",
                        "rejected",
                        error.to_string(),
                    );
                    return Err(error);
                }
            }
        }

        self.failed_commands += 1;
        let error = PlanRuntimeError::RetryExhausted;
        self.record_audit(
            &request.idempotency_key,
            &query_hash,
            "retry",
            "exhausted",
            error.to_string(),
        );
        Err(error)
    }

    pub fn report(&self) -> PlanRuntimeReport {
        PlanRuntimeReport {
            records: self.records.len(),
            promoted: self
                .records
                .values()
                .filter(|record| record.promoted)
                .count(),
            observations: self.observations,
            audit_events: self.audit.len(),
            idempotent_replays: self.idempotent_replays,
            retry_attempts: self.retry_attempts,
            failed_commands: self.failed_commands,
            regression_violations: self.regression_violations,
            sql_contract_commands: PlanRuntimeSqlPlan::canonical().commands.len(),
        }
    }

    pub fn audit_events(&self) -> &[PlanRuntimeAuditEvent] {
        &self.audit
    }

    fn apply_command(
        &mut self,
        command: &PlanRuntimeCommand,
    ) -> Result<PlanRuntimeOutcome, PlanRuntimeError> {
        match command {
            PlanRuntimeCommand::RegisterPlan { plan } => {
                plan.validate().map_err(PlanRuntimeError::PlanFreeze)?;
                if self.records.contains_key(&plan.query_hash) {
                    return Err(PlanRuntimeError::DuplicatePlan(plan.query_hash.clone()));
                }
                self.records.insert(
                    plan.query_hash.clone(),
                    PlanRuntimeRecord {
                        plan: plan.clone(),
                        executions: 0,
                        stable_days: 0,
                        baseline_p95_ms: 0,
                        baseline_cost: 0,
                        promoted: false,
                    },
                );
                Ok(outcome(&plan.query_hash, "registered", true))
            }
            PlanRuntimeCommand::RecordObservation {
                sample,
                executions,
                stable_days,
            } => {
                sample.validate().map_err(PlanRuntimeError::PlanFreeze)?;
                if *executions == 0 {
                    return Err(PlanRuntimeError::InvalidObservation("executions"));
                }
                let record = self
                    .records
                    .get_mut(&sample.query_hash)
                    .ok_or_else(|| PlanRuntimeError::UnknownPlan(sample.query_hash.clone()))?;
                record.executions = *executions;
                record.stable_days = *stable_days;
                record.baseline_p95_ms = sample.baseline_p95_ms;
                record.baseline_cost = sample.baseline_cost;
                self.observations += 1;
                Ok(outcome(&sample.query_hash, "observed", true))
            }
            PlanRuntimeCommand::EvaluatePromotion { query_hash } => {
                validate_required("query_hash", query_hash)?;
                let record = self
                    .records
                    .get_mut(query_hash)
                    .ok_or_else(|| PlanRuntimeError::UnknownPlan(query_hash.clone()))?;
                if record.executions >= record.plan.promotion.min_executions
                    && record.stable_days >= record.plan.promotion.stable_days
                {
                    record.promoted = true;
                    Ok(outcome(query_hash, "promoted", true))
                } else {
                    Ok(outcome(query_hash, "deferred", false))
                }
            }
            PlanRuntimeCommand::EvaluateRegression { sample } => {
                let record = self
                    .records
                    .get(&sample.query_hash)
                    .ok_or_else(|| PlanRuntimeError::UnknownPlan(sample.query_hash.clone()))?;
                let violates = sample
                    .violates(&record.plan.regression)
                    .map_err(PlanRuntimeError::PlanFreeze)?;
                if violates {
                    self.regression_violations += 1;
                    Ok(outcome(&sample.query_hash, "candidate_rejected", false))
                } else {
                    Ok(outcome(&sample.query_hash, "candidate_accepted", true))
                }
            }
        }
    }

    fn record_audit(
        &mut self,
        idempotency_key: &str,
        query_hash: &str,
        action: &'static str,
        outcome: &'static str,
        detail: String,
    ) {
        self.audit.push(PlanRuntimeAuditEvent {
            sequence: self.audit.len() as u64 + 1,
            idempotency_key: idempotency_key.to_string(),
            query_hash: query_hash.to_string(),
            action,
            outcome,
            detail,
        });
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlanRuntimeError {
    DuplicatePlan(String),
    InvalidConfig(&'static str),
    InvalidObservation(&'static str),
    InvalidSqlContract(&'static str),
    MissingRequiredField(&'static str),
    PlanFreeze(PlanFreezeError),
    RetryExhausted,
    UnknownPlan(String),
}

impl fmt::Display for PlanRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePlan(query_hash) => {
                write!(formatter, "plan already registered: {query_hash}")
            }
            Self::InvalidConfig(field) => write!(formatter, "{field} must be greater than zero"),
            Self::InvalidObservation(field) => {
                write!(formatter, "{field} must be greater than zero")
            }
            Self::InvalidSqlContract(field) => {
                write!(formatter, "invalid plan runtime SQL contract: {field}")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::PlanFreeze(error) => write!(formatter, "{error}"),
            Self::RetryExhausted => write!(formatter, "retry attempts exhausted"),
            Self::UnknownPlan(query_hash) => write!(formatter, "unknown plan: {query_hash}"),
        }
    }
}

impl Error for PlanRuntimeError {}

pub fn canonical_plan_runtime_report() -> Result<PlanRuntimeReport, PlanRuntimeError> {
    let contract = PlanRuntimeSqlPlan::canonical();
    contract.validate()?;

    let mut runtime = PlanRuntime::new(PlanRuntimeConfig {
        max_attempts: 3,
        retry_backoff_ms: 25,
    })?;
    let plan = canonical_plan();

    runtime.execute(request(
        "register-abc123",
        PlanRuntimeCommand::RegisterPlan { plan: plan.clone() },
    ))?;
    runtime.execute(request(
        "register-abc123",
        PlanRuntimeCommand::RegisterPlan { plan: plan.clone() },
    ))?;
    runtime.execute(request(
        "observe-abc123",
        PlanRuntimeCommand::RecordObservation {
            sample: PlanRegressionSample {
                query_hash: plan.query_hash.clone(),
                baseline_p95_ms: 100,
                candidate_p95_ms: 100,
                baseline_cost: 1_000,
                candidate_cost: 1_000,
            },
            executions: 100,
            stable_days: 7,
        },
    ))?;
    runtime.execute(request(
        "promote-abc123",
        PlanRuntimeCommand::EvaluatePromotion {
            query_hash: plan.query_hash.clone(),
        },
    ))?;
    runtime.execute(request(
        "regression-allowed-abc123",
        PlanRuntimeCommand::EvaluateRegression {
            sample: PlanRegressionSample {
                query_hash: plan.query_hash.clone(),
                baseline_p95_ms: 100,
                candidate_p95_ms: 105,
                baseline_cost: 1_000,
                candidate_cost: 1_100,
            },
        },
    ))?;
    let _ = runtime.execute_with_retry(
        request(
            "regression-blocked-abc123",
            PlanRuntimeCommand::EvaluateRegression {
                sample: PlanRegressionSample {
                    query_hash: plan.query_hash.clone(),
                    baseline_p95_ms: 100,
                    candidate_p95_ms: 130,
                    baseline_cost: 1_000,
                    candidate_cost: 1_000,
                },
            },
        ),
        1,
    )?;
    let missing = runtime.execute(request(
        "missing-plan",
        PlanRuntimeCommand::EvaluatePromotion {
            query_hash: "missing-query-hash".to_string(),
        },
    ));
    if !matches!(missing, Err(PlanRuntimeError::UnknownPlan(_))) {
        return Err(PlanRuntimeError::InvalidSqlContract("missing-plan-guard"));
    }

    Ok(runtime.report())
}

pub fn canonical_plan_runtime_sql_plan() -> Result<PlanRuntimeSqlPlan, PlanRuntimeError> {
    let plan = PlanRuntimeSqlPlan::canonical();
    plan.validate()?;
    Ok(plan)
}

fn canonical_plan() -> PlanFreezePlan {
    PlanFreezePlan {
        query_hash: "abc123".to_string(),
        plan_xml: "<Plan />".to_string(),
        hint_set_name: "stable_orders_plan".to_string(),
        promotion: crate::PlanPromotionPolicy {
            min_executions: 100,
            stable_days: 7,
        },
        regression: crate::PlanRegressionPolicy {
            max_latency_regression_percent: 10,
            max_cost_regression_percent: 20,
        },
    }
}

fn request(idempotency_key: &str, command: PlanRuntimeCommand) -> PlanRuntimeRequest {
    PlanRuntimeRequest {
        idempotency_key: idempotency_key.to_string(),
        command,
    }
}

fn outcome(query_hash: &str, action: &'static str, accepted: bool) -> PlanRuntimeOutcome {
    PlanRuntimeOutcome {
        query_hash: query_hash.to_string(),
        action,
        accepted,
        idempotent_replay: false,
    }
}

fn command_query_hash(command: &PlanRuntimeCommand) -> String {
    match command {
        PlanRuntimeCommand::RegisterPlan { plan } => plan.query_hash.clone(),
        PlanRuntimeCommand::RecordObservation { sample, .. } => sample.query_hash.clone(),
        PlanRuntimeCommand::EvaluatePromotion { query_hash } => query_hash.clone(),
        PlanRuntimeCommand::EvaluateRegression { sample } => sample.query_hash.clone(),
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), PlanRuntimeError> {
    if value.trim().is_empty() {
        return Err(PlanRuntimeError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_runtime_report_covers_durable_controls() {
        let report = canonical_plan_runtime_report().expect("runtime report");

        assert_eq!(report.records, 1);
        assert_eq!(report.promoted, 1);
        assert_eq!(report.observations, 1);
        assert_eq!(report.audit_events, 8);
        assert_eq!(report.idempotent_replays, 1);
        assert_eq!(report.retry_attempts, 1);
        assert_eq!(report.failed_commands, 1);
        assert_eq!(report.regression_violations, 1);
        assert_eq!(report.sql_contract_commands, 5);
    }

    #[test]
    fn duplicate_idempotency_key_replays_without_mutating_state() {
        let mut runtime = PlanRuntime::new(PlanRuntimeConfig {
            max_attempts: 2,
            retry_backoff_ms: 10,
        })
        .unwrap();
        let plan = canonical_plan();

        let first = runtime
            .execute(request(
                "register-1",
                PlanRuntimeCommand::RegisterPlan { plan: plan.clone() },
            ))
            .unwrap();
        let second = runtime
            .execute(request(
                "register-1",
                PlanRuntimeCommand::RegisterPlan { plan },
            ))
            .unwrap();

        assert_eq!(first.action, "registered");
        assert!(second.idempotent_replay);
        assert_eq!(runtime.report().records, 1);
        assert_eq!(runtime.report().idempotent_replays, 1);
    }

    #[test]
    fn transient_failures_are_bounded_by_retry_policy() {
        let mut runtime = PlanRuntime::new(PlanRuntimeConfig {
            max_attempts: 2,
            retry_backoff_ms: 10,
        })
        .unwrap();

        let error = runtime
            .execute_with_retry(
                request(
                    "register-exhausted",
                    PlanRuntimeCommand::RegisterPlan {
                        plan: canonical_plan(),
                    },
                ),
                2,
            )
            .unwrap_err();

        assert_eq!(error, PlanRuntimeError::RetryExhausted);
        assert_eq!(runtime.report().retry_attempts, 2);
        assert_eq!(runtime.report().failed_commands, 1);
    }

    #[test]
    fn sql_contract_has_five_durable_runtime_commands() {
        let plan = canonical_plan_runtime_sql_plan().expect("sql contract");

        assert_eq!(plan.commands.len(), 5);
        assert!(plan.script().contains("plan_runtime_commands"));
        assert!(plan.script().contains("plan_runtime_audit"));
    }
}
