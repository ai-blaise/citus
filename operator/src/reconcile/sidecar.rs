// FEATURE: O5

use std::error::Error;
use std::fmt;

use crate::crds::sidecar::{
    ResourceRequirements, SidecarDeploymentSpec, SidecarDeploymentSpecError, SidecarDeploymentType,
};

/// Default container port used by every ai-blaise citus sidecar. Matches the
/// `AI_BLAISE_LISTEN_ADDR` default emitted by `sidecar-deployments.yaml` so
/// the reconcile plan can be inlined into the existing chart without
/// renumbering ports.
pub const SIDECAR_DEFAULT_PORT: u16 = 8080;

/// Default grace period applied when scaling a sidecar Deployment to zero on
/// CR deletion. The grace period is measured in seconds.
pub const SIDECAR_DELETION_GRACE_SECONDS: u32 = 30;

/// Suffix appended to the CR name to derive the managed Deployment name.
pub const SIDECAR_DEPLOYMENT_NAME_PREFIX: &str = "ai-blaise-citus-sidecar-";

/// Per-sidecar runtime configuration that the reconciler folds into the
/// generated Deployment. Currently this mirrors only the readiness/health
/// probe paths exposed by every sidecar; future per-type extensions can live
/// here without leaking into the CRD spec layer.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SidecarRuntimeProfile {
    pub readiness_path: String,
    pub health_path: String,
    pub metrics_path: String,
}

impl Default for SidecarRuntimeProfile {
    fn default() -> Self {
        Self {
            readiness_path: "/readyz".to_string(),
            health_path: "/healthz".to_string(),
            metrics_path: "/metrics".to_string(),
        }
    }
}

/// Resolved reconcile plan derived from a `SidecarDeploymentSpec`. The plan
/// captures the Deployment + Service intent generated from the CRD and is
/// agnostic to per-sidecar-type configuration: the reconciler stores the
/// type-specific YAML opaquely under `config_yaml` and lets the sidecar
/// itself parse it on startup.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SidecarReconcilePlan {
    pub sidecar_name: String,
    pub deployment_name: String,
    pub service_name: String,
    pub sidecar_type: SidecarDeploymentType,
    pub replicas: u32,
    pub resources: ResourceRequirements,
    pub image: Option<String>,
    pub config_yaml: Option<String>,
    pub profile: SidecarRuntimeProfile,
    pub port: u16,
    pub deletion_grace_seconds: u32,
}

impl SidecarReconcilePlan {
    /// Build a reconcile plan from the named Sidecar CR. `sidecar_name` is
    /// the `metadata.name` of the owning CRD so the generated Deployment +
    /// Service stay deterministic across reconciliation passes.
    pub fn from_spec(
        sidecar_name: &str,
        spec: &SidecarDeploymentSpec,
    ) -> Result<Self, SidecarReconcileError> {
        Self::from_spec_with_profile(sidecar_name, spec, SidecarRuntimeProfile::default())
    }

    /// Same as [`Self::from_spec`], but lets the caller override the runtime
    /// profile (probe paths) for tests or non-default sidecars.
    pub fn from_spec_with_profile(
        sidecar_name: &str,
        spec: &SidecarDeploymentSpec,
        profile: SidecarRuntimeProfile,
    ) -> Result<Self, SidecarReconcileError> {
        let trimmed = sidecar_name.trim();
        if trimmed.is_empty() {
            return Err(SidecarReconcileError::MissingSidecarName);
        }
        spec.validate()?;

        let suffix = sidecar_name_suffix(&spec.sidecar_type);
        let deployment_name = format!("{SIDECAR_DEPLOYMENT_NAME_PREFIX}{trimmed}-{suffix}");
        let service_name = deployment_name.clone();

        Ok(Self {
            sidecar_name: trimmed.to_string(),
            deployment_name,
            service_name,
            sidecar_type: spec.sidecar_type.clone(),
            replicas: spec.replicas,
            resources: spec.resources.clone(),
            image: spec.image.clone(),
            config_yaml: spec.config_yaml.clone(),
            profile,
            port: SIDECAR_DEFAULT_PORT,
            deletion_grace_seconds: SIDECAR_DELETION_GRACE_SECONDS,
        })
    }

    /// Container image repository derived from the sidecar type. Matches the
    /// `citus-sidecar-<name>` convention used by `sidecar-deployments.yaml`.
    pub fn image_repository(&self) -> String {
        format!("citus-sidecar-{}", sidecar_name_suffix(&self.sidecar_type))
    }

    /// Image reference rendered into the Deployment. Production apply mode
    /// must pass an explicit digest-pinned image through the CR spec; dry-run
    /// mode keeps the historical repository-only convention for plan diffs.
    pub fn image_ref(&self) -> String {
        self.image
            .clone()
            .unwrap_or_else(|| self.image_repository())
    }

    pub fn validate_apply_ready(&self) -> Result<(), SidecarReconcileError> {
        let Some(image) = self.image.as_deref() else {
            return Err(SidecarReconcileError::MissingApplyImage);
        };
        if !is_digest_pinned_image_ref(image) {
            return Err(SidecarReconcileError::MutableImageReference(
                image.to_string(),
            ));
        }
        Ok(())
    }

    /// Render the Kubernetes Deployment manifest emitted for the sidecar. We
    /// emit the manifest as a deterministic string rather than a typed
    /// `k8s_openapi` object so the apply layer can re-use the existing
    /// kubectl patch flow and the tests can assert on the rendered output.
    pub fn deployment_manifest_yaml(&self) -> String {
        let suffix = sidecar_name_suffix(&self.sidecar_type);
        format!(
            r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: {deployment}
  labels:
    app.kubernetes.io/name: ai-blaise-citus
    app.kubernetes.io/component: sidecar-{suffix}
    ai-blaise.com/sidecar-cr: {sidecar_name}
spec:
  replicas: {replicas}
  selector:
    matchLabels:
      app.kubernetes.io/name: ai-blaise-citus
      app.kubernetes.io/component: sidecar-{suffix}
      ai-blaise.com/sidecar-cr: {sidecar_name}
  template:
    metadata:
      labels:
        app.kubernetes.io/name: ai-blaise-citus
        app.kubernetes.io/component: sidecar-{suffix}
        ai-blaise.com/sidecar-cr: {sidecar_name}
    spec:
      securityContext:
        runAsNonRoot: true
        seccompProfile:
          type: RuntimeDefault
      terminationGracePeriodSeconds: {grace}
      containers:
        - name: {suffix}
          image: {image}
          args:
            - serve
          env:
            - name: AI_BLAISE_COMPONENT
              value: "{suffix}"
            - name: AI_BLAISE_LISTEN_ADDR
              value: "0.0.0.0:{port}"
          ports:
            - name: http
              containerPort: {port}
          readinessProbe:
            httpGet:
              path: {readiness_path}
              port: http
            initialDelaySeconds: 3
            periodSeconds: 10
          livenessProbe:
            httpGet:
              path: {health_path}
              port: http
            initialDelaySeconds: 5
            periodSeconds: 20
          securityContext:
            allowPrivilegeEscalation: false
            capabilities:
              drop:
                - ALL
            readOnlyRootFilesystem: true
          resources:
            requests:
              cpu: {cpu}m
              memory: {memory}Mi
            limits:
              cpu: {cpu}m
              memory: {memory}Mi
"#,
            deployment = self.deployment_name,
            suffix = suffix,
            sidecar_name = self.sidecar_name,
            replicas = self.replicas,
            grace = self.deletion_grace_seconds,
            image = self.image_ref(),
            port = self.port,
            readiness_path = self.profile.readiness_path,
            health_path = self.profile.health_path,
            cpu = self.resources.cpu_millis,
            memory = self.resources.memory_mib,
        )
    }

    /// Render the Kubernetes Service manifest emitted for the sidecar.
    pub fn service_manifest_yaml(&self) -> String {
        let suffix = sidecar_name_suffix(&self.sidecar_type);
        format!(
            r#"apiVersion: v1
kind: Service
metadata:
  name: {service}
  labels:
    app.kubernetes.io/name: ai-blaise-citus
    app.kubernetes.io/component: sidecar-{suffix}
    ai-blaise.com/sidecar-cr: {sidecar_name}
spec:
  selector:
    app.kubernetes.io/name: ai-blaise-citus
    app.kubernetes.io/component: sidecar-{suffix}
    ai-blaise.com/sidecar-cr: {sidecar_name}
  ports:
    - name: http
      port: {port}
      targetPort: http
"#,
            service = self.service_name,
            suffix = suffix,
            sidecar_name = self.sidecar_name,
            port = self.port,
        )
    }

    /// Render the strategic-merge patch the reconciler applies when the spec
    /// changes mid-flight. We emit only the fields the operator owns so the
    /// patch is safe to apply repeatedly.
    pub fn deployment_patch_yaml(&self) -> String {
        format!(
            r#"spec:
  replicas: {replicas}
  template:
    spec:
      containers:
        - name: {suffix}
          resources:
            requests:
              cpu: {cpu}m
              memory: {memory}Mi
            limits:
              cpu: {cpu}m
              memory: {memory}Mi
"#,
            replicas = self.replicas,
            suffix = sidecar_name_suffix(&self.sidecar_type),
            cpu = self.resources.cpu_millis,
            memory = self.resources.memory_mib,
        )
    }

    /// Render the deletion plan run when the CR is removed. The reconciler
    /// scales the Deployment to zero first, waits for the grace period, then
    /// deletes the Deployment + Service. We emit the steps as strings so the
    /// reconcile-plans CLI can echo them for live debugging.
    pub fn deletion_plan(&self) -> SidecarDeletionPlan {
        SidecarDeletionPlan {
            steps: vec![
                SidecarDeletionStep::ScaleToZero {
                    deployment: self.deployment_name.clone(),
                },
                SidecarDeletionStep::WaitGrace {
                    deployment: self.deployment_name.clone(),
                    grace_seconds: self.deletion_grace_seconds,
                },
                SidecarDeletionStep::DeleteService {
                    service: self.service_name.clone(),
                },
                SidecarDeletionStep::DeleteDeployment {
                    deployment: self.deployment_name.clone(),
                },
            ],
        }
    }

    /// Render the status probe URLs used to surface `/readyz` + `/metrics`
    /// into the CR's status. The reconciler uses the cluster-local service
    /// DNS name so the probes can run from inside the operator pod.
    pub fn status_probe_urls(&self) -> SidecarStatusProbeUrls {
        SidecarStatusProbeUrls {
            readyz: format!(
                "http://{service}:{port}{path}",
                service = self.service_name,
                port = self.port,
                path = self.profile.readiness_path,
            ),
            metrics: format!(
                "http://{service}:{port}{path}",
                service = self.service_name,
                port = self.port,
                path = self.profile.metrics_path,
            ),
        }
    }
}

/// Deletion plan emitted when the CR is removed.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SidecarDeletionPlan {
    pub steps: Vec<SidecarDeletionStep>,
}

/// Discrete step in a sidecar deletion plan.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SidecarDeletionStep {
    ScaleToZero {
        deployment: String,
    },
    WaitGrace {
        deployment: String,
        grace_seconds: u32,
    },
    DeleteService {
        service: String,
    },
    DeleteDeployment {
        deployment: String,
    },
}

/// Status probe URLs that the reconciler resolves into `.status.readyz` and
/// `.status.metricsUrl` fields on the Sidecar CR.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SidecarStatusProbeUrls {
    pub readyz: String,
    pub metrics: String,
}

fn sidecar_name_suffix(sidecar_type: &SidecarDeploymentType) -> String {
    match sidecar_type {
        SidecarDeploymentType::Analytical => "analytical".to_string(),
        SidecarDeploymentType::Vectorizer => "vectorizer".to_string(),
        SidecarDeploymentType::Cdc => "cdc".to_string(),
        SidecarDeploymentType::ColdTier => "coldtier".to_string(),
        SidecarDeploymentType::Raft => "raft".to_string(),
        SidecarDeploymentType::Hlc => "hlc".to_string(),
        SidecarDeploymentType::TxnStatus => "txn-status".to_string(),
        SidecarDeploymentType::SchemaJob => "schema-job".to_string(),
        SidecarDeploymentType::Realtime => "realtime".to_string(),
        SidecarDeploymentType::Auth => "auth".to_string(),
        SidecarDeploymentType::Storage => "storage".to_string(),
        SidecarDeploymentType::Postgrest => "postgrest".to_string(),
        SidecarDeploymentType::Graphql => "graphql".to_string(),
        SidecarDeploymentType::EdgeFunctions => "edge-functions".to_string(),
        SidecarDeploymentType::Backup => "backup".to_string(),
        SidecarDeploymentType::Repack => "repack".to_string(),
        SidecarDeploymentType::Mcp => "mcp".to_string(),
        SidecarDeploymentType::Custom(name) => sanitize_custom_sidecar_name(name),
    }
}

pub fn is_digest_pinned_image_ref(image: &str) -> bool {
    let Some((_, digest)) = image.rsplit_once("@") else {
        return false;
    };
    if image.trim() != image || image.trim().is_empty() {
        return false;
    }
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sanitize_custom_sidecar_name(raw: &str) -> String {
    raw.trim()
        .chars()
        .map(|character| match character {
            'A'..='Z' => character.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' | '-' => character,
            _ => '-',
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SidecarReconcileError {
    InvalidSpec(SidecarDeploymentSpecError),
    MissingApplyImage,
    MissingSidecarName,
    MutableImageReference(String),
}

impl fmt::Display for SidecarReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(error) => write!(formatter, "{error}"),
            Self::MissingApplyImage => write!(
                formatter,
                "sidecar apply mode requires spec.image to be digest-pinned"
            ),
            Self::MissingSidecarName => write!(formatter, "sidecar_name must not be empty"),
            Self::MutableImageReference(image) => write!(
                formatter,
                "sidecar apply mode requires an immutable sha256 digest image, got {image}"
            ),
        }
    }
}

impl Error for SidecarReconcileError {}

impl From<SidecarDeploymentSpecError> for SidecarReconcileError {
    fn from(error: SidecarDeploymentSpecError) -> Self {
        Self::InvalidSpec(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline_spec(sidecar_type: SidecarDeploymentType) -> SidecarDeploymentSpec {
        SidecarDeploymentSpec {
            sidecar_type,
            replicas: 2,
            resources: ResourceRequirements {
                cpu_millis: 250,
                memory_mib: 512,
            },
            image: None,
            config_yaml: Some("subscriptions:\n  max_per_tenant: 1000".to_string()),
        }
    }

    #[test]
    fn realtime_plan_renders_deployment_and_service_manifest() {
        let spec = baseline_spec(SidecarDeploymentType::Realtime);

        let plan = SidecarReconcilePlan::from_spec("primary", &spec).expect("valid plan");

        assert_eq!(plan.sidecar_name, "primary");
        assert_eq!(
            plan.deployment_name,
            "ai-blaise-citus-sidecar-primary-realtime"
        );
        assert_eq!(plan.service_name, plan.deployment_name);
        assert_eq!(plan.image_repository(), "citus-sidecar-realtime");
        assert_eq!(plan.image_ref(), "citus-sidecar-realtime");
        assert_eq!(plan.replicas, 2);
        assert_eq!(plan.port, SIDECAR_DEFAULT_PORT);

        let deployment = plan.deployment_manifest_yaml();
        assert!(deployment.contains("apiVersion: apps/v1"));
        assert!(deployment.contains("name: ai-blaise-citus-sidecar-primary-realtime"));
        assert!(deployment.contains("ai-blaise.com/sidecar-cr: primary"));
        assert!(deployment.contains("replicas: 2"));
        assert!(deployment.contains("image: citus-sidecar-realtime"));
        assert!(deployment.contains("containerPort: 8080"));
        assert!(deployment.contains("cpu: 250m"));
        assert!(deployment.contains("memory: 512Mi"));
        assert!(deployment.contains("readOnlyRootFilesystem: true"));
        assert!(deployment.contains("terminationGracePeriodSeconds: 30"));
        assert!(deployment.contains("path: /readyz"));
        assert!(deployment.contains("path: /healthz"));

        let service = plan.service_manifest_yaml();
        assert!(service.contains("kind: Service"));
        assert!(service.contains("name: ai-blaise-citus-sidecar-primary-realtime"));
        assert!(service.contains("targetPort: http"));

        let patch = plan.deployment_patch_yaml();
        assert!(patch.contains("replicas: 2"));
        assert!(patch.contains("cpu: 250m"));
        assert!(patch.contains("memory: 512Mi"));

        let urls = plan.status_probe_urls();
        assert_eq!(
            urls.readyz,
            "http://ai-blaise-citus-sidecar-primary-realtime:8080/readyz"
        );
        assert_eq!(
            urls.metrics,
            "http://ai-blaise-citus-sidecar-primary-realtime:8080/metrics"
        );

        let deletion = plan.deletion_plan();
        assert_eq!(deletion.steps.len(), 4);
        assert!(matches!(
            deletion.steps[0],
            SidecarDeletionStep::ScaleToZero { .. }
        ));
        assert!(matches!(
            deletion.steps[1],
            SidecarDeletionStep::WaitGrace {
                grace_seconds: SIDECAR_DELETION_GRACE_SECONDS,
                ..
            }
        ));
        assert!(matches!(
            deletion.steps[2],
            SidecarDeletionStep::DeleteService { .. }
        ));
        assert!(matches!(
            deletion.steps[3],
            SidecarDeletionStep::DeleteDeployment { .. }
        ));
    }

    #[test]
    fn every_typed_sidecar_renders_a_deterministic_suffix() {
        let cases = [
            (SidecarDeploymentType::Analytical, "analytical"),
            (SidecarDeploymentType::Vectorizer, "vectorizer"),
            (SidecarDeploymentType::Cdc, "cdc"),
            (SidecarDeploymentType::ColdTier, "coldtier"),
            (SidecarDeploymentType::Raft, "raft"),
            (SidecarDeploymentType::Hlc, "hlc"),
            (SidecarDeploymentType::TxnStatus, "txn-status"),
            (SidecarDeploymentType::SchemaJob, "schema-job"),
            (SidecarDeploymentType::Realtime, "realtime"),
            (SidecarDeploymentType::Auth, "auth"),
            (SidecarDeploymentType::Storage, "storage"),
            (SidecarDeploymentType::Postgrest, "postgrest"),
            (SidecarDeploymentType::Graphql, "graphql"),
            (SidecarDeploymentType::EdgeFunctions, "edge-functions"),
            (SidecarDeploymentType::Backup, "backup"),
            (SidecarDeploymentType::Repack, "repack"),
            (SidecarDeploymentType::Mcp, "mcp"),
        ];

        for (sidecar_type, expected_suffix) in cases {
            let mut spec = baseline_spec(sidecar_type);
            spec.config_yaml = None;
            let plan = SidecarReconcilePlan::from_spec("alpha", &spec).expect("valid plan");
            assert_eq!(
                plan.deployment_name,
                format!("ai-blaise-citus-sidecar-alpha-{expected_suffix}"),
                "deployment name mismatch for {expected_suffix}"
            );
            assert_eq!(
                plan.image_repository(),
                format!("citus-sidecar-{expected_suffix}"),
                "image repository mismatch for {expected_suffix}"
            );
        }
    }

    #[test]
    fn custom_sidecar_name_is_sanitized_for_kubernetes() {
        let mut spec = baseline_spec(SidecarDeploymentType::Custom(
            "Custom Analytics_v2".to_string(),
        ));
        spec.config_yaml = None;

        let plan = SidecarReconcilePlan::from_spec("alpha", &spec).expect("valid plan");
        assert_eq!(
            plan.deployment_name,
            "ai-blaise-citus-sidecar-alpha-custom-analytics-v2"
        );
        assert_eq!(plan.image_repository(), "citus-sidecar-custom-analytics-v2");
    }

    #[test]
    fn digest_pinned_image_is_rendered_for_apply_mode() {
        let mut spec = baseline_spec(SidecarDeploymentType::Realtime);
        spec.image = Some(
            "127.0.0.1:5001/citus-sidecar-realtime@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
        );

        let plan = SidecarReconcilePlan::from_spec("primary", &spec).expect("valid plan");
        assert_eq!(plan.validate_apply_ready(), Ok(()));
        assert!(plan.deployment_manifest_yaml().contains(
            "image: 127.0.0.1:5001/citus-sidecar-realtime@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ));
    }

    #[test]
    fn apply_mode_rejects_missing_or_mutable_images() {
        let spec = baseline_spec(SidecarDeploymentType::Realtime);
        let plan = SidecarReconcilePlan::from_spec("primary", &spec).expect("valid plan");
        assert_eq!(
            plan.validate_apply_ready(),
            Err(SidecarReconcileError::MissingApplyImage)
        );

        let mut spec = baseline_spec(SidecarDeploymentType::Realtime);
        spec.image = Some("citus-sidecar-realtime:latest".to_string());
        let plan = SidecarReconcilePlan::from_spec("primary", &spec).expect("valid plan");
        assert_eq!(
            plan.validate_apply_ready(),
            Err(SidecarReconcileError::MutableImageReference(
                "citus-sidecar-realtime:latest".to_string()
            ))
        );
    }

    #[test]
    fn empty_sidecar_name_is_rejected() {
        let spec = baseline_spec(SidecarDeploymentType::Realtime);
        assert_eq!(
            SidecarReconcilePlan::from_spec("  ", &spec),
            Err(SidecarReconcileError::MissingSidecarName)
        );
    }

    #[test]
    fn zero_replicas_propagates_validation_error() {
        let mut spec = baseline_spec(SidecarDeploymentType::Realtime);
        spec.replicas = 0;

        assert_eq!(
            SidecarReconcilePlan::from_spec("primary", &spec),
            Err(SidecarReconcileError::InvalidSpec(
                SidecarDeploymentSpecError::InvalidReplicaCount
            ))
        );
    }

    #[test]
    fn config_yaml_is_carried_opaquely_so_per_type_specs_stay_in_the_sidecar() {
        let spec = baseline_spec(SidecarDeploymentType::Realtime);

        let plan = SidecarReconcilePlan::from_spec("primary", &spec).expect("valid plan");
        assert_eq!(
            plan.config_yaml.as_deref(),
            Some("subscriptions:\n  max_per_tenant: 1000")
        );
    }
}
