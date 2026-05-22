// FEATURE: EF3

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FunctionSpec {
    pub name: String,
    pub runtime: FunctionRuntime,
    pub source: FunctionSource,
    pub triggers: Vec<FunctionTrigger>,
    pub env_secrets: Vec<String>,
}

impl FunctionSpec {
    pub fn validate(&self) -> Result<(), FunctionSpecError> {
        validate_required("name", &self.name)?;
        self.source.validate()?;
        if self.triggers.is_empty() {
            return Err(FunctionSpecError::MissingRequiredField("triggers"));
        }
        for trigger in &self.triggers {
            trigger.validate()?;
        }
        validate_optional_list("env_secrets", &self.env_secrets)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FunctionRuntime {
    Deno,
    Bun,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FunctionSource {
    GitRef {
        repository: String,
        reference: String,
        path: String,
    },
    Inline {
        code: String,
    },
}

impl FunctionSource {
    fn validate(&self) -> Result<(), FunctionSpecError> {
        match self {
            Self::GitRef {
                repository,
                reference,
                path,
            } => {
                validate_required("source.repository", repository)?;
                validate_required("source.reference", reference)?;
                validate_required("source.path", path)
            }
            Self::Inline { code } => validate_required("source.code", code),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FunctionTrigger {
    Http { path: String },
    Scheduled { schedule: String },
    Event { table: String, event: FunctionEvent },
}

impl FunctionTrigger {
    fn validate(&self) -> Result<(), FunctionSpecError> {
        match self {
            Self::Http { path } => validate_required("triggers.path", path),
            Self::Scheduled { schedule } => validate_required("triggers.schedule", schedule),
            Self::Event { table, .. } => validate_required("triggers.table", table),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FunctionEvent {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FunctionSpecError {
    MissingRequiredField(&'static str),
}

impl fmt::Display for FunctionSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for FunctionSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), FunctionSpecError> {
    if value.trim().is_empty() {
        return Err(FunctionSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional_list(field: &'static str, values: &[String]) -> Result<(), FunctionSpecError> {
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(FunctionSpecError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_inline_function_passes() {
        let spec = FunctionSpec {
            name: "order-created".to_string(),
            runtime: FunctionRuntime::Deno,
            source: FunctionSource::Inline {
                code: "export default async function handler(req) { return Response.json({ ok: true }); }"
                    .to_string(),
            },
            triggers: vec![FunctionTrigger::Http {
                path: "/orders".to_string(),
            }],
            env_secrets: vec!["orders-api-key".to_string()],
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn function_rejects_empty_trigger_list() {
        let mut spec = minimal_spec();
        spec.triggers = Vec::new();

        assert_eq!(
            spec.validate(),
            Err(FunctionSpecError::MissingRequiredField("triggers"))
        );
    }

    #[test]
    fn git_source_requires_reference() {
        let mut spec = minimal_spec();
        spec.source = FunctionSource::GitRef {
            repository: "https://github.com/ai-blaise/functions".to_string(),
            reference: String::new(),
            path: "orders/index.ts".to_string(),
        };

        assert_eq!(
            spec.validate(),
            Err(FunctionSpecError::MissingRequiredField("source.reference"))
        );
    }

    fn minimal_spec() -> FunctionSpec {
        FunctionSpec {
            name: "order-created".to_string(),
            runtime: FunctionRuntime::Bun,
            source: FunctionSource::Inline {
                code: "export default { fetch: () => new Response('ok') }".to_string(),
            },
            triggers: vec![FunctionTrigger::Scheduled {
                schedule: "*/5 * * * *".to_string(),
            }],
            env_secrets: Vec::new(),
        }
    }
}
