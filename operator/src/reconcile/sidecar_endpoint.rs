// FEATURE: SC7

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use ai_blaise_citus_sidecar_shared::RetargetDecision;

const ENDPOINT_SLICE_MANAGED_BY: &str = "ai-blaise-citus-operator";
const RETARGET_LABEL: &str = "ai-blaise.citus/endpoint-retarget";
const RETARGET_GENERATION_ANNOTATION: &str = "ai-blaise.citus/retarget-generation";
const RETARGET_REASON_ANNOTATION: &str = "ai-blaise.citus/retarget-reason";
const RETARGET_SELECTED_ANNOTATION: &str = "ai-blaise.citus/retarget-selected";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EndpointSliceAddressType {
    Ipv4,
    Ipv6,
    Fqdn,
}

impl EndpointSliceAddressType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ipv4 => "IPv4",
            Self::Ipv6 => "IPv6",
            Self::Fqdn => "FQDN",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SidecarEndpointCandidate {
    pub endpoint_id: String,
    pub target_ref_name: String,
    pub addresses: Vec<String>,
    pub port_name: String,
    pub port: u16,
    pub zone: Option<String>,
    pub ready: bool,
}

impl SidecarEndpointCandidate {
    pub fn validate(&self) -> Result<(), SidecarEndpointRetargetError> {
        validate_dns_label("endpoint_id", &self.endpoint_id)?;
        validate_dns_label("target_ref_name", &self.target_ref_name)?;
        validate_dns_label("port_name", &self.port_name)?;
        if self.port == 0 {
            return Err(SidecarEndpointRetargetError::InvalidPort(self.port));
        }
        if self.addresses.is_empty() {
            return Err(SidecarEndpointRetargetError::MissingAddresses(
                self.endpoint_id.clone(),
            ));
        }
        for address in &self.addresses {
            validate_address(address)?;
        }
        if let Some(zone) = &self.zone {
            validate_kubernetes_value("zone", zone)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SidecarEndpointRetargetStatus {
    Active,
    FailClosed,
}

impl SidecarEndpointRetargetStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::FailClosed => "fail-closed",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SidecarEndpointRetargetPlan {
    pub service_name: String,
    pub endpoint_slice_name: String,
    pub sidecar_name: String,
    pub address_type: EndpointSliceAddressType,
    pub port_name: String,
    pub port: u16,
    pub generation: u64,
    pub selected_endpoint_id: Option<String>,
    pub status: SidecarEndpointRetargetStatus,
    pub reason: String,
    pub endpoints: Vec<SidecarEndpointCandidate>,
}

impl SidecarEndpointRetargetPlan {
    pub fn from_decision(
        service_name: impl Into<String>,
        sidecar_name: impl Into<String>,
        decision: &RetargetDecision,
        candidates: Vec<SidecarEndpointCandidate>,
    ) -> Result<Self, SidecarEndpointRetargetError> {
        let service_name = service_name.into();
        let sidecar_name = sidecar_name.into();
        validate_dns_label("service_name", &service_name)?;
        validate_dns_label("sidecar_name", &sidecar_name)?;

        let mut seen_ids = BTreeSet::new();
        for candidate in &candidates {
            candidate.validate()?;
            if !seen_ids.insert(candidate.endpoint_id.clone()) {
                return Err(SidecarEndpointRetargetError::DuplicateEndpointId(
                    candidate.endpoint_id.clone(),
                ));
            }
        }

        let endpoint_slice_name = format!("{service_name}-retarget");
        let (port_name, port) = candidates
            .first()
            .map(|candidate| (candidate.port_name.clone(), candidate.port))
            .unwrap_or_else(|| ("http".to_string(), 8080));
        for candidate in &candidates {
            if candidate.port_name != port_name || candidate.port != port {
                return Err(SidecarEndpointRetargetError::InconsistentServicePort {
                    endpoint_id: candidate.endpoint_id.clone(),
                    expected: format!("{port_name}:{port}"),
                    actual: format!("{}:{}", candidate.port_name, candidate.port),
                });
            }
        }
        let mut endpoints = Vec::new();
        let mut status = SidecarEndpointRetargetStatus::FailClosed;
        let mut selected_endpoint_id = None;
        let mut reason = decision.reason.clone();

        if let Some(selection) = &decision.selection {
            let mut selected = candidates
                .into_iter()
                .find(|candidate| candidate.endpoint_id == selection.id)
                .ok_or_else(|| {
                    SidecarEndpointRetargetError::SelectedEndpointMissing(selection.id.clone())
                })?;
            if selected.ready {
                selected.addresses.sort();
                selected_endpoint_id = Some(selected.endpoint_id.clone());
                status = SidecarEndpointRetargetStatus::Active;
                endpoints.push(selected);
            } else {
                reason = format!(
                    "selected endpoint {} is not Kubernetes-ready; fail closed",
                    selection.id
                );
            }
        }

        Ok(Self {
            service_name,
            endpoint_slice_name,
            sidecar_name,
            address_type: EndpointSliceAddressType::Ipv4,
            port_name,
            port,
            generation: decision.generation,
            selected_endpoint_id,
            status,
            reason,
            endpoints,
        })
    }

    pub fn validate(&self) -> Result<(), SidecarEndpointRetargetError> {
        validate_dns_label("service_name", &self.service_name)?;
        validate_dns_label("endpoint_slice_name", &self.endpoint_slice_name)?;
        validate_dns_label("sidecar_name", &self.sidecar_name)?;
        if self.status == SidecarEndpointRetargetStatus::Active && self.endpoints.len() != 1 {
            return Err(SidecarEndpointRetargetError::ActivePlanRequiresOneEndpoint(
                self.endpoints.len(),
            ));
        }
        if self.status == SidecarEndpointRetargetStatus::FailClosed && !self.endpoints.is_empty() {
            return Err(SidecarEndpointRetargetError::FailClosedPlanMustBeEmpty);
        }
        for endpoint in &self.endpoints {
            endpoint.validate()?;
        }
        Ok(())
    }

    pub fn endpoint_slice_manifest_yaml(&self) -> Result<String, SidecarEndpointRetargetError> {
        self.validate()?;
        let selected = self.selected_endpoint_id.as_deref().unwrap_or("none");
        let mut yaml = format!(
            "apiVersion: discovery.k8s.io/v1\nkind: EndpointSlice\nmetadata:\n  name: {slice}\n  labels:\n    kubernetes.io/service-name: {service}\n    endpointslice.kubernetes.io/managed-by: {managed_by}\n    ai-blaise.citus/sidecar: {sidecar}\n    {retarget_label}: managed\n  annotations:\n    {generation_annotation}: {generation}\n    {selected_annotation}: {selected}\n    {reason_annotation}: {reason}\naddressType: {address_type}\nports:\n  - name: {port_name}\n    protocol: TCP\n    port: {port}\n",
            slice = self.endpoint_slice_name,
            service = self.service_name,
            managed_by = ENDPOINT_SLICE_MANAGED_BY,
            sidecar = self.sidecar_name,
            retarget_label = RETARGET_LABEL,
            generation_annotation = RETARGET_GENERATION_ANNOTATION,
            generation = self.generation,
            selected_annotation = RETARGET_SELECTED_ANNOTATION,
            selected = selected,
            reason_annotation = RETARGET_REASON_ANNOTATION,
            reason = yaml_quote(&self.reason),
            address_type = self.address_type.as_str(),
            port_name = self.port_name.as_str(),
            port = self.port,
        );

        if self.endpoints.is_empty() {
            yaml.push_str("endpoints: []\n");
            return Ok(yaml);
        }

        yaml.push_str("endpoints:\n");
        for endpoint in &self.endpoints {
            yaml.push_str("  - addresses:\n");
            for address in &endpoint.addresses {
                yaml.push_str(&format!("      - {}\n", yaml_quote(address)));
            }
            yaml.push_str(
                "    conditions:\n      ready: true\n      serving: true\n      terminating: false\n",
            );
            if let Some(zone) = &endpoint.zone {
                yaml.push_str(&format!("    zone: {}\n", yaml_quote(zone)));
            }
            yaml.push_str(&format!(
                "    targetRef:\n      kind: Pod\n      name: {}\n",
                endpoint.target_ref_name
            ));
        }
        Ok(yaml)
    }

    pub fn service_merge_patch_json(&self) -> Result<String, SidecarEndpointRetargetError> {
        self.validate()?;
        let selected = self.selected_endpoint_id.as_deref().unwrap_or("none");
        Ok(format!(
            "{{\n  \"metadata\": {{\n    \"annotations\": {{\n      \"{generation_annotation}\": \"{generation}\",\n      \"{reason_annotation}\": {reason},\n      \"{selected_annotation}\": \"{selected}\"\n    }},\n    \"labels\": {{\n      \"{retarget_label}\": \"managed\"\n    }}\n  }},\n  \"spec\": {{\n    \"selector\": null\n  }}\n}}\n",
            generation_annotation = RETARGET_GENERATION_ANNOTATION,
            generation = self.generation,
            reason_annotation = RETARGET_REASON_ANNOTATION,
            reason = json_quote(&self.reason),
            selected_annotation = RETARGET_SELECTED_ANNOTATION,
            selected = json_escape(selected),
            retarget_label = RETARGET_LABEL,
        ))
    }

    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SidecarEndpointRetargetError {
    InvalidDnsLabel {
        field: &'static str,
        value: String,
    },
    InvalidKubernetesValue {
        field: &'static str,
        value: String,
    },
    InvalidAddress(String),
    InvalidPort(u16),
    MissingAddresses(String),
    DuplicateEndpointId(String),
    InconsistentServicePort {
        endpoint_id: String,
        expected: String,
        actual: String,
    },
    SelectedEndpointMissing(String),
    ActivePlanRequiresOneEndpoint(usize),
    FailClosedPlanMustBeEmpty,
}

impl fmt::Display for SidecarEndpointRetargetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDnsLabel { field, value } => {
                write!(formatter, "invalid Kubernetes DNS label {field}: {value}")
            }
            Self::InvalidKubernetesValue { field, value } => {
                write!(formatter, "invalid Kubernetes value {field}: {value}")
            }
            Self::InvalidAddress(address) => {
                write!(formatter, "invalid EndpointSlice address: {address}")
            }
            Self::InvalidPort(port) => write!(formatter, "invalid EndpointSlice port: {port}"),
            Self::MissingAddresses(id) => {
                write!(formatter, "endpoint candidate {id} has no addresses")
            }
            Self::DuplicateEndpointId(id) => {
                write!(formatter, "duplicate endpoint candidate id: {id}")
            }
            Self::InconsistentServicePort {
                endpoint_id,
                expected,
                actual,
            } => write!(
                formatter,
                "endpoint candidate {endpoint_id} uses service port {actual}, expected {expected}"
            ),
            Self::SelectedEndpointMissing(id) => {
                write!(
                    formatter,
                    "selected endpoint has no Kubernetes candidate: {id}"
                )
            }
            Self::ActivePlanRequiresOneEndpoint(count) => write!(
                formatter,
                "active EndpointSlice retarget plan must contain exactly one endpoint, got {count}"
            ),
            Self::FailClosedPlanMustBeEmpty => write!(
                formatter,
                "fail-closed EndpointSlice retarget plan must not contain endpoints"
            ),
        }
    }
}

impl Error for SidecarEndpointRetargetError {}

fn validate_dns_label(
    field: &'static str,
    value: &str,
) -> Result<(), SidecarEndpointRetargetError> {
    let bytes = value.as_bytes();
    let valid = !bytes.is_empty()
        && bytes.len() <= 63
        && bytes[0].is_ascii_alphanumeric()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(SidecarEndpointRetargetError::InvalidDnsLabel {
            field,
            value: value.to_string(),
        })
    }
}

fn validate_kubernetes_value(
    field: &'static str,
    value: &str,
) -> Result<(), SidecarEndpointRetargetError> {
    let valid = !value.trim().is_empty()
        && value.len() <= 63
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        });
    if valid {
        Ok(())
    } else {
        Err(SidecarEndpointRetargetError::InvalidKubernetesValue {
            field,
            value: value.to_string(),
        })
    }
}

fn validate_address(address: &str) -> Result<(), SidecarEndpointRetargetError> {
    if address.trim().is_empty() || address.chars().any(char::is_whitespace) {
        Err(SidecarEndpointRetargetError::InvalidAddress(
            address.to_string(),
        ))
    } else {
        Ok(())
    }
}

fn yaml_quote(value: &str) -> String {
    format!("\"{}\"", json_escape(value))
}

fn json_quote(value: &str) -> String {
    format!("\"{}\"", json_escape(value))
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_blaise_citus_sidecar_shared::{EndpointRegistry, RetargetConfig};

    #[test]
    fn active_plan_renders_selected_endpoint_slice_and_service_patch() {
        let config = RetargetConfig::parse(
            "id=primary,target=http://primary:8080,priority=1;\
             id=standby,target=http://standby:8080,priority=2",
        )
        .unwrap();
        let registry = EndpointRegistry::new(config);
        let plan = SidecarEndpointRetargetPlan::from_decision(
            "ai-blaise-realtime",
            "realtime",
            &registry.select(),
            candidates(),
        )
        .unwrap();

        assert_eq!(plan.status, SidecarEndpointRetargetStatus::Active);
        assert_eq!(plan.selected_endpoint_id.as_deref(), Some("primary"));
        assert_eq!(plan.endpoint_count(), 1);
        let manifest = plan.endpoint_slice_manifest_yaml().unwrap();
        assert!(manifest.contains("kind: EndpointSlice"));
        assert!(manifest.contains("name: ai-blaise-realtime-retarget"));
        assert!(manifest.contains("kubernetes.io/service-name: ai-blaise-realtime"));
        assert!(manifest.contains("ai-blaise.citus/retarget-selected: primary"));
        assert!(manifest.contains("- \"10.0.0.10\""));
        assert!(manifest.contains("name: realtime-primary-0"));

        let patch = plan.service_merge_patch_json().unwrap();
        assert!(patch.contains("\"selector\": null"));
        assert!(patch.contains("\"ai-blaise.citus/retarget-selected\": \"primary\""));
    }

    #[test]
    fn no_selection_renders_empty_fail_closed_slice() {
        let config = RetargetConfig::parse(
            "id=primary,target=http://primary:8080,priority=1,failover_after=1;\
             id=standby,target=http://standby:8080,priority=2,failover_after=1",
        )
        .unwrap();
        let mut registry = EndpointRegistry::new(config);
        registry.record_failure("primary", "down").unwrap();
        registry.record_failure("standby", "down").unwrap();

        let plan = SidecarEndpointRetargetPlan::from_decision(
            "ai-blaise-realtime",
            "realtime",
            &registry.select(),
            candidates(),
        )
        .unwrap();

        assert_eq!(plan.status, SidecarEndpointRetargetStatus::FailClosed);
        assert_eq!(plan.selected_endpoint_id, None);
        assert_eq!(plan.endpoint_count(), 0);
        assert!(plan
            .endpoint_slice_manifest_yaml()
            .unwrap()
            .contains("endpoints: []"));
        assert!(plan
            .service_merge_patch_json()
            .unwrap()
            .contains("\"ai-blaise.citus/retarget-selected\": \"none\""));
    }

    #[test]
    fn selected_kubernetes_not_ready_fails_closed() {
        let config =
            RetargetConfig::parse("id=primary,target=http://primary:8080,priority=1").unwrap();
        let registry = EndpointRegistry::new(config);
        let mut candidates = candidates();
        candidates[0].ready = false;

        let plan = SidecarEndpointRetargetPlan::from_decision(
            "ai-blaise-realtime",
            "realtime",
            &registry.select(),
            candidates,
        )
        .unwrap();

        assert_eq!(plan.status, SidecarEndpointRetargetStatus::FailClosed);
        assert_eq!(plan.endpoint_count(), 0);
        assert!(plan.reason.contains("not Kubernetes-ready"));
    }

    #[test]
    fn duplicate_candidate_ids_are_rejected() {
        let config =
            RetargetConfig::parse("id=primary,target=http://primary:8080,priority=1").unwrap();
        let registry = EndpointRegistry::new(config);
        let mut candidates = candidates();
        candidates[1].endpoint_id = "primary".to_string();

        assert_eq!(
            SidecarEndpointRetargetPlan::from_decision(
                "ai-blaise-realtime",
                "realtime",
                &registry.select(),
                candidates,
            ),
            Err(SidecarEndpointRetargetError::DuplicateEndpointId(
                "primary".to_string()
            ))
        );
    }

    fn candidates() -> Vec<SidecarEndpointCandidate> {
        vec![
            SidecarEndpointCandidate {
                endpoint_id: "primary".to_string(),
                target_ref_name: "realtime-primary-0".to_string(),
                addresses: vec!["10.0.0.10".to_string()],
                port_name: "http".to_string(),
                port: 8080,
                zone: Some("us-east1-b".to_string()),
                ready: true,
            },
            SidecarEndpointCandidate {
                endpoint_id: "standby".to_string(),
                target_ref_name: "realtime-standby-0".to_string(),
                addresses: vec!["10.0.1.10".to_string()],
                port_name: "http".to_string(),
                port: 8080,
                zone: Some("us-east1-c".to_string()),
                ready: true,
            },
        ]
    }
}
