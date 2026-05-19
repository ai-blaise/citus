// FEATURE: C4
// FEATURE: C5

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConflictPolicySpec {
    pub table: String,
    pub class: ConflictClass,
    pub resolution: ConflictResolution,
    pub custom_function: Option<String>,
}

impl ConflictPolicySpec {
    pub fn validate(&self) -> Result<(), ConflictPolicySpecError> {
        validate_required("table", &self.table)?;

        match self.resolution {
            ConflictResolution::CustomFunction => {
                validate_required_option("custom_function", &self.custom_function)
            }
            _ => validate_optional("custom_function", &self.custom_function),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConflictClass {
    InsertInsert,
    InsertUpdate,
    UpdateUpdate,
    UpdateDelete,
    DeleteUpdate,
    DeleteDelete,
    Constraint,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConflictResolution {
    LastWriteWins,
    OriginWins,
    TargetWins,
    Skip,
    Error,
    CustomFunction,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ConflictPolicySpecError {
    MissingRequiredField(&'static str),
}

impl fmt::Display for ConflictPolicySpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for ConflictPolicySpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), ConflictPolicySpecError> {
    if value.trim().is_empty() {
        return Err(ConflictPolicySpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(
    field: &'static str,
    value: &Option<String>,
) -> Result<(), ConflictPolicySpecError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(ConflictPolicySpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_option(
    field: &'static str,
    value: &Option<String>,
) -> Result<(), ConflictPolicySpecError> {
    match value {
        Some(value) => validate_required(field, value),
        None => Err(ConflictPolicySpecError::MissingRequiredField(field)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_conflict_policy_passes() {
        let spec = ConflictPolicySpec {
            table: "public.reference_accounts".to_string(),
            class: ConflictClass::UpdateUpdate,
            resolution: ConflictResolution::LastWriteWins,
            custom_function: None,
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn custom_resolution_requires_function() {
        let mut spec = minimal_spec();
        spec.resolution = ConflictResolution::CustomFunction;

        assert_eq!(
            spec.validate(),
            Err(ConflictPolicySpecError::MissingRequiredField(
                "custom_function"
            ))
        );
    }

    #[test]
    fn conflict_policy_rejects_empty_table() {
        let mut spec = minimal_spec();
        spec.table = " ".to_string();

        assert_eq!(
            spec.validate(),
            Err(ConflictPolicySpecError::MissingRequiredField("table"))
        );
    }

    fn minimal_spec() -> ConflictPolicySpec {
        ConflictPolicySpec {
            table: "public.reference_accounts".to_string(),
            class: ConflictClass::InsertInsert,
            resolution: ConflictResolution::Skip,
            custom_function: None,
        }
    }
}
