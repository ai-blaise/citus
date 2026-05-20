// FEATURE: MR1
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
        validate_required("tablespace_name", &self.tablespace_name)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RegionSpecError {
    MissingRequiredField(&'static str),
}

impl fmt::Display for RegionSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
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

    fn minimal_spec() -> RegionSpec {
        RegionSpec {
            name: "us-east-1".to_string(),
            kubernetes_zone: "us-east-1a".to_string(),
            tablespace_name: "ts_us_east_1".to_string(),
            leader_pinned: false,
        }
    }
}
