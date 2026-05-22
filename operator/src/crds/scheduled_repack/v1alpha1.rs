// FEATURE: R7

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScheduledRepackSpec {
    pub target: String,
    pub schedule: String,
    pub strategy: RepackStrategy,
}

impl ScheduledRepackSpec {
    pub fn validate(&self) -> Result<(), ScheduledRepackSpecError> {
        validate_required("target", &self.target)?;
        validate_required("schedule", &self.schedule)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RepackStrategy {
    PgRepack,
    RepackConcurrentlyPg19,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ScheduledRepackSpecError {
    MissingRequiredField(&'static str),
}

impl fmt::Display for ScheduledRepackSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for ScheduledRepackSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), ScheduledRepackSpecError> {
    if value.trim().is_empty() {
        return Err(ScheduledRepackSpecError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_scheduled_repack_passes() {
        let spec = ScheduledRepackSpec {
            target: "public.orders".to_string(),
            schedule: "0 3 * * 0".to_string(),
            strategy: RepackStrategy::PgRepack,
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn scheduled_repack_rejects_empty_target() {
        let spec = ScheduledRepackSpec {
            target: String::new(),
            schedule: "0 3 * * 0".to_string(),
            strategy: RepackStrategy::RepackConcurrentlyPg19,
        };

        assert_eq!(
            spec.validate(),
            Err(ScheduledRepackSpecError::MissingRequiredField("target"))
        );
    }
}
