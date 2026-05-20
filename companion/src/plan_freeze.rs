// FEATURE: PM3
// FEATURE: PM4

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanFreezePlan {
    pub query_hash: String,
    pub plan_xml: String,
    pub hint_set_name: String,
    pub promotion: PlanPromotionPolicy,
    pub regression: PlanRegressionPolicy,
}

impl PlanFreezePlan {
    pub fn validate(&self) -> Result<(), PlanFreezeError> {
        validate_required("query_hash", &self.query_hash)?;
        validate_required("plan_xml", &self.plan_xml)?;
        validate_required("hint_set_name", &self.hint_set_name)?;
        self.promotion.validate()?;
        self.regression.validate()
    }

    pub fn to_sql_plan(&self) -> Result<PlanFreezeSqlPlan, PlanFreezeError> {
        self.validate()?;
        PlanFreezeSqlPlan::new(
            "PM3",
            vec![
                format!(
                    "SELECT companion_internal.plan_freeze({}, {}, {});",
                    sql_literal(&self.query_hash),
                    sql_literal(&self.plan_xml),
                    sql_literal(&self.hint_set_name)
                ),
                format!(
                    "SELECT companion_internal.plan_auto_promote({}, {}, {});",
                    sql_literal(&self.query_hash),
                    self.promotion.min_executions,
                    self.promotion.stable_days
                ),
                format!(
                    "SELECT companion_internal.plan_regression_guard({}, {}, {});",
                    sql_literal(&self.query_hash),
                    self.regression.max_latency_regression_percent,
                    self.regression.max_cost_regression_percent
                ),
            ],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanPromotionPolicy {
    pub min_executions: u32,
    pub stable_days: u32,
}

impl PlanPromotionPolicy {
    fn validate(&self) -> Result<(), PlanFreezeError> {
        if self.min_executions == 0 {
            return Err(PlanFreezeError::InvalidPromotionPolicy("min_executions"));
        }
        if self.stable_days == 0 {
            return Err(PlanFreezeError::InvalidPromotionPolicy("stable_days"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanRegressionPolicy {
    pub max_latency_regression_percent: u32,
    pub max_cost_regression_percent: u32,
}

impl PlanRegressionPolicy {
    fn validate(&self) -> Result<(), PlanFreezeError> {
        if self.max_latency_regression_percent == 0 {
            return Err(PlanFreezeError::InvalidRegressionPolicy(
                "max_latency_regression_percent",
            ));
        }
        if self.max_cost_regression_percent == 0 {
            return Err(PlanFreezeError::InvalidRegressionPolicy(
                "max_cost_regression_percent",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanRegressionSample {
    pub query_hash: String,
    pub baseline_p95_ms: u64,
    pub candidate_p95_ms: u64,
    pub baseline_cost: u64,
    pub candidate_cost: u64,
}

impl PlanRegressionSample {
    pub fn validate(&self) -> Result<(), PlanFreezeError> {
        validate_required("query_hash", &self.query_hash)?;
        if self.baseline_p95_ms == 0 || self.baseline_cost == 0 {
            return Err(PlanFreezeError::InvalidBaseline);
        }
        Ok(())
    }

    pub fn violates(&self, policy: &PlanRegressionPolicy) -> Result<bool, PlanFreezeError> {
        self.validate()?;
        policy.validate()?;
        Ok(percent_regressed(
            self.baseline_p95_ms,
            self.candidate_p95_ms,
            policy.max_latency_regression_percent,
        ) || percent_regressed(
            self.baseline_cost,
            self.candidate_cost,
            policy.max_cost_regression_percent,
        ))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanFreezeSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl PlanFreezeSqlPlan {
    fn new(feature_id: &'static str, commands: Vec<String>) -> Result<Self, PlanFreezeError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(PlanFreezeError::MissingRequiredField("commands"));
        }
        Ok(Self {
            feature_id,
            commands,
        })
    }

    pub fn script(&self) -> String {
        self.commands.join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlanFreezeError {
    InvalidBaseline,
    InvalidPromotionPolicy(&'static str),
    InvalidRegressionPolicy(&'static str),
    MissingRequiredField(&'static str),
}

impl fmt::Display for PlanFreezeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBaseline => write!(formatter, "baseline values must be greater than zero"),
            Self::InvalidPromotionPolicy(field) => {
                write!(formatter, "{field} must be greater than zero")
            }
            Self::InvalidRegressionPolicy(field) => {
                write!(formatter, "{field} must be greater than zero")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for PlanFreezeError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), PlanFreezeError> {
    if value.trim().is_empty() {
        return Err(PlanFreezeError::MissingRequiredField(field));
    }
    Ok(())
}

fn percent_regressed(baseline: u64, candidate: u64, max_percent: u32) -> bool {
    let allowed = baseline as u128 * (100 + max_percent as u128);
    candidate as u128 * 100 > allowed
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_freeze_renders_freeze_and_guards() {
        let plan = PlanFreezePlan {
            query_hash: "abc123".to_string(),
            plan_xml: "<Plan />".to_string(),
            hint_set_name: "stable_orders_plan".to_string(),
            promotion: PlanPromotionPolicy {
                min_executions: 100,
                stable_days: 7,
            },
            regression: PlanRegressionPolicy {
                max_latency_regression_percent: 10,
                max_cost_regression_percent: 20,
            },
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "PM3");
        assert!(plan.script().contains("plan_freeze"));
        assert!(plan.script().contains("plan_regression_guard"));
    }

    #[test]
    fn regression_sample_detects_latency_regression() {
        let policy = PlanRegressionPolicy {
            max_latency_regression_percent: 10,
            max_cost_regression_percent: 10,
        };
        let sample = PlanRegressionSample {
            query_hash: "abc123".to_string(),
            baseline_p95_ms: 100,
            candidate_p95_ms: 112,
            baseline_cost: 1000,
            candidate_cost: 1000,
        };

        assert_eq!(sample.violates(&policy), Ok(true));
    }
}
