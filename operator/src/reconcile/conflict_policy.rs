// FEATURE: C4
// FEATURE: C5

use std::error::Error;
use std::fmt;

use crate::crds::conflict_policy::{
    ConflictClass, ConflictPolicySpec, ConflictPolicySpecError, ConflictResolution,
};

pub const CONFLICT_POLICY_TABLE: &str = "companion_internal.replication_conflict_policies";
pub const CONFLICT_STATUS_TABLE: &str = "companion_internal.replication_conflict_status";

const CONFLICT_POLICY_TABLES_SQL: &str = r#"CREATE TABLE IF NOT EXISTS companion_internal.replication_conflict_policies (
    policy_name text PRIMARY KEY,
    table_name text NOT NULL,
    conflict_class text NOT NULL,
    resolution text NOT NULL,
    custom_function text,
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS companion_internal.replication_conflict_status (
    policy_name text NOT NULL REFERENCES companion_internal.replication_conflict_policies(policy_name) ON DELETE CASCADE,
    conflict_class text NOT NULL,
    conflict_count bigint NOT NULL DEFAULT 0,
    last_conflict_at timestamptz,
    PRIMARY KEY (policy_name, conflict_class)
);"#;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConflictPolicyReconcilePlan {
    pub policy_name: String,
    pub table: String,
    pub class: ConflictClass,
    pub resolution: ConflictResolution,
    pub custom_function: Option<String>,
}

impl ConflictPolicyReconcilePlan {
    pub fn from_spec(
        policy_name: &str,
        spec: &ConflictPolicySpec,
    ) -> Result<Self, ConflictPolicyReconcileError> {
        let trimmed_name = policy_name.trim();
        if trimmed_name.is_empty() {
            return Err(ConflictPolicyReconcileError::MissingPolicyName);
        }
        spec.validate()?;

        Ok(Self {
            policy_name: trimmed_name.to_string(),
            table: spec.table.clone(),
            class: spec.class,
            resolution: spec.resolution,
            custom_function: spec.custom_function.clone(),
        })
    }

    pub fn class_str(&self) -> &'static str {
        match self.class {
            ConflictClass::InsertInsert => "insert_exists",
            ConflictClass::InsertUpdate => "update_exists",
            ConflictClass::UpdateUpdate => "update_origin_differs",
            ConflictClass::UpdateDelete => "update_missing",
            ConflictClass::DeleteUpdate => "delete_origin_differs",
            ConflictClass::DeleteDelete => "delete_missing",
            ConflictClass::Constraint => "delete_exists",
        }
    }

    pub fn resolution_str(&self) -> &'static str {
        match self.resolution {
            ConflictResolution::LastWriteWins => "apply_remote_if_newer",
            ConflictResolution::OriginWins => "apply_remote",
            ConflictResolution::TargetWins | ConflictResolution::Skip => "keep_local",
            ConflictResolution::Error => "error",
            ConflictResolution::CustomFunction => "merge_function",
        }
    }

    pub fn apply_plan(&self) -> ConflictPolicyApplyPlan {
        let mut steps = vec![
            ConflictPolicyApplyStep::new(
                "ensure_ai_blaise_citus_extension",
                "CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;",
                true,
            ),
            ConflictPolicyApplyStep::new(
                "ensure_conflict_policy_tables",
                CONFLICT_POLICY_TABLES_SQL,
                true,
            ),
        ];

        if let Some(custom_function) = &self.custom_function {
            steps.push(ConflictPolicyApplyStep::new(
                "validate_custom_function",
                validate_custom_function_sql(custom_function),
                true,
            ));
        }

        steps.push(ConflictPolicyApplyStep::new(
            "upsert_conflict_policy",
            upsert_policy_sql(self),
            true,
        ));

        ConflictPolicyApplyPlan { steps }
    }

    pub fn apply_sql_script(&self) -> String {
        self.apply_plan().sql_script()
    }

    pub fn teardown_sql(&self) -> String {
        format!(
            "DELETE FROM {table} WHERE policy_name = {name};",
            table = CONFLICT_POLICY_TABLE,
            name = sql_literal(&self.policy_name),
        )
    }

    pub fn status_sql(&self) -> String {
        format!(
            "SELECT conflict_class, conflict_count, last_conflict_at FROM {table} WHERE policy_name = {name};",
            table = CONFLICT_STATUS_TABLE,
            name = sql_literal(&self.policy_name),
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConflictPolicyApplyStep {
    pub name: String,
    pub sql: String,
    pub idempotent: bool,
}

impl ConflictPolicyApplyStep {
    fn new(name: impl Into<String>, sql: impl Into<String>, idempotent: bool) -> Self {
        Self {
            name: name.into(),
            sql: sql.into(),
            idempotent,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConflictPolicyApplyPlan {
    pub steps: Vec<ConflictPolicyApplyStep>,
}

impl ConflictPolicyApplyPlan {
    pub fn sql_script(&self) -> String {
        self.steps
            .iter()
            .map(|step| step.sql.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn upsert_policy_sql(plan: &ConflictPolicyReconcilePlan) -> String {
    format!(
        r#"INSERT INTO {table}(policy_name, table_name, conflict_class, resolution, custom_function)
VALUES ({name}, {target_table}, {class}, {resolution}, {custom})
ON CONFLICT (policy_name) DO UPDATE
SET table_name = EXCLUDED.table_name,
    conflict_class = EXCLUDED.conflict_class,
    resolution = EXCLUDED.resolution,
    custom_function = EXCLUDED.custom_function,
    updated_at = now();"#,
        table = CONFLICT_POLICY_TABLE,
        name = sql_literal(&plan.policy_name),
        target_table = sql_literal(&plan.table),
        class = sql_literal(plan.class_str()),
        resolution = sql_literal(plan.resolution_str()),
        custom = optional_sql_literal(&plan.custom_function),
    )
}

fn validate_custom_function_sql(function: &str) -> String {
    let signature = format!("{function}(jsonb,jsonb)");
    format!(
        r#"DO $ai_blaise_conflict_policy$
BEGIN
    IF to_regprocedure({signature}) IS NULL THEN
        RAISE EXCEPTION 'conflict policy custom function % must exist with (jsonb, jsonb) signature', {function};
    END IF;
END
$ai_blaise_conflict_policy$;"#,
        signature = sql_literal(&signature),
        function = sql_literal(function),
    )
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn optional_sql_literal(value: &Option<String>) -> String {
    value
        .as_deref()
        .map(sql_literal)
        .unwrap_or_else(|| "NULL".to_string())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ConflictPolicyReconcileError {
    InvalidSpec(ConflictPolicySpecError),
    MissingPolicyName,
}

impl fmt::Display for ConflictPolicyReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(error) => write!(formatter, "{error}"),
            Self::MissingPolicyName => write!(formatter, "policy_name must not be empty"),
        }
    }
}

impl Error for ConflictPolicyReconcileError {}

impl From<ConflictPolicySpecError> for ConflictPolicyReconcileError {
    fn from(error: ConflictPolicySpecError) -> Self {
        Self::InvalidSpec(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_update_plan_renders_last_write_wins_resolution() {
        let spec = ConflictPolicySpec {
            table: "public.reference_accounts".to_string(),
            class: ConflictClass::UpdateUpdate,
            resolution: ConflictResolution::LastWriteWins,
            custom_function: None,
        };

        let plan =
            ConflictPolicyReconcilePlan::from_spec("accounts-lww", &spec).expect("valid plan");

        assert_eq!(plan.policy_name, "accounts-lww");
        assert_eq!(plan.class_str(), "update_origin_differs");
        assert_eq!(plan.resolution_str(), "apply_remote_if_newer");

        let apply_plan = plan.apply_plan();
        assert_eq!(apply_plan.steps.len(), 3);
        assert!(apply_plan.sql_script().contains(CONFLICT_POLICY_TABLE));
        assert!(apply_plan
            .sql_script()
            .contains("'public.reference_accounts'"));
        assert!(apply_plan.sql_script().contains("'update_origin_differs'"));
        assert!(apply_plan.sql_script().contains("'apply_remote_if_newer'"));
        assert!(apply_plan.sql_script().contains(", NULL)"));

        let teardown = plan.teardown_sql();
        assert!(teardown.contains(CONFLICT_POLICY_TABLE));

        let status = plan.status_sql();
        assert!(status.contains(CONFLICT_STATUS_TABLE));
    }

    #[test]
    fn custom_function_plan_inserts_validation_step() {
        let spec = ConflictPolicySpec {
            table: "public.merge_targets".to_string(),
            class: ConflictClass::InsertUpdate,
            resolution: ConflictResolution::CustomFunction,
            custom_function: Some("public.merge_remote_into_local".to_string()),
        };

        let plan =
            ConflictPolicyReconcilePlan::from_spec("merge-policy", &spec).expect("valid plan");

        assert_eq!(plan.class_str(), "update_exists");
        assert_eq!(plan.resolution_str(), "merge_function");

        let apply_plan = plan.apply_plan();
        assert_eq!(apply_plan.steps.len(), 4);
        assert_eq!(apply_plan.steps[2].name, "validate_custom_function");
        assert!(apply_plan.steps[2].sql.contains("to_regprocedure"));
        assert!(apply_plan.steps[2]
            .sql
            .contains("'public.merge_remote_into_local(jsonb,jsonb)'"));
    }

    #[test]
    fn all_seven_classes_map_to_spock_strings() {
        let cases = [
            (ConflictClass::InsertInsert, "insert_exists"),
            (ConflictClass::InsertUpdate, "update_exists"),
            (ConflictClass::UpdateUpdate, "update_origin_differs"),
            (ConflictClass::UpdateDelete, "update_missing"),
            (ConflictClass::DeleteUpdate, "delete_origin_differs"),
            (ConflictClass::DeleteDelete, "delete_missing"),
            (ConflictClass::Constraint, "delete_exists"),
        ];

        for (class, expected) in cases {
            let spec = ConflictPolicySpec {
                table: "public.t".to_string(),
                class,
                resolution: ConflictResolution::LastWriteWins,
                custom_function: None,
            };
            let plan = ConflictPolicyReconcilePlan::from_spec("policy", &spec).expect("valid plan");
            assert_eq!(plan.class_str(), expected, "class string mismatch");
        }
    }

    #[test]
    fn all_resolutions_map_to_companion_strings() {
        let cases = [
            (ConflictResolution::LastWriteWins, "apply_remote_if_newer"),
            (ConflictResolution::OriginWins, "apply_remote"),
            (ConflictResolution::TargetWins, "keep_local"),
            (ConflictResolution::Skip, "keep_local"),
            (ConflictResolution::Error, "error"),
            (ConflictResolution::CustomFunction, "merge_function"),
        ];

        for (resolution, expected) in cases {
            let spec = ConflictPolicySpec {
                table: "public.t".to_string(),
                class: ConflictClass::UpdateUpdate,
                resolution,
                custom_function: matches!(resolution, ConflictResolution::CustomFunction)
                    .then(|| "public.merge".to_string()),
            };
            let plan = ConflictPolicyReconcilePlan::from_spec("policy", &spec).expect("valid plan");
            assert_eq!(plan.resolution_str(), expected, "resolution mismatch");
        }
    }

    #[test]
    fn empty_policy_name_is_rejected() {
        let spec = ConflictPolicySpec {
            table: "public.t".to_string(),
            class: ConflictClass::UpdateUpdate,
            resolution: ConflictResolution::LastWriteWins,
            custom_function: None,
        };

        assert_eq!(
            ConflictPolicyReconcilePlan::from_spec("  ", &spec),
            Err(ConflictPolicyReconcileError::MissingPolicyName)
        );
    }

    #[test]
    fn missing_custom_function_propagates_validation_error() {
        let spec = ConflictPolicySpec {
            table: "public.t".to_string(),
            class: ConflictClass::UpdateUpdate,
            resolution: ConflictResolution::CustomFunction,
            custom_function: None,
        };

        assert_eq!(
            ConflictPolicyReconcilePlan::from_spec("policy", &spec),
            Err(ConflictPolicyReconcileError::InvalidSpec(
                ConflictPolicySpecError::MissingRequiredField("custom_function")
            ))
        );
    }

    #[test]
    fn quoted_table_is_escaped_for_sql() {
        let spec = ConflictPolicySpec {
            table: "public.\"weird'name\"".to_string(),
            class: ConflictClass::UpdateUpdate,
            resolution: ConflictResolution::LastWriteWins,
            custom_function: None,
        };

        let plan = ConflictPolicyReconcilePlan::from_spec("policy", &spec).expect("valid plan");
        assert!(plan.apply_sql_script().contains("'public.\"weird''name\"'"));
    }
}
