//! Storage sidecar contracts.

// FEATURE: Sto1
// FEATURE: Sto3
// FEATURE: Sto4
// FEATURE: Sto5

use ai_blaise_citus_sidecar_shared::{SidecarContractError, StorageContract};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StorageSidecarPlan {
    pub contract: StorageContract,
    pub provider: ObjectStoreProvider,
    pub buckets: Vec<BucketPolicy>,
    pub antivirus: Option<AntivirusPlan>,
}

impl StorageSidecarPlan {
    pub fn validate(&self) -> Result<(), StorageSidecarError> {
        self.contract.validate()?;
        if self.buckets.is_empty() {
            return Err(StorageSidecarError::MissingRequiredField("buckets"));
        }
        for bucket in &self.buckets {
            bucket.validate()?;
        }
        if let Some(antivirus) = &self.antivirus {
            antivirus.validate()?;
        }
        Ok(())
    }

    fn bucket_policy(
        &self,
        bucket: &str,
        tenant_id: &str,
    ) -> Result<&BucketPolicy, StorageSidecarError> {
        self.buckets
            .iter()
            .find(|policy| policy.bucket == bucket && policy.tenant_id == tenant_id)
            .ok_or(StorageSidecarError::PolicyNotFound)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ObjectStoreProvider {
    S3,
    Gcs,
    AzureBlob,
    Minio,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BucketPolicy {
    pub bucket: String,
    pub tenant_id: String,
    pub acl: BucketAcl,
    pub max_object_bytes: u64,
}

impl BucketPolicy {
    fn validate(&self) -> Result<(), StorageSidecarError> {
        validate_required("bucket", &self.bucket)?;
        validate_required("tenant_id", &self.tenant_id)?;
        if self.max_object_bytes == 0 {
            return Err(StorageSidecarError::InvalidObjectSize);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BucketAcl {
    Private,
    TenantRead,
    TenantReadWrite,
    PublicRead,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ObjectMetadataRecord {
    pub bucket: String,
    pub object_key: String,
    pub tenant_id: String,
    pub content_type: String,
    pub size_bytes: u64,
}

impl ObjectMetadataRecord {
    pub fn validate(&self) -> Result<(), StorageSidecarError> {
        validate_required("metadata.bucket", &self.bucket)?;
        validate_required("metadata.object_key", &self.object_key)?;
        validate_required("metadata.tenant_id", &self.tenant_id)?;
        validate_required("metadata.content_type", &self.content_type)?;
        if self.size_bytes == 0 {
            return Err(StorageSidecarError::InvalidObjectSize);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PresignedUrlPlan {
    pub bucket: String,
    pub object_key: String,
    pub tenant_id: String,
    pub method: PresignedMethod,
    pub ttl_seconds: u32,
}

impl PresignedUrlPlan {
    pub fn validate(&self) -> Result<(), StorageSidecarError> {
        validate_required("presign.bucket", &self.bucket)?;
        validate_required("presign.object_key", &self.object_key)?;
        validate_required("presign.tenant_id", &self.tenant_id)?;
        if self.ttl_seconds == 0 {
            return Err(StorageSidecarError::InvalidPresignedTtl);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PresignedMethod {
    Get,
    Put,
    Delete,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AntivirusPlan {
    pub scanner_endpoint: String,
    pub quarantine_bucket: String,
    pub fail_closed: bool,
}

impl AntivirusPlan {
    fn validate(&self) -> Result<(), StorageSidecarError> {
        validate_required("antivirus.scanner_endpoint", &self.scanner_endpoint)?;
        if !self.scanner_endpoint.starts_with("http://")
            && !self.scanner_endpoint.starts_with("https://")
        {
            return Err(StorageSidecarError::InvalidScannerEndpoint);
        }
        validate_required("antivirus.quarantine_bucket", &self.quarantine_bucket)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StorageSidecarError {
    AccessDenied(&'static str),
    InvalidObjectSize,
    InvalidPresignedTtl,
    InvalidScannerEndpoint,
    MissingRequiredField(&'static str),
    ObjectTooLarge {
        size_bytes: u64,
        max_object_bytes: u64,
    },
    PolicyNotFound,
    PresignedTtlExceedsPolicy {
        ttl_seconds: u32,
        max_ttl_seconds: u32,
    },
    SharedContract(String),
}

impl fmt::Display for StorageSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AccessDenied(reason) => write!(formatter, "storage access denied: {reason}"),
            Self::InvalidObjectSize => write!(formatter, "object size must be greater than zero"),
            Self::InvalidPresignedTtl => {
                write!(formatter, "presigned URL TTL must be greater than zero")
            }
            Self::InvalidScannerEndpoint => {
                write!(
                    formatter,
                    "scanner endpoint must start with http:// or https://"
                )
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::ObjectTooLarge {
                size_bytes,
                max_object_bytes,
            } => write!(
                formatter,
                "object size {size_bytes} exceeds bucket limit {max_object_bytes}"
            ),
            Self::PolicyNotFound => write!(formatter, "no bucket policy matched bucket and tenant"),
            Self::PresignedTtlExceedsPolicy {
                ttl_seconds,
                max_ttl_seconds,
            } => write!(
                formatter,
                "presigned URL TTL {ttl_seconds} exceeds policy {max_ttl_seconds}"
            ),
            Self::SharedContract(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for StorageSidecarError {}

impl From<SidecarContractError> for StorageSidecarError {
    fn from(error: SidecarContractError) -> Self {
        Self::SharedContract(error.to_string())
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), StorageSidecarError> {
    if value.trim().is_empty() {
        return Err(StorageSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ObjectUploadRequest {
    pub metadata: ObjectMetadataRecord,
    pub content_digest: String,
    pub scan_signature: String,
}

impl ObjectUploadRequest {
    pub fn validate(&self) -> Result<(), StorageSidecarError> {
        self.metadata.validate()?;
        validate_required("upload.content_digest", &self.content_digest)?;
        validate_required("upload.scan_signature", &self.scan_signature)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AntivirusVerdict {
    Clean,
    Infected,
    NotScanned,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AntivirusScanResult {
    pub object_key: String,
    pub verdict: AntivirusVerdict,
    pub scanner_endpoint: Option<String>,
    pub quarantined: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StoredObjectRecord {
    pub metadata: ObjectMetadataRecord,
    pub content_digest: String,
    pub antivirus_verdict: AntivirusVerdict,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ObjectUploadResult {
    pub metadata: ObjectMetadataRecord,
    pub content_digest: String,
    pub antivirus_verdict: AntivirusVerdict,
    pub stored: bool,
    pub quarantined: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PresignedUrlIssue {
    pub plan: PresignedUrlPlan,
    pub url: String,
    pub expires_in_seconds: u32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StorageRuntimeState {
    pub stored_objects: usize,
    pub quarantined_objects: usize,
    pub issued_urls: usize,
    pub scanned_objects: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StorageRuntimeReport {
    pub upload: ObjectUploadResult,
    pub presigned_url: PresignedUrlIssue,
    pub state: StorageRuntimeState,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StorageRuntime {
    plan: StorageSidecarPlan,
    objects: Vec<StoredObjectRecord>,
    quarantine: Vec<StoredObjectRecord>,
    issued_urls: Vec<PresignedUrlIssue>,
    scans: Vec<AntivirusScanResult>,
}

impl StorageRuntime {
    pub fn new(plan: StorageSidecarPlan) -> Result<Self, StorageSidecarError> {
        plan.validate()?;

        Ok(Self {
            plan,
            objects: Vec::new(),
            quarantine: Vec::new(),
            issued_urls: Vec::new(),
            scans: Vec::new(),
        })
    }

    pub fn state(&self) -> StorageRuntimeState {
        StorageRuntimeState {
            stored_objects: self.objects.len(),
            quarantined_objects: self.quarantine.len(),
            issued_urls: self.issued_urls.len(),
            scanned_objects: self.scans.len(),
        }
    }

    pub fn issue_presigned_url(
        &mut self,
        plan: &PresignedUrlPlan,
    ) -> Result<PresignedUrlIssue, StorageSidecarError> {
        plan.validate()?;
        if plan.ttl_seconds > self.plan.contract.presigned_url_ttl_seconds {
            return Err(StorageSidecarError::PresignedTtlExceedsPolicy {
                ttl_seconds: plan.ttl_seconds,
                max_ttl_seconds: self.plan.contract.presigned_url_ttl_seconds,
            });
        }

        let bucket_policy = self.plan.bucket_policy(&plan.bucket, &plan.tenant_id)?;
        if !acl_allows_method(bucket_policy.acl, plan.method) {
            return Err(StorageSidecarError::AccessDenied(
                "bucket ACL does not allow presigned method",
            ));
        }

        let issue = PresignedUrlIssue {
            plan: plan.clone(),
            url: deterministic_presigned_url(&self.plan.provider, plan),
            expires_in_seconds: plan.ttl_seconds,
        };
        self.issued_urls.push(issue.clone());
        Ok(issue)
    }

    pub fn put_object(
        &mut self,
        request: &ObjectUploadRequest,
    ) -> Result<ObjectUploadResult, StorageSidecarError> {
        request.validate()?;
        let bucket_policy = self
            .plan
            .bucket_policy(&request.metadata.bucket, &request.metadata.tenant_id)?;
        if bucket_policy.acl != BucketAcl::TenantReadWrite {
            return Err(StorageSidecarError::AccessDenied(
                "bucket ACL does not allow object writes",
            ));
        }
        if request.metadata.size_bytes > bucket_policy.max_object_bytes {
            return Err(StorageSidecarError::ObjectTooLarge {
                size_bytes: request.metadata.size_bytes,
                max_object_bytes: bucket_policy.max_object_bytes,
            });
        }

        let scan = self.scan_object(request);
        let quarantined = scan.quarantined;
        let record = StoredObjectRecord {
            metadata: request.metadata.clone(),
            content_digest: request.content_digest.clone(),
            antivirus_verdict: scan.verdict,
        };

        if quarantined {
            self.quarantine.push(record);
        } else {
            self.objects.push(record);
        }
        self.scans.push(scan.clone());

        Ok(ObjectUploadResult {
            metadata: request.metadata.clone(),
            content_digest: request.content_digest.clone(),
            antivirus_verdict: scan.verdict,
            stored: !quarantined,
            quarantined,
        })
    }

    fn scan_object(&self, request: &ObjectUploadRequest) -> AntivirusScanResult {
        let verdict = match &self.plan.antivirus {
            Some(_) if is_infected_signature(&request.scan_signature) => AntivirusVerdict::Infected,
            Some(_) => AntivirusVerdict::Clean,
            None => AntivirusVerdict::NotScanned,
        };

        AntivirusScanResult {
            object_key: request.metadata.object_key.clone(),
            scanner_endpoint: self
                .plan
                .antivirus
                .as_ref()
                .map(|plan| plan.scanner_endpoint.clone()),
            quarantined: verdict == AntivirusVerdict::Infected,
            verdict,
        }
    }
}

fn acl_allows_method(acl: BucketAcl, method: PresignedMethod) -> bool {
    match method {
        PresignedMethod::Get => matches!(
            acl,
            BucketAcl::TenantRead | BucketAcl::TenantReadWrite | BucketAcl::PublicRead
        ),
        PresignedMethod::Put | PresignedMethod::Delete => acl == BucketAcl::TenantReadWrite,
    }
}

fn deterministic_presigned_url(provider: &ObjectStoreProvider, plan: &PresignedUrlPlan) -> String {
    format!(
        "https://{}.ai-blaise.local/{}/{}?method={}&tenant={}&ttl={}&signature=ai-blaise-canonical",
        provider_slug(provider),
        plan.bucket,
        plan.object_key,
        method_slug(&plan.method),
        plan.tenant_id,
        plan.ttl_seconds,
    )
}

fn provider_slug(provider: &ObjectStoreProvider) -> &'static str {
    match provider {
        ObjectStoreProvider::S3 => "s3",
        ObjectStoreProvider::Gcs => "gcs",
        ObjectStoreProvider::AzureBlob => "azure-blob",
        ObjectStoreProvider::Minio => "minio",
    }
}

fn method_slug(method: &PresignedMethod) -> &'static str {
    match method {
        PresignedMethod::Get => "get",
        PresignedMethod::Put => "put",
        PresignedMethod::Delete => "delete",
    }
}

fn is_infected_signature(signature: &str) -> bool {
    let signature = signature.to_ascii_lowercase();
    signature.contains("malware") || signature.contains("eicar")
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StorageCanonicalReport {
    pub plan: StorageSidecarPlan,
    pub metadata: ObjectMetadataRecord,
    pub presigned_url: PresignedUrlPlan,
}

pub fn canonical_storage_plan() -> StorageSidecarPlan {
    StorageSidecarPlan {
        contract: StorageContract {
            bucket: "tenant-files".to_string(),
            metadata_table: "storage.objects".to_string(),
            presigned_url_ttl_seconds: 900,
            acl_tenant_column: "tenant_id".to_string(),
        },
        provider: ObjectStoreProvider::S3,
        buckets: vec![BucketPolicy {
            bucket: "tenant-files".to_string(),
            tenant_id: "tenant-a".to_string(),
            acl: BucketAcl::TenantReadWrite,
            max_object_bytes: 10_485_760,
        }],
        antivirus: Some(AntivirusPlan {
            scanner_endpoint: "http://clamav.storage.svc:3310".to_string(),
            quarantine_bucket: "quarantine".to_string(),
            fail_closed: true,
        }),
    }
}

pub fn canonical_metadata_record() -> ObjectMetadataRecord {
    ObjectMetadataRecord {
        bucket: "tenant-files".to_string(),
        object_key: "orders/1.pdf".to_string(),
        tenant_id: "tenant-a".to_string(),
        content_type: "application/pdf".to_string(),
        size_bytes: 42,
    }
}

pub fn canonical_presigned_url_plan() -> PresignedUrlPlan {
    PresignedUrlPlan {
        bucket: "tenant-files".to_string(),
        object_key: "orders/1.pdf".to_string(),
        tenant_id: "tenant-a".to_string(),
        method: PresignedMethod::Put,
        ttl_seconds: 900,
    }
}

pub fn canonical_upload_request() -> ObjectUploadRequest {
    ObjectUploadRequest {
        metadata: canonical_metadata_record(),
        content_digest: "sha256:6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"
            .to_string(),
        scan_signature: "clean:application-pdf".to_string(),
    }
}

pub fn canonical_storage_report() -> Result<StorageCanonicalReport, StorageSidecarError> {
    let plan = canonical_storage_plan();
    let metadata = canonical_metadata_record();
    let presigned_url = canonical_presigned_url_plan();

    plan.validate()?;
    metadata.validate()?;
    presigned_url.validate()?;

    Ok(StorageCanonicalReport {
        plan,
        metadata,
        presigned_url,
    })
}

pub fn canonical_storage_runtime_report() -> Result<StorageRuntimeReport, StorageSidecarError> {
    let mut runtime = StorageRuntime::new(canonical_storage_plan())?;
    let presigned_url = runtime.issue_presigned_url(&canonical_presigned_url_plan())?;
    let upload = runtime.put_object(&canonical_upload_request())?;

    Ok(StorageRuntimeReport {
        upload,
        presigned_url,
        state: runtime.state(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_sidecar_plan_validates_bucket_and_antivirus() {
        assert_eq!(canonical_storage_plan().validate(), Ok(()));
    }

    #[test]
    fn canonical_storage_report_is_deterministic() {
        let report = canonical_storage_report().expect("canonical report");

        assert_eq!(report.plan.contract.bucket, "tenant-files");
        assert_eq!(report.metadata.object_key, "orders/1.pdf");
        assert_eq!(report.presigned_url.ttl_seconds, 900);
    }

    #[test]
    fn storage_runtime_stores_clean_object_and_issues_url() {
        let report = canonical_storage_runtime_report().expect("runtime report");

        assert!(report.upload.stored);
        assert!(!report.upload.quarantined);
        assert_eq!(report.upload.antivirus_verdict, AntivirusVerdict::Clean);
        assert_eq!(report.state.stored_objects, 1);
        assert_eq!(report.state.quarantined_objects, 0);
        assert_eq!(report.state.issued_urls, 1);
        assert_eq!(report.state.scanned_objects, 1);
        assert_eq!(
            report.presigned_url.url,
            "https://s3.ai-blaise.local/tenant-files/orders/1.pdf?method=put&tenant=tenant-a&ttl=900&signature=ai-blaise-canonical"
        );
    }

    #[test]
    fn storage_runtime_quarantines_infected_signature() {
        let mut runtime = StorageRuntime::new(canonical_storage_plan()).expect("runtime");
        let mut request = canonical_upload_request();
        request.scan_signature = "malware:eicar-test".to_string();

        let upload = runtime.put_object(&request).expect("upload");

        assert!(!upload.stored);
        assert!(upload.quarantined);
        assert_eq!(upload.antivirus_verdict, AntivirusVerdict::Infected);
        assert_eq!(runtime.state().stored_objects, 0);
        assert_eq!(runtime.state().quarantined_objects, 1);
    }

    #[test]
    fn storage_runtime_rejects_oversize_object() {
        let mut runtime = StorageRuntime::new(canonical_storage_plan()).expect("runtime");
        let mut request = canonical_upload_request();
        request.metadata.size_bytes = 10_485_761;

        assert_eq!(
            runtime.put_object(&request),
            Err(StorageSidecarError::ObjectTooLarge {
                size_bytes: 10_485_761,
                max_object_bytes: 10_485_760,
            })
        );
    }

    #[test]
    fn storage_runtime_rejects_presign_ttl_over_policy() {
        let mut runtime = StorageRuntime::new(canonical_storage_plan()).expect("runtime");
        let mut plan = canonical_presigned_url_plan();
        plan.ttl_seconds = 901;

        assert_eq!(
            runtime.issue_presigned_url(&plan),
            Err(StorageSidecarError::PresignedTtlExceedsPolicy {
                ttl_seconds: 901,
                max_ttl_seconds: 900,
            })
        );
    }

    #[test]
    fn metadata_record_requires_positive_size() {
        let mut metadata = canonical_metadata_record();
        metadata.size_bytes = 0;

        assert_eq!(
            metadata.validate(),
            Err(StorageSidecarError::InvalidObjectSize)
        );
    }

    #[test]
    fn presigned_url_requires_ttl() {
        let mut plan = canonical_presigned_url_plan();
        plan.ttl_seconds = 0;

        assert_eq!(
            plan.validate(),
            Err(StorageSidecarError::InvalidPresignedTtl)
        );
    }

    #[test]
    fn antivirus_requires_http_endpoint() {
        let plan = AntivirusPlan {
            scanner_endpoint: "clamd://localhost".to_string(),
            quarantine_bucket: "quarantine".to_string(),
            fail_closed: true,
        };

        assert_eq!(
            plan.validate(),
            Err(StorageSidecarError::InvalidScannerEndpoint)
        );
    }
}
