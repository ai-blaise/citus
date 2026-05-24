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
        validate_qualified_name("target", &self.target)?;
        validate_required("schedule", &self.schedule)?;
        validate_cron_schedule(&self.schedule)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RepackStrategy {
    PgRepack,
    RepackConcurrentlyPg19,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ScheduledRepackSpecError {
    InvalidCronSchedule,
    InvalidIdentifier(&'static str),
    MissingRequiredField(&'static str),
}

impl fmt::Display for ScheduledRepackSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCronSchedule => {
                write!(formatter, "schedule must be a five-field cron expression")
            }
            Self::InvalidIdentifier(field) => write!(
                formatter,
                "{field} must be a schema-qualified SQL identifier"
            ),
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

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ScheduledRepackSpecError> {
    validate_required(field, value)?;
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        Ok(())
    } else {
        Err(ScheduledRepackSpecError::InvalidIdentifier(field))
    }
}

fn validate_qualified_name(
    field: &'static str,
    value: &str,
) -> Result<(), ScheduledRepackSpecError> {
    validate_required(field, value)?;
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() == 2
        && parts
            .iter()
            .all(|part| validate_identifier(field, part).is_ok())
    {
        Ok(())
    } else {
        Err(ScheduledRepackSpecError::InvalidIdentifier(field))
    }
}

fn validate_cron_schedule(value: &str) -> Result<(), ScheduledRepackSpecError> {
    let fields = value.split_whitespace().collect::<Vec<_>>();
    if fields.len() == 5 && fields.iter().all(|field| !field.trim().is_empty()) {
        Ok(())
    } else {
        Err(ScheduledRepackSpecError::InvalidCronSchedule)
    }
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

    #[test]
    fn scheduled_repack_rejects_unqualified_target() {
        let spec = ScheduledRepackSpec {
            target: "orders".to_string(),
            schedule: "0 3 * * 0".to_string(),
            strategy: RepackStrategy::PgRepack,
        };

        assert_eq!(
            spec.validate(),
            Err(ScheduledRepackSpecError::InvalidIdentifier("target"))
        );
    }

    #[test]
    fn scheduled_repack_rejects_invalid_cron_shape() {
        let spec = ScheduledRepackSpec {
            target: "public.orders".to_string(),
            schedule: "hourly".to_string(),
            strategy: RepackStrategy::PgRepack,
        };

        assert_eq!(
            spec.validate(),
            Err(ScheduledRepackSpecError::InvalidCronSchedule)
        );
    }
}
