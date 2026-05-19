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
    InvalidObjectSize,
    InvalidPresignedTtl,
    InvalidScannerEndpoint,
    MissingRequiredField(&'static str),
    SharedContract(String),
}

impl fmt::Display for StorageSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_sidecar_plan_validates_bucket_and_antivirus() {
        assert_eq!(valid_plan().validate(), Ok(()));
    }

    #[test]
    fn metadata_record_requires_positive_size() {
        let mut metadata = valid_metadata();
        metadata.size_bytes = 0;

        assert_eq!(
            metadata.validate(),
            Err(StorageSidecarError::InvalidObjectSize)
        );
    }

    #[test]
    fn presigned_url_requires_ttl() {
        let plan = PresignedUrlPlan {
            bucket: "tenant-files".to_string(),
            object_key: "orders/1.pdf".to_string(),
            tenant_id: "tenant-a".to_string(),
            method: PresignedMethod::Put,
            ttl_seconds: 0,
        };

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

    fn valid_plan() -> StorageSidecarPlan {
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

    fn valid_metadata() -> ObjectMetadataRecord {
        ObjectMetadataRecord {
            bucket: "tenant-files".to_string(),
            object_key: "orders/1.pdf".to_string(),
            tenant_id: "tenant-a".to_string(),
            content_type: "application/pdf".to_string(),
            size_bytes: 42,
        }
    }
}
