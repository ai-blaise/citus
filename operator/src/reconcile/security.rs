// FEATURE: Auth1
// FEATURE: Auth3
// FEATURE: Sec7
// FEATURE: Sec8
// FEATURE: Sec12

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

const OPERATOR_SERVICE_ACCOUNT: &str = "ai-blaise-citus-operator";
const RUNTIME_UID: u32 = 10_001;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WorkloadSecurityPlan {
    pub workload: WorkloadKind,
    pub service_account: String,
    pub pod_security: PodSecurityContextPlan,
    pub container_security: ContainerSecurityContextPlan,
    pub rbac: RbacPolicyPlan,
    pub secrets: SecretAccessPlan,
    pub tls: TlsPolicyPlan,
    pub auth: Option<AuthBoundaryPlan>,
}

impl WorkloadSecurityPlan {
    pub fn operator() -> Self {
        Self {
            workload: WorkloadKind::Operator,
            service_account: OPERATOR_SERVICE_ACCOUNT.to_string(),
            pod_security: PodSecurityContextPlan::restricted(),
            container_security: ContainerSecurityContextPlan::restricted(),
            rbac: RbacPolicyPlan::operator_controller(),
            secrets: SecretAccessPlan::default(),
            tls: TlsPolicyPlan::disabled(),
            auth: None,
        }
    }

    pub fn pool(cluster_name: &str) -> Self {
        let workload_name = format!("{cluster_name}-pool");
        Self {
            workload: WorkloadKind::Pool,
            service_account: workload_name.clone(),
            pod_security: PodSecurityContextPlan::restricted(),
            container_security: ContainerSecurityContextPlan::restricted(),
            rbac: RbacPolicyPlan::no_kubernetes_api(),
            secrets: SecretAccessPlan::referenced(vec![SecretReferencePlan {
                name: format!("{workload_name}-postgres-auth"),
                key: "password".to_string(),
                mount: SecretMountMode::Env,
            }]),
            tls: TlsPolicyPlan::required(format!("{workload_name}-tls"), true),
            auth: Some(AuthBoundaryPlan::postgres_pool()),
        }
    }

    pub fn sidecar(cluster_name: &str, sidecar_suffix: &str) -> Self {
        let workload_name = format!("{cluster_name}-sidecar-{sidecar_suffix}");
        Self {
            workload: WorkloadKind::Sidecar {
                sidecar_type: sidecar_suffix.to_string(),
            },
            service_account: workload_name.clone(),
            pod_security: PodSecurityContextPlan::restricted(),
            container_security: ContainerSecurityContextPlan::restricted(),
            rbac: RbacPolicyPlan::no_kubernetes_api(),
            secrets: SecretAccessPlan::referenced(vec![SecretReferencePlan {
                name: format!("{workload_name}-runtime"),
                key: "config".to_string(),
                mount: SecretMountMode::ReadOnlyVolume,
            }]),
            tls: TlsPolicyPlan::required(format!("{workload_name}-tls"), true),
            auth: Some(AuthBoundaryPlan::sidecar_http()),
        }
    }

    pub fn validate(&self) -> Result<(), WorkloadSecurityError> {
        self.workload.validate()?;
        validate_kubernetes_name("service_account", &self.service_account)?;
        self.pod_security.validate()?;
        self.container_security.validate()?;
        self.rbac.validate()?;
        self.secrets.validate()?;
        self.tls.validate()?;
        if let Some(auth) = &self.auth {
            auth.validate()?;
        }
        Ok(())
    }

    pub fn denies_kubernetes_api(&self) -> bool {
        matches!(self.rbac.access, KubernetesApiAccess::None)
    }

    pub fn requires_tls(&self) -> bool {
        matches!(self.tls.mode, TlsMode::Required)
    }

    pub fn secret_reference_count(&self) -> usize {
        self.secrets.references.len() + self.tls.secret_reference_count()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WorkloadKind {
    Operator,
    Pool,
    Sidecar { sidecar_type: String },
}

impl WorkloadKind {
    fn validate(&self) -> Result<(), WorkloadSecurityError> {
        match self {
            Self::Operator | Self::Pool => Ok(()),
            Self::Sidecar { sidecar_type } => {
                validate_kubernetes_label("sidecar_type", sidecar_type)
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PodSecurityContextPlan {
    pub run_as_non_root: bool,
    pub run_as_user: u32,
    pub seccomp_profile: SeccompProfile,
}

impl PodSecurityContextPlan {
    fn restricted() -> Self {
        Self {
            run_as_non_root: true,
            run_as_user: RUNTIME_UID,
            seccomp_profile: SeccompProfile::RuntimeDefault,
        }
    }

    fn validate(&self) -> Result<(), WorkloadSecurityError> {
        if !self.run_as_non_root || self.run_as_user == 0 {
            return Err(WorkloadSecurityError::RootContainer);
        }
        if self.seccomp_profile != SeccompProfile::RuntimeDefault {
            return Err(WorkloadSecurityError::NonDefaultSeccomp);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SeccompProfile {
    RuntimeDefault,
    Unconfined,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ContainerSecurityContextPlan {
    pub allow_privilege_escalation: bool,
    pub read_only_root_filesystem: bool,
    pub drop_all_capabilities: bool,
}

impl ContainerSecurityContextPlan {
    fn restricted() -> Self {
        Self {
            allow_privilege_escalation: false,
            read_only_root_filesystem: true,
            drop_all_capabilities: true,
        }
    }

    fn validate(&self) -> Result<(), WorkloadSecurityError> {
        if self.allow_privilege_escalation {
            return Err(WorkloadSecurityError::PrivilegeEscalationAllowed);
        }
        if !self.read_only_root_filesystem {
            return Err(WorkloadSecurityError::WritableRootFilesystem);
        }
        if !self.drop_all_capabilities {
            return Err(WorkloadSecurityError::CapabilitiesNotDropped);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RbacPolicyPlan {
    pub access: KubernetesApiAccess,
    pub rules: Vec<RbacRulePlan>,
}

impl RbacPolicyPlan {
    fn no_kubernetes_api() -> Self {
        Self {
            access: KubernetesApiAccess::None,
            rules: Vec::new(),
        }
    }

    fn operator_controller() -> Self {
        Self {
            access: KubernetesApiAccess::Scoped,
            rules: vec![
                RbacRulePlan {
                    api_group: "ai-blaise.com".to_string(),
                    resources: vec![
                        "citusclusters".to_string(),
                        "hypertables".to_string(),
                        "migrations".to_string(),
                        "shardgroups".to_string(),
                        "tenants".to_string(),
                    ],
                    verbs: vec![
                        "get".to_string(),
                        "list".to_string(),
                        "watch".to_string(),
                        "patch".to_string(),
                        "update".to_string(),
                    ],
                },
                RbacRulePlan {
                    api_group: "".to_string(),
                    resources: vec!["events".to_string()],
                    verbs: vec!["create".to_string(), "patch".to_string()],
                },
            ],
        }
    }

    fn validate(&self) -> Result<(), WorkloadSecurityError> {
        match self.access {
            KubernetesApiAccess::None if !self.rules.is_empty() => {
                return Err(WorkloadSecurityError::UnexpectedRbacRule);
            }
            KubernetesApiAccess::None => return Ok(()),
            KubernetesApiAccess::Scoped if self.rules.is_empty() => {
                return Err(WorkloadSecurityError::MissingRbacRule);
            }
            KubernetesApiAccess::Scoped => {}
        }

        for rule in &self.rules {
            rule.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum KubernetesApiAccess {
    None,
    Scoped,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RbacRulePlan {
    pub api_group: String,
    pub resources: Vec<String>,
    pub verbs: Vec<String>,
}

impl RbacRulePlan {
    fn validate(&self) -> Result<(), WorkloadSecurityError> {
        validate_optional_api_group(&self.api_group)?;
        validate_required_list("rbac.resources", &self.resources)?;
        validate_required_list("rbac.verbs", &self.verbs)?;
        if self.resources.iter().any(|resource| resource == "*")
            || self.verbs.iter().any(|verb| verb == "*")
        {
            return Err(WorkloadSecurityError::WildcardRbacRule);
        }
        if self
            .resources
            .iter()
            .any(|resource| resource.eq_ignore_ascii_case("secrets"))
        {
            return Err(WorkloadSecurityError::SecretRbacForbidden);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct SecretAccessPlan {
    pub references: Vec<SecretReferencePlan>,
    pub inline_values: Vec<String>,
}

impl SecretAccessPlan {
    fn referenced(references: Vec<SecretReferencePlan>) -> Self {
        Self {
            references,
            inline_values: Vec::new(),
        }
    }

    fn validate(&self) -> Result<(), WorkloadSecurityError> {
        if !self.inline_values.is_empty() {
            return Err(WorkloadSecurityError::InlineSecretValue);
        }

        let mut seen = BTreeSet::new();
        for reference in &self.references {
            reference.validate()?;
            let key = format!("{}:{}", reference.name, reference.key);
            if !seen.insert(key) {
                return Err(WorkloadSecurityError::DuplicateSecretReference);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SecretReferencePlan {
    pub name: String,
    pub key: String,
    pub mount: SecretMountMode,
}

impl SecretReferencePlan {
    fn validate(&self) -> Result<(), WorkloadSecurityError> {
        validate_kubernetes_name("secret.name", &self.name)?;
        validate_secret_key("secret.key", &self.key)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SecretMountMode {
    Env,
    ReadOnlyVolume,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TlsPolicyPlan {
    pub mode: TlsMode,
    pub min_version: TlsVersion,
    pub cert_secret_ref: Option<SecretReferencePlan>,
    pub key_secret_ref: Option<SecretReferencePlan>,
    pub ca_secret_ref: Option<SecretReferencePlan>,
    pub require_client_cert: bool,
}

impl TlsPolicyPlan {
    fn disabled() -> Self {
        Self {
            mode: TlsMode::Disabled,
            min_version: TlsVersion::Tls13,
            cert_secret_ref: None,
            key_secret_ref: None,
            ca_secret_ref: None,
            require_client_cert: false,
        }
    }

    fn required(secret_name: String, require_client_cert: bool) -> Self {
        Self {
            mode: TlsMode::Required,
            min_version: TlsVersion::Tls13,
            cert_secret_ref: Some(SecretReferencePlan {
                name: secret_name.clone(),
                key: "tls.crt".to_string(),
                mount: SecretMountMode::ReadOnlyVolume,
            }),
            key_secret_ref: Some(SecretReferencePlan {
                name: secret_name.clone(),
                key: "tls.key".to_string(),
                mount: SecretMountMode::ReadOnlyVolume,
            }),
            ca_secret_ref: Some(SecretReferencePlan {
                name: secret_name,
                key: "ca.crt".to_string(),
                mount: SecretMountMode::ReadOnlyVolume,
            }),
            require_client_cert,
        }
    }

    fn validate(&self) -> Result<(), WorkloadSecurityError> {
        match self.mode {
            TlsMode::Disabled => {
                if self.cert_secret_ref.is_some()
                    || self.key_secret_ref.is_some()
                    || self.ca_secret_ref.is_some()
                    || self.require_client_cert
                {
                    return Err(WorkloadSecurityError::UnexpectedTlsSecret);
                }
            }
            TlsMode::Required => {
                if self.min_version != TlsVersion::Tls13 {
                    return Err(WorkloadSecurityError::WeakTlsVersion);
                }
                self.cert_secret_ref
                    .as_ref()
                    .ok_or(WorkloadSecurityError::MissingTlsSecret("tls.cert"))?
                    .validate()?;
                self.key_secret_ref
                    .as_ref()
                    .ok_or(WorkloadSecurityError::MissingTlsSecret("tls.key"))?
                    .validate()?;
                self.ca_secret_ref
                    .as_ref()
                    .ok_or(WorkloadSecurityError::MissingTlsSecret("tls.ca"))?
                    .validate()?;
                if !self.require_client_cert {
                    return Err(WorkloadSecurityError::ClientCertificateNotRequired);
                }
            }
        }
        Ok(())
    }

    fn secret_reference_count(&self) -> usize {
        usize::from(self.cert_secret_ref.is_some())
            + usize::from(self.key_secret_ref.is_some())
            + usize::from(self.ca_secret_ref.is_some())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TlsMode {
    Disabled,
    Required,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AuthBoundaryPlan {
    pub issuer: String,
    pub audience: String,
    pub tenant_claim: String,
    pub fail_closed: bool,
}

impl AuthBoundaryPlan {
    fn postgres_pool() -> Self {
        Self {
            issuer: "https://auth.example.com".to_string(),
            audience: "postgres".to_string(),
            tenant_claim: "tenant_id".to_string(),
            fail_closed: true,
        }
    }

    fn sidecar_http() -> Self {
        Self {
            issuer: "https://auth.example.com".to_string(),
            audience: "ai-blaise-sidecars".to_string(),
            tenant_claim: "tenant_id".to_string(),
            fail_closed: true,
        }
    }

    fn validate(&self) -> Result<(), WorkloadSecurityError> {
        validate_required("auth.issuer", &self.issuer)?;
        if !self.issuer.starts_with("https://") {
            return Err(WorkloadSecurityError::InsecureAuthIssuer);
        }
        validate_required("auth.audience", &self.audience)?;
        validate_required("auth.tenant_claim", &self.tenant_claim)?;
        if !self.fail_closed {
            return Err(WorkloadSecurityError::AuthDoesNotFailClosed);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WorkloadSecurityReport {
    pub workloads: usize,
    pub tls_required: usize,
    pub auth_boundaries: usize,
    pub secret_refs: usize,
    pub rbac_rules: usize,
    pub kube_api_denied: usize,
    pub run_as_non_root: usize,
    pub read_only_rootfs: usize,
    pub drop_all_capabilities: usize,
}

impl WorkloadSecurityReport {
    pub fn from_plans(plans: &[WorkloadSecurityPlan]) -> Result<Self, WorkloadSecurityError> {
        if plans.is_empty() {
            return Err(WorkloadSecurityError::MissingRequiredField("workloads"));
        }

        let mut report = Self {
            workloads: plans.len(),
            tls_required: 0,
            auth_boundaries: 0,
            secret_refs: 0,
            rbac_rules: 0,
            kube_api_denied: 0,
            run_as_non_root: 0,
            read_only_rootfs: 0,
            drop_all_capabilities: 0,
        };

        for plan in plans {
            plan.validate()?;
            if plan.requires_tls() {
                report.tls_required += 1;
            }
            if plan.auth.is_some() {
                report.auth_boundaries += 1;
            }
            report.secret_refs += plan.secret_reference_count();
            report.rbac_rules += plan.rbac.rules.len();
            if plan.denies_kubernetes_api() {
                report.kube_api_denied += 1;
            }
            if plan.pod_security.run_as_non_root {
                report.run_as_non_root += 1;
            }
            if plan.container_security.read_only_root_filesystem {
                report.read_only_rootfs += 1;
            }
            if plan.container_security.drop_all_capabilities {
                report.drop_all_capabilities += 1;
            }
        }

        Ok(report)
    }
}

pub fn canonical_operator_security_plans() -> Vec<WorkloadSecurityPlan> {
    vec![
        WorkloadSecurityPlan::operator(),
        WorkloadSecurityPlan::pool("ai-blaise-citus"),
        WorkloadSecurityPlan::sidecar("ai-blaise-citus", "vectorizer"),
        WorkloadSecurityPlan::sidecar("ai-blaise-citus", "realtime"),
        WorkloadSecurityPlan::sidecar("ai-blaise-citus", "mcp"),
        WorkloadSecurityPlan::sidecar("ai-blaise-citus", "custom-audit-exporter"),
    ]
}

pub fn canonical_operator_security_report() -> Result<WorkloadSecurityReport, WorkloadSecurityError>
{
    WorkloadSecurityReport::from_plans(&canonical_operator_security_plans())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WorkloadSecurityError {
    AuthDoesNotFailClosed,
    CapabilitiesNotDropped,
    ClientCertificateNotRequired,
    DuplicateSecretReference,
    InlineSecretValue,
    InsecureAuthIssuer,
    InvalidKubernetesName(&'static str),
    InvalidSecretKey(&'static str),
    MissingRequiredField(&'static str),
    MissingRbacRule,
    MissingTlsSecret(&'static str),
    NonDefaultSeccomp,
    PrivilegeEscalationAllowed,
    RootContainer,
    SecretRbacForbidden,
    UnexpectedRbacRule,
    UnexpectedTlsSecret,
    WeakTlsVersion,
    WildcardRbacRule,
    WritableRootFilesystem,
}

impl fmt::Display for WorkloadSecurityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthDoesNotFailClosed => write!(formatter, "auth boundary must fail closed"),
            Self::CapabilitiesNotDropped => {
                write!(formatter, "containers must drop ALL capabilities")
            }
            Self::ClientCertificateNotRequired => {
                write!(formatter, "TLS must require client certificates")
            }
            Self::DuplicateSecretReference => write!(formatter, "secret references must be unique"),
            Self::InlineSecretValue => write!(
                formatter,
                "secret values must be references, not inline values"
            ),
            Self::InsecureAuthIssuer => write!(formatter, "auth issuer must use https"),
            Self::InvalidKubernetesName(field) => {
                write!(formatter, "{field} must be a valid Kubernetes name")
            }
            Self::InvalidSecretKey(field) => {
                write!(formatter, "{field} must be a valid Secret key")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::MissingRbacRule => write!(
                formatter,
                "scoped Kubernetes API access requires explicit RBAC rules"
            ),
            Self::MissingTlsSecret(field) => {
                write!(formatter, "{field} secret reference is required")
            }
            Self::NonDefaultSeccomp => write!(formatter, "seccompProfile must be RuntimeDefault"),
            Self::PrivilegeEscalationAllowed => {
                write!(formatter, "allowPrivilegeEscalation must be false")
            }
            Self::RootContainer => write!(formatter, "containers must run as a non-root UID"),
            Self::SecretRbacForbidden => {
                write!(formatter, "workload RBAC must not grant Secret API access")
            }
            Self::UnexpectedRbacRule => write!(
                formatter,
                "workloads without Kubernetes API access must not carry RBAC rules"
            ),
            Self::UnexpectedTlsSecret => write!(
                formatter,
                "disabled TLS must not carry certificate references"
            ),
            Self::WeakTlsVersion => write!(formatter, "TLS must require TLS 1.3"),
            Self::WildcardRbacRule => write!(
                formatter,
                "RBAC rules must not use wildcard verbs or resources"
            ),
            Self::WritableRootFilesystem => {
                write!(formatter, "readOnlyRootFilesystem must be true")
            }
        }
    }
}

impl Error for WorkloadSecurityError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), WorkloadSecurityError> {
    if value.trim().is_empty() {
        return Err(WorkloadSecurityError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(
    field: &'static str,
    values: &[String],
) -> Result<(), WorkloadSecurityError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(WorkloadSecurityError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional_api_group(value: &str) -> Result<(), WorkloadSecurityError> {
    if value == "*" {
        return Err(WorkloadSecurityError::WildcardRbacRule);
    }
    Ok(())
}

fn validate_kubernetes_name(field: &'static str, value: &str) -> Result<(), WorkloadSecurityError> {
    let value = value.trim();
    validate_required(field, value)?;
    if value.len() > 253 {
        return Err(WorkloadSecurityError::InvalidKubernetesName(field));
    }
    for label in value.split('.') {
        validate_kubernetes_label(field, label)?;
    }
    Ok(())
}

fn validate_kubernetes_label(
    field: &'static str,
    value: &str,
) -> Result<(), WorkloadSecurityError> {
    let value = value.trim();
    validate_required(field, value)?;
    if value.len() > 63 || value.starts_with('-') || value.ends_with('-') {
        return Err(WorkloadSecurityError::InvalidKubernetesName(field));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(WorkloadSecurityError::InvalidKubernetesName(field));
    }
    Ok(())
}

fn validate_secret_key(field: &'static str, value: &str) -> Result<(), WorkloadSecurityError> {
    let value = value.trim();
    validate_required(field, value)?;
    if value == "." || value == ".." || value.contains('/') {
        return Err(WorkloadSecurityError::InvalidSecretKey(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_operator_security_report_is_deterministic() {
        let report = canonical_operator_security_report().expect("security report");

        assert_eq!(
            report,
            WorkloadSecurityReport {
                workloads: 6,
                tls_required: 5,
                auth_boundaries: 5,
                secret_refs: 20,
                rbac_rules: 2,
                kube_api_denied: 5,
                run_as_non_root: 6,
                read_only_rootfs: 6,
                drop_all_capabilities: 6,
            }
        );
    }

    #[test]
    fn pool_security_requires_mtls_and_secret_refs() {
        let plan = WorkloadSecurityPlan::pool("ai-blaise-citus");

        assert_eq!(plan.validate(), Ok(()));
        assert!(plan.requires_tls());
        assert_eq!(plan.secret_reference_count(), 4);
    }

    #[test]
    fn custom_sidecars_inherit_restricted_runtime_boundary() {
        let plan = WorkloadSecurityPlan::sidecar("ai-blaise-citus", "custom-audit-exporter");

        assert_eq!(plan.validate(), Ok(()));
        assert!(plan.denies_kubernetes_api());
        assert!(plan.container_security.read_only_root_filesystem);
    }

    #[test]
    fn rbac_rejects_secret_and_wildcard_grants() {
        let secret_grant = RbacPolicyPlan {
            access: KubernetesApiAccess::Scoped,
            rules: vec![RbacRulePlan {
                api_group: "".to_string(),
                resources: vec!["secrets".to_string()],
                verbs: vec!["get".to_string()],
            }],
        };
        let wildcard_grant = RbacPolicyPlan {
            access: KubernetesApiAccess::Scoped,
            rules: vec![RbacRulePlan {
                api_group: "ai-blaise.com".to_string(),
                resources: vec!["*".to_string()],
                verbs: vec!["get".to_string()],
            }],
        };

        assert_eq!(
            secret_grant.validate(),
            Err(WorkloadSecurityError::SecretRbacForbidden)
        );
        assert_eq!(
            wildcard_grant.validate(),
            Err(WorkloadSecurityError::WildcardRbacRule)
        );
    }

    #[test]
    fn security_context_rejects_privileged_or_writable_containers() {
        let mut plan = WorkloadSecurityPlan::sidecar("ai-blaise-citus", "mcp");
        plan.container_security.allow_privilege_escalation = true;
        assert_eq!(
            plan.validate(),
            Err(WorkloadSecurityError::PrivilegeEscalationAllowed)
        );

        plan.container_security.allow_privilege_escalation = false;
        plan.container_security.read_only_root_filesystem = false;
        assert_eq!(
            plan.validate(),
            Err(WorkloadSecurityError::WritableRootFilesystem)
        );
    }

    #[test]
    fn tls_and_auth_policies_fail_closed() {
        let mut plan = WorkloadSecurityPlan::pool("ai-blaise-citus");
        plan.tls.require_client_cert = false;
        assert_eq!(
            plan.validate(),
            Err(WorkloadSecurityError::ClientCertificateNotRequired)
        );

        let mut plan = WorkloadSecurityPlan::pool("ai-blaise-citus");
        plan.auth.as_mut().expect("auth").fail_closed = false;
        assert_eq!(
            plan.validate(),
            Err(WorkloadSecurityError::AuthDoesNotFailClosed)
        );
    }

    #[test]
    fn secrets_must_be_references_not_inline_values() {
        let mut plan = WorkloadSecurityPlan::pool("ai-blaise-citus");
        plan.secrets
            .inline_values
            .push("password=secret".to_string());

        assert_eq!(
            plan.validate(),
            Err(WorkloadSecurityError::InlineSecretValue)
        );
    }
}
