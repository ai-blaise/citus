// FEATURE: Auth1
// FEATURE: Auth3
// FEATURE: A9
// FEATURE: Sec7
// FEATURE: Sec8
// FEATURE: Sec9
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
    pub external_secrets: Vec<ExternalSecretBindingPlan>,
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
            external_secrets: Vec::new(),
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
            external_secrets: vec![ExternalSecretBindingPlan::runtime_secret(
                format!("{workload_name}-postgres-auth"),
                "password",
                "postgres/pool/password",
                "password",
            )],
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
            external_secrets: vec![ExternalSecretBindingPlan::runtime_secret(
                format!("{workload_name}-runtime"),
                "config",
                format!("sidecars/{sidecar_suffix}/runtime-config"),
                "config",
            )],
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
        for external_secret in &self.external_secrets {
            external_secret.validate()?;
        }
        self.secrets.validate(&self.external_secrets)?;
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
                    api_group: "citus.ai-blaise.io".to_string(),
                    resources: vec![
                        "citusclusters".to_string(),
                        "citusclusters/status".to_string(),
                        "citusclusters/finalizers".to_string(),
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
                    api_group: "postgresql.cnpg.io".to_string(),
                    resources: vec!["clusters".to_string(), "imagecatalogs".to_string()],
                    verbs: vec![
                        "get".to_string(),
                        "list".to_string(),
                        "create".to_string(),
                        "patch".to_string(),
                    ],
                },
                RbacRulePlan {
                    api_group: "".to_string(),
                    resources: vec!["configmaps".to_string()],
                    verbs: vec![
                        "get".to_string(),
                        "list".to_string(),
                        "create".to_string(),
                        "patch".to_string(),
                        "delete".to_string(),
                    ],
                },
                RbacRulePlan {
                    api_group: "".to_string(),
                    resources: vec!["secrets".to_string()],
                    verbs: vec!["get".to_string()],
                },
                RbacRulePlan {
                    api_group: "".to_string(),
                    resources: vec!["pods".to_string()],
                    verbs: vec!["get".to_string(), "list".to_string()],
                },
                RbacRulePlan {
                    api_group: "batch".to_string(),
                    resources: vec!["jobs".to_string()],
                    verbs: vec![
                        "get".to_string(),
                        "list".to_string(),
                        "create".to_string(),
                        "patch".to_string(),
                        "delete".to_string(),
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
            && (!self.api_group.is_empty() || self.verbs.iter().any(|verb| verb != "get"))
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

    fn validate(
        &self,
        external_secrets: &[ExternalSecretBindingPlan],
    ) -> Result<(), WorkloadSecurityError> {
        if !self.inline_values.is_empty() {
            return Err(WorkloadSecurityError::InlineSecretValue);
        }

        let mut seen = BTreeSet::new();
        let external_targets = external_secrets
            .iter()
            .map(|binding| format!("{}:{}", binding.target_secret_name, binding.target_key))
            .collect::<BTreeSet<_>>();

        for reference in &self.references {
            reference.validate()?;
            let key = format!("{}:{}", reference.name, reference.key);
            if !seen.insert(key.clone()) {
                return Err(WorkloadSecurityError::DuplicateSecretReference);
            }
            if !external_targets.contains(&key) {
                return Err(WorkloadSecurityError::MissingExternalSecretBinding);
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
pub struct ExternalSecretBindingPlan {
    pub secret_store_ref: String,
    pub target_secret_name: String,
    pub target_key: String,
    pub remote_key: String,
    pub remote_property: String,
    pub refresh_interval_minutes: u32,
}

impl ExternalSecretBindingPlan {
    fn runtime_secret(
        target_secret_name: String,
        target_key: &str,
        remote_key: impl Into<String>,
        remote_property: &str,
    ) -> Self {
        Self {
            secret_store_ref: "ai-blaise-cluster-secrets".to_string(),
            target_secret_name,
            target_key: target_key.to_string(),
            remote_key: remote_key.into(),
            remote_property: remote_property.to_string(),
            refresh_interval_minutes: 5,
        }
    }

    fn validate(&self) -> Result<(), WorkloadSecurityError> {
        validate_kubernetes_name("external_secret.secret_store_ref", &self.secret_store_ref)?;
        validate_kubernetes_name(
            "external_secret.target_secret_name",
            &self.target_secret_name,
        )?;
        validate_secret_key("external_secret.target_key", &self.target_key)?;
        validate_remote_secret_key("external_secret.remote_key", &self.remote_key)?;
        validate_secret_key("external_secret.remote_property", &self.remote_property)?;
        if self.refresh_interval_minutes == 0 {
            return Err(WorkloadSecurityError::InvalidExternalSecretRefreshInterval);
        }
        Ok(())
    }
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
    pub external_secret_bindings: usize,
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
            external_secret_bindings: 0,
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
            report.external_secret_bindings += plan.external_secrets.len();
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalSecretManifestPlan {
    pub api_version: String,
    pub kind: String,
    pub metadata_name: String,
    pub secret_store_ref: String,
    pub target_secret_name: String,
    pub target_key: String,
    pub remote_key: String,
    pub remote_property: String,
    pub refresh_interval_minutes: u32,
}

impl ExternalSecretManifestPlan {
    fn from_binding(binding: &ExternalSecretBindingPlan) -> Self {
        Self {
            api_version: "external-secrets.io/v1beta1".to_string(),
            kind: "ExternalSecret".to_string(),
            metadata_name: binding.target_secret_name.clone(),
            secret_store_ref: binding.secret_store_ref.clone(),
            target_secret_name: binding.target_secret_name.clone(),
            target_key: binding.target_key.clone(),
            remote_key: binding.remote_key.clone(),
            remote_property: binding.remote_property.clone(),
            refresh_interval_minutes: binding.refresh_interval_minutes,
        }
    }

    fn validate(&self, binding: &ExternalSecretBindingPlan) -> Result<(), WorkloadSecurityError> {
        if self.api_version != "external-secrets.io/v1beta1" || self.kind != "ExternalSecret" {
            return Err(WorkloadSecurityError::InvalidExternalSecretManifest);
        }
        validate_kubernetes_name("external_secret.metadata_name", &self.metadata_name)?;
        validate_kubernetes_name("external_secret.secret_store_ref", &self.secret_store_ref)?;
        validate_kubernetes_name(
            "external_secret.target_secret_name",
            &self.target_secret_name,
        )?;
        validate_secret_key("external_secret.target_key", &self.target_key)?;
        validate_remote_secret_key("external_secret.remote_key", &self.remote_key)?;
        validate_secret_key("external_secret.remote_property", &self.remote_property)?;
        if self.refresh_interval_minutes == 0 {
            return Err(WorkloadSecurityError::InvalidExternalSecretRefreshInterval);
        }
        if self.metadata_name != binding.target_secret_name
            || self.secret_store_ref != binding.secret_store_ref
            || self.target_secret_name != binding.target_secret_name
            || self.target_key != binding.target_key
            || self.remote_key != binding.remote_key
            || self.remote_property != binding.remote_property
            || self.refresh_interval_minutes != binding.refresh_interval_minutes
        {
            return Err(WorkloadSecurityError::ExternalSecretBindingMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TlsSecretManifestPlan {
    pub secret_name: String,
    pub cert_key: String,
    pub private_key: String,
    pub ca_key: String,
    pub min_version: TlsVersion,
    pub require_client_cert: bool,
}

impl TlsSecretManifestPlan {
    fn from_tls_policy(tls: &TlsPolicyPlan) -> Result<Option<Self>, WorkloadSecurityError> {
        if tls.mode == TlsMode::Disabled {
            return Ok(None);
        }

        let cert = tls
            .cert_secret_ref
            .as_ref()
            .ok_or(WorkloadSecurityError::MissingTlsSecret("tls.cert"))?;
        let key = tls
            .key_secret_ref
            .as_ref()
            .ok_or(WorkloadSecurityError::MissingTlsSecret("tls.key"))?;
        let ca = tls
            .ca_secret_ref
            .as_ref()
            .ok_or(WorkloadSecurityError::MissingTlsSecret("tls.ca"))?;

        Ok(Some(Self {
            secret_name: cert.name.clone(),
            cert_key: cert.key.clone(),
            private_key: key.key.clone(),
            ca_key: ca.key.clone(),
            min_version: tls.min_version,
            require_client_cert: tls.require_client_cert,
        }))
    }

    fn validate(&self, tls: &TlsPolicyPlan) -> Result<(), WorkloadSecurityError> {
        validate_kubernetes_name("tls.secret_name", &self.secret_name)?;
        if self.cert_key != "tls.crt" || self.private_key != "tls.key" || self.ca_key != "ca.crt" {
            return Err(WorkloadSecurityError::InvalidTlsSecretManifest);
        }
        if self.min_version != TlsVersion::Tls13 {
            return Err(WorkloadSecurityError::WeakTlsVersion);
        }
        if !self.require_client_cert {
            return Err(WorkloadSecurityError::ClientCertificateNotRequired);
        }

        let cert = tls
            .cert_secret_ref
            .as_ref()
            .ok_or(WorkloadSecurityError::MissingTlsSecret("tls.cert"))?;
        let key = tls
            .key_secret_ref
            .as_ref()
            .ok_or(WorkloadSecurityError::MissingTlsSecret("tls.key"))?;
        let ca = tls
            .ca_secret_ref
            .as_ref()
            .ok_or(WorkloadSecurityError::MissingTlsSecret("tls.ca"))?;
        if cert.name != self.secret_name
            || key.name != self.secret_name
            || ca.name != self.secret_name
            || cert.key != self.cert_key
            || key.key != self.private_key
            || ca.key != self.ca_key
        {
            return Err(WorkloadSecurityError::TlsSecretBindingMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SupplyChainAttestationPlan {
    pub image_ref: String,
    pub source_revision: String,
    pub sbom_path: String,
    pub cosign_bundle_path: String,
    pub provenance_predicate_type: String,
}

impl SupplyChainAttestationPlan {
    fn validate(&self) -> Result<(), WorkloadSecurityError> {
        validate_required("supply_chain.image_ref", &self.image_ref)?;
        validate_required("supply_chain.source_revision", &self.source_revision)?;
        validate_required("supply_chain.sbom_path", &self.sbom_path)?;
        validate_required("supply_chain.cosign_bundle_path", &self.cosign_bundle_path)?;
        validate_required(
            "supply_chain.provenance_predicate_type",
            &self.provenance_predicate_type,
        )?;
        if !is_sha256_digest_ref(&self.image_ref) {
            return Err(WorkloadSecurityError::MutableImageReference);
        }
        if !is_hex_revision(&self.source_revision) {
            return Err(WorkloadSecurityError::InvalidSourceRevision);
        }
        if !self.sbom_path.ends_with(".spdx.json") {
            return Err(WorkloadSecurityError::InvalidSbomPath);
        }
        if !self.cosign_bundle_path.ends_with(".sigstore.json") {
            return Err(WorkloadSecurityError::InvalidCosignBundlePath);
        }
        if !self
            .provenance_predicate_type
            .contains("slsa.dev/provenance/v1")
        {
            return Err(WorkloadSecurityError::MissingProvenancePredicate);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SecuritySupplyChainReport {
    pub workloads: usize,
    pub external_secret_manifests: usize,
    pub runtime_secret_refs: usize,
    pub tls_manifests: usize,
    pub tls_secret_refs: usize,
    pub supply_chain_artifacts: usize,
    pub sbom_documents: usize,
    pub cosign_attestations: usize,
    pub digest_pinned_images: usize,
    pub fail_closed_checks: usize,
}

pub fn canonical_security_supply_chain_report(
) -> Result<SecuritySupplyChainReport, WorkloadSecurityError> {
    let plans = canonical_operator_security_plans();
    let artifacts = canonical_supply_chain_attestation_plans();
    let mut external_secret_manifests = 0;
    let mut runtime_secret_refs = 0;
    let mut tls_manifests = 0;
    let mut tls_secret_refs = 0;

    for plan in &plans {
        plan.validate()?;
        runtime_secret_refs += plan.secrets.references.len();

        for binding in &plan.external_secrets {
            let manifest = ExternalSecretManifestPlan::from_binding(binding);
            manifest.validate(binding)?;
            external_secret_manifests += 1;
        }

        if let Some(manifest) = TlsSecretManifestPlan::from_tls_policy(&plan.tls)? {
            manifest.validate(&plan.tls)?;
            tls_manifests += 1;
            tls_secret_refs += plan.tls.secret_reference_count();
        }
    }

    for artifact in &artifacts {
        artifact.validate()?;
    }

    Ok(SecuritySupplyChainReport {
        workloads: plans.len(),
        external_secret_manifests,
        runtime_secret_refs,
        tls_manifests,
        tls_secret_refs,
        supply_chain_artifacts: artifacts.len(),
        sbom_documents: artifacts.len(),
        cosign_attestations: artifacts.len(),
        digest_pinned_images: artifacts.len(),
        fail_closed_checks: security_supply_chain_fail_closed_checks()?,
    })
}

const FIXTURE_SUPPLY_CHAIN_SOURCE_REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

fn canonical_supply_chain_attestation_plans() -> Vec<SupplyChainAttestationPlan> {
    let revision = FIXTURE_SUPPLY_CHAIN_SOURCE_REVISION;
    vec![
        supply_chain_artifact(
            "operator",
            "1111111111111111111111111111111111111111111111111111111111111111",
            revision,
        ),
        supply_chain_artifact(
            "pool",
            "2222222222222222222222222222222222222222222222222222222222222222",
            revision,
        ),
        supply_chain_artifact(
            "sidecar-vectorizer",
            "3333333333333333333333333333333333333333333333333333333333333333",
            revision,
        ),
        supply_chain_artifact(
            "sidecar-mcp",
            "4444444444444444444444444444444444444444444444444444444444444444",
            revision,
        ),
    ]
}

fn supply_chain_artifact(
    component: &str,
    digest: &str,
    revision: &str,
) -> SupplyChainAttestationPlan {
    SupplyChainAttestationPlan {
        image_ref: format!("ghcr.io/ai-blaise/citus-{component}@sha256:{digest}"),
        source_revision: revision.to_string(),
        sbom_path: format!("artifacts/sbom/citus-{component}.spdx.json"),
        cosign_bundle_path: format!("artifacts/attestations/citus-{component}.sigstore.json"),
        provenance_predicate_type: "https://slsa.dev/provenance/v1".to_string(),
    }
}

fn security_supply_chain_fail_closed_checks() -> Result<usize, WorkloadSecurityError> {
    let mut passed = 0;

    let mut missing_binding = WorkloadSecurityPlan::sidecar("ai-blaise-citus", "mcp");
    missing_binding.external_secrets.clear();
    passed += expect_security_error(
        "missing-external-secret-binding",
        missing_binding.validate(),
        WorkloadSecurityError::MissingExternalSecretBinding,
    )?;

    let mut weak_tls = WorkloadSecurityPlan::pool("ai-blaise-citus");
    weak_tls.tls.min_version = TlsVersion::Tls12;
    passed += expect_security_error(
        "weak-tls-version",
        weak_tls.validate(),
        WorkloadSecurityError::WeakTlsVersion,
    )?;

    let mut zero_refresh = ExternalSecretBindingPlan::runtime_secret(
        "ai-blaise-citus-pool-postgres-auth".to_string(),
        "password",
        "postgres/pool/password",
        "password",
    );
    zero_refresh.refresh_interval_minutes = 0;
    passed += expect_security_error(
        "zero-external-secret-refresh",
        zero_refresh.validate(),
        WorkloadSecurityError::InvalidExternalSecretRefreshInterval,
    )?;

    let mut mutable_image = supply_chain_artifact(
        "operator",
        "5555555555555555555555555555555555555555555555555555555555555555",
        FIXTURE_SUPPLY_CHAIN_SOURCE_REVISION,
    );
    mutable_image.image_ref = "ghcr.io/ai-blaise/citus-operator:latest".to_string();
    passed += expect_security_error(
        "mutable-image-reference",
        mutable_image.validate(),
        WorkloadSecurityError::MutableImageReference,
    )?;

    let mut missing_sbom = supply_chain_artifact(
        "pool",
        "6666666666666666666666666666666666666666666666666666666666666666",
        FIXTURE_SUPPLY_CHAIN_SOURCE_REVISION,
    );
    missing_sbom.sbom_path = "artifacts/sbom/citus-pool.json".to_string();
    passed += expect_security_error(
        "invalid-sbom-path",
        missing_sbom.validate(),
        WorkloadSecurityError::InvalidSbomPath,
    )?;

    Ok(passed)
}

fn expect_security_error<T>(
    label: &'static str,
    result: Result<T, WorkloadSecurityError>,
    expected: WorkloadSecurityError,
) -> Result<usize, WorkloadSecurityError> {
    match result {
        Ok(_) => Err(WorkloadSecurityError::FailClosedCheckDidNotFail(label)),
        Err(error) if error == expected => Ok(1),
        Err(error) => Err(error),
    }
}

fn is_sha256_digest_ref(image_ref: &str) -> bool {
    let Some((_, digest)) = image_ref.split_once("@sha256:") else {
        return false;
    };
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_hex_revision(revision: &str) -> bool {
    revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
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
    ExternalSecretBindingMismatch,
    FailClosedCheckDidNotFail(&'static str),
    InlineSecretValue,
    InsecureAuthIssuer,
    InvalidCosignBundlePath,
    InvalidExternalSecretManifest,
    InvalidExternalSecretRefreshInterval,
    InvalidKubernetesName(&'static str),
    InvalidRemoteSecretKey(&'static str),
    InvalidSbomPath,
    InvalidSecretKey(&'static str),
    InvalidSourceRevision,
    InvalidTlsSecretManifest,
    MissingExternalSecretBinding,
    MissingProvenancePredicate,
    MutableImageReference,
    MissingRequiredField(&'static str),
    MissingRbacRule,
    MissingTlsSecret(&'static str),
    NonDefaultSeccomp,
    PrivilegeEscalationAllowed,
    RootContainer,
    SecretRbacForbidden,
    TlsSecretBindingMismatch,
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
            Self::ExternalSecretBindingMismatch => write!(
                formatter,
                "rendered ExternalSecret must match the runtime Secret reference"
            ),
            Self::FailClosedCheckDidNotFail(label) => {
                write!(
                    formatter,
                    "fail-closed security check did not fail: {label}"
                )
            }
            Self::InvalidCosignBundlePath => write!(
                formatter,
                "cosign attestation bundle must use a .sigstore.json artifact path"
            ),
            Self::InvalidExternalSecretManifest => write!(
                formatter,
                "rendered ExternalSecret manifest must use the supported API and kind"
            ),
            Self::InvalidExternalSecretRefreshInterval => {
                write!(
                    formatter,
                    "ExternalSecret refresh interval must be positive"
                )
            }
            Self::InlineSecretValue => write!(
                formatter,
                "secret values must be references, not inline values"
            ),
            Self::InsecureAuthIssuer => write!(formatter, "auth issuer must use https"),
            Self::InvalidKubernetesName(field) => {
                write!(formatter, "{field} must be a valid Kubernetes name")
            }
            Self::InvalidRemoteSecretKey(field) => {
                write!(formatter, "{field} must be a valid remote secret key")
            }
            Self::InvalidSbomPath => write!(
                formatter,
                "SBOM artifact must use a .spdx.json artifact path"
            ),
            Self::InvalidSecretKey(field) => {
                write!(formatter, "{field} must be a valid Secret key")
            }
            Self::InvalidSourceRevision => write!(
                formatter,
                "supply-chain source revision must be a full git SHA"
            ),
            Self::InvalidTlsSecretManifest => write!(
                formatter,
                "rendered TLS Secret manifest must expose tls.crt, tls.key, and ca.crt"
            ),
            Self::MissingExternalSecretBinding => write!(
                formatter,
                "runtime Secret references must be backed by ExternalSecret bindings"
            ),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::MissingProvenancePredicate => write!(
                formatter,
                "cosign attestation metadata must include SLSA provenance predicate type"
            ),
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
            Self::TlsSecretBindingMismatch => write!(
                formatter,
                "rendered TLS Secret metadata must match TLS Secret references"
            ),
            Self::MutableImageReference => write!(
                formatter,
                "supply-chain image references must be immutable sha256 digests"
            ),
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

fn validate_remote_secret_key(
    field: &'static str,
    value: &str,
) -> Result<(), WorkloadSecurityError> {
    let value = value.trim();
    validate_required(field, value)?;
    if value.starts_with('/')
        || value.ends_with('/')
        || value.contains("//")
        || value.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err(WorkloadSecurityError::InvalidRemoteSecretKey(field));
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
                external_secret_bindings: 5,
                rbac_rules: 7,
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
        assert_eq!(plan.external_secrets.len(), 1);
    }

    #[test]
    fn custom_sidecars_inherit_restricted_runtime_boundary() {
        let plan = WorkloadSecurityPlan::sidecar("ai-blaise-citus", "custom-audit-exporter");

        assert_eq!(plan.validate(), Ok(()));
        assert!(plan.denies_kubernetes_api());
        assert!(plan.container_security.read_only_root_filesystem);
    }

    #[test]
    fn rbac_rejects_secret_mutation_and_wildcard_grants() {
        let secret_grant = RbacPolicyPlan {
            access: KubernetesApiAccess::Scoped,
            rules: vec![RbacRulePlan {
                api_group: "".to_string(),
                resources: vec!["secrets".to_string()],
                verbs: vec!["list".to_string()],
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

    #[test]
    fn runtime_secret_refs_must_have_external_secret_bindings() {
        let mut plan = WorkloadSecurityPlan::sidecar("ai-blaise-citus", "mcp");
        plan.external_secrets.clear();

        assert_eq!(
            plan.validate(),
            Err(WorkloadSecurityError::MissingExternalSecretBinding)
        );
    }

    #[test]
    fn external_secret_bindings_reject_unusable_remote_metadata() {
        let mut plan = WorkloadSecurityPlan::sidecar("ai-blaise-citus", "mcp");
        plan.external_secrets[0].remote_key = " sidecars//mcp ".to_string();

        assert_eq!(
            plan.validate(),
            Err(WorkloadSecurityError::InvalidRemoteSecretKey(
                "external_secret.remote_key"
            ))
        );
    }

    #[test]
    fn security_supply_chain_report_is_deterministic() {
        let report = canonical_security_supply_chain_report().expect("supply-chain report");

        assert_eq!(
            report,
            SecuritySupplyChainReport {
                workloads: 6,
                external_secret_manifests: 5,
                runtime_secret_refs: 5,
                tls_manifests: 5,
                tls_secret_refs: 15,
                supply_chain_artifacts: 4,
                sbom_documents: 4,
                cosign_attestations: 4,
                digest_pinned_images: 4,
                fail_closed_checks: 5,
            }
        );
    }

    #[test]
    fn security_supply_chain_rejects_mutable_images_and_bad_attestations() {
        let mut artifact = supply_chain_artifact(
            "operator",
            "7777777777777777777777777777777777777777777777777777777777777777",
            FIXTURE_SUPPLY_CHAIN_SOURCE_REVISION,
        );
        artifact.image_ref = "ghcr.io/ai-blaise/citus-operator:latest".to_string();
        assert_eq!(
            artifact.validate(),
            Err(WorkloadSecurityError::MutableImageReference)
        );

        let mut artifact = supply_chain_artifact(
            "operator",
            "8888888888888888888888888888888888888888888888888888888888888888",
            FIXTURE_SUPPLY_CHAIN_SOURCE_REVISION,
        );
        artifact.cosign_bundle_path = "artifacts/attestations/operator.json".to_string();
        assert_eq!(
            artifact.validate(),
            Err(WorkloadSecurityError::InvalidCosignBundlePath)
        );
    }

    #[test]
    fn rendered_security_manifests_match_runtime_refs() {
        let plan = WorkloadSecurityPlan::pool("ai-blaise-citus");
        let binding = &plan.external_secrets[0];
        let manifest = ExternalSecretManifestPlan::from_binding(binding);
        assert_eq!(manifest.validate(binding), Ok(()));

        let tls_manifest = TlsSecretManifestPlan::from_tls_policy(&plan.tls)
            .expect("tls manifest")
            .expect("tls required");
        assert_eq!(tls_manifest.validate(&plan.tls), Ok(()));
        assert_eq!(tls_manifest.cert_key, "tls.crt");
        assert_eq!(tls_manifest.private_key, "tls.key");
        assert_eq!(tls_manifest.ca_key, "ca.crt");
    }
}
