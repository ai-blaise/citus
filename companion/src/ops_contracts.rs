// FEATURE: D7
// FEATURE: D8
// FEATURE: D9
// FEATURE: D10
// FEATURE: D11
// FEATURE: A9
// FEATURE: MR9
// FEATURE: RT5
// FEATURE: S7
// FEATURE: Sec7
// FEATURE: Sec8
// FEATURE: Sec9
// FEATURE: Sec13
// FEATURE: T6
// FEATURE: T7

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub const OPERATIONS_CONTRACT_FEATURE_IDS: &[&str] = &[
    "A9", "D7", "D8", "D9", "D10", "D11", "MR9", "RT5", "S7", "Sec7", "Sec8", "Sec9", "Sec13",
    "T6", "T7",
];

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OperationsReadinessContract {
    pub checks: Vec<OperationsCheck>,
}

impl OperationsReadinessContract {
    pub fn validate(&self) -> Result<(), OperationsContractError> {
        if self.checks.is_empty() {
            return Err(OperationsContractError::MissingRequiredField("checks"));
        }

        let mut seen = BTreeSet::new();
        for check in &self.checks {
            check.validate()?;
            seen.insert(check.feature_id);
        }

        for feature_id in OPERATIONS_CONTRACT_FEATURE_IDS {
            if !seen.contains(feature_id) {
                return Err(OperationsContractError::MissingFeature(feature_id));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OperationsCheck {
    pub feature_id: &'static str,
    pub artifact: String,
    pub gate: OperationsGate,
}

impl OperationsCheck {
    fn validate(&self) -> Result<(), OperationsContractError> {
        validate_required("check.feature_id", self.feature_id)?;
        validate_required("check.artifact", &self.artifact)?;
        self.gate.validate()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OperationsGate {
    HelmRender { values_file: String },
    ScriptContract { path: String },
    Runbook { path: String, drill: String },
    RuntimeToggle { key: String, expected_value: String },
    SecurityPolicy { control: String },
    Compatibility { client: String, protocol: String },
}

impl OperationsGate {
    fn validate(&self) -> Result<(), OperationsContractError> {
        match self {
            Self::HelmRender { values_file } => validate_required("helm.values_file", values_file),
            Self::ScriptContract { path } => validate_required("script.path", path),
            Self::Runbook { path, drill } => {
                validate_required("runbook.path", path)?;
                validate_required("runbook.drill", drill)
            }
            Self::RuntimeToggle {
                key,
                expected_value,
            } => {
                validate_required("runtime.key", key)?;
                validate_required("runtime.expected_value", expected_value)
            }
            Self::SecurityPolicy { control } => validate_required("security.control", control),
            Self::Compatibility { client, protocol } => {
                validate_required("compatibility.client", client)?;
                validate_required("compatibility.protocol", protocol)
            }
        }
    }
}

pub fn canonical_operations_readiness_contract() -> OperationsReadinessContract {
    OperationsReadinessContract {
        checks: vec![
            helm("D7", "deploy/k8s/helm/citus-overlay/values.yaml"),
            script("D8", "scripts/citus-scale/deploy.sh"),
            runbook(
                "D9",
                "docs/ai-blaise/RUNBOOKS/upgrade.md",
                "canary upgrade rehearsal",
            ),
            runbook(
                "D10",
                "docs/ai-blaise/RUNBOOKS/production.md",
                "production readiness review",
            ),
            script("D11", "tools/citus-mcp/src/lib.rs"),
            runbook(
                "MR9",
                "docs/ai-blaise/RUNBOOKS/disaster-recovery.md",
                "regional failover drill",
            ),
            compatibility("RT5", "supabase-js", "Phoenix channels"),
            security("S7", "pgactive conflict-policy gate"),
            security("A9", "vector provider keys use external secret references"),
            security("Sec7", "external secret references only"),
            security("Sec8", "TLS to clients, Postgres, and sidecars"),
            security("Sec9", "SBOM and cosign release attestation"),
            security("Sec13", "pool CIDR access control"),
            runtime("T6", "postgres.io_method", "io_uring"),
            runtime("T7", "pool.protocol_pipeline", "enabled"),
        ],
    }
}

fn helm(feature_id: &'static str, values_file: &str) -> OperationsCheck {
    OperationsCheck {
        feature_id,
        artifact: "deploy/k8s/helm/citus-overlay".to_string(),
        gate: OperationsGate::HelmRender {
            values_file: values_file.to_string(),
        },
    }
}

fn script(feature_id: &'static str, path: &str) -> OperationsCheck {
    OperationsCheck {
        feature_id,
        artifact: path.to_string(),
        gate: OperationsGate::ScriptContract {
            path: path.to_string(),
        },
    }
}

fn runbook(feature_id: &'static str, path: &str, drill: &str) -> OperationsCheck {
    OperationsCheck {
        feature_id,
        artifact: path.to_string(),
        gate: OperationsGate::Runbook {
            path: path.to_string(),
            drill: drill.to_string(),
        },
    }
}

fn runtime(feature_id: &'static str, key: &str, expected_value: &str) -> OperationsCheck {
    OperationsCheck {
        feature_id,
        artifact: "deploy/k8s/helm/citus-overlay/values.yaml".to_string(),
        gate: OperationsGate::RuntimeToggle {
            key: key.to_string(),
            expected_value: expected_value.to_string(),
        },
    }
}

fn security(feature_id: &'static str, control: &str) -> OperationsCheck {
    OperationsCheck {
        feature_id,
        artifact: "docs/ai-blaise/RUNBOOKS/production.md".to_string(),
        gate: OperationsGate::SecurityPolicy {
            control: control.to_string(),
        },
    }
}

fn compatibility(feature_id: &'static str, client: &str, protocol: &str) -> OperationsCheck {
    OperationsCheck {
        feature_id,
        artifact: "docs/ai-blaise/RUNBOOKS/production.md".to_string(),
        gate: OperationsGate::Compatibility {
            client: client.to_string(),
            protocol: protocol.to_string(),
        },
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OperationsContractError {
    MissingFeature(&'static str),
    MissingRequiredField(&'static str),
}

impl fmt::Display for OperationsContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFeature(feature_id) => {
                write!(
                    formatter,
                    "operations contract missing feature {feature_id}"
                )
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for OperationsContractError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), OperationsContractError> {
    if value.trim().is_empty() {
        return Err(OperationsContractError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operations_contract_covers_install_security_and_runbooks() {
        let contract = canonical_operations_readiness_contract();

        assert_eq!(contract.validate(), Ok(()));
        assert_eq!(contract.checks.len(), OPERATIONS_CONTRACT_FEATURE_IDS.len());
    }

    #[test]
    fn operations_contract_rejects_empty_runbook_drill() {
        let contract = OperationsReadinessContract {
            checks: vec![runbook("D9", "docs/ai-blaise/RUNBOOKS/upgrade.md", " ")],
        };

        assert_eq!(
            contract.validate(),
            Err(OperationsContractError::MissingRequiredField(
                "runbook.drill"
            ))
        );
    }
}
