// FEATURE: MR1
// FEATURE: MR3
// FEATURE: MR4
// FEATURE: MR8

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionSpec {
    pub name: String,
    pub kubernetes_zone: String,
    pub tablespace_name: String,
    pub leader_pinned: bool,
}

impl RegionSpec {
    pub fn validate(&self) -> Result<(), RegionSpecError> {
        validate_required("name", &self.name)?;
        validate_required("kubernetes_zone", &self.kubernetes_zone)?;
        validate_required("tablespace_name", &self.tablespace_name)?;
        validate_region_name("name", &self.name)?;
        validate_zone_name("kubernetes_zone", &self.kubernetes_zone)?;
        validate_tablespace_name("tablespace_name", &self.tablespace_name)?;
        if !self.kubernetes_zone.starts_with(&self.name) {
            return Err(RegionSpecError::ZoneRegionMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RegionSpecError {
    InvalidRegionName(&'static str),
    InvalidTablespaceName(&'static str),
    InvalidZoneName(&'static str),
    MissingRequiredField(&'static str),
    ZoneRegionMismatch,
}

impl fmt::Display for RegionSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRegionName(field) => {
                write!(
                    formatter,
                    "{field} must be a lowercase region label such as us-east-1"
                )
            }
            Self::InvalidTablespaceName(field) => {
                write!(formatter, "{field} must be a safe PostgreSQL identifier")
            }
            Self::InvalidZoneName(field) => {
                write!(
                    formatter,
                    "{field} must be a lowercase zone label such as us-east-1a"
                )
            }
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
            Self::ZoneRegionMismatch => {
                write!(
                    formatter,
                    "kubernetes_zone must belong to the declared region name"
                )
            }
        }
    }
}

impl Error for RegionSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), RegionSpecError> {
    if value.trim().is_empty() {
        return Err(RegionSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_region_name(field: &'static str, value: &str) -> Result<(), RegionSpecError> {
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() < 3
        || parts.iter().any(|part| part.is_empty())
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || !value.as_bytes()[0].is_ascii_lowercase()
        || !value.as_bytes()[value.len() - 1].is_ascii_digit()
    {
        return Err(RegionSpecError::InvalidRegionName(field));
    }
    Ok(())
}

fn validate_zone_name(field: &'static str, value: &str) -> Result<(), RegionSpecError> {
    let bytes = value.as_bytes();
    if bytes.len() < 2
        || !bytes[0].is_ascii_lowercase()
        || !bytes[bytes.len() - 1].is_ascii_lowercase()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
    {
        return Err(RegionSpecError::InvalidZoneName(field));
    }
    Ok(())
}

fn validate_tablespace_name(field: &'static str, value: &str) -> Result<(), RegionSpecError> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 63
        || !(bytes[0].is_ascii_lowercase() || bytes[0] == b'_')
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
    {
        return Err(RegionSpecError::InvalidTablespaceName(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_region_passes() {
        let spec = RegionSpec {
            name: "us-east-1".to_string(),
            kubernetes_zone: "us-east-1a".to_string(),
            tablespace_name: "ts_us_east_1".to_string(),
            leader_pinned: true,
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn region_rejects_empty_zone() {
        let mut spec = minimal_spec();
        spec.kubernetes_zone = " ".to_string();

        assert_eq!(
            spec.validate(),
            Err(RegionSpecError::MissingRequiredField("kubernetes_zone"))
        );
    }

    #[test]
    fn region_rejects_unsafe_labels_and_tablespaces() {
        let mut spec = minimal_spec();
        spec.name = "us_east_1".to_string();
        assert_eq!(
            spec.validate(),
            Err(RegionSpecError::InvalidRegionName("name"))
        );

        let mut spec = minimal_spec();
        spec.kubernetes_zone = "us-east-1".to_string();
        assert_eq!(
            spec.validate(),
            Err(RegionSpecError::InvalidZoneName("kubernetes_zone"))
        );

        let mut spec = minimal_spec();
        spec.tablespace_name = "ts-us-east-1".to_string();
        assert_eq!(
            spec.validate(),
            Err(RegionSpecError::InvalidTablespaceName("tablespace_name"))
        );
    }

    #[test]
    fn region_rejects_zone_outside_region() {
        let mut spec = minimal_spec();
        spec.kubernetes_zone = "us-west-2a".to_string();

        assert_eq!(spec.validate(), Err(RegionSpecError::ZoneRegionMismatch));
    }

    fn minimal_spec() -> RegionSpec {
        RegionSpec {
            name: "us-east-1".to_string(),
            kubernetes_zone: "us-east-1a".to_string(),
            tablespace_name: "ts_us_east_1".to_string(),
            leader_pinned: false,
        }
    }
}
