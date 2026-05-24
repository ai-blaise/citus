//! Runtime contract derived from the Vectorizer CRD.
//!
//! The operator renders these values from `FEATURE: A8` Vectorizer specs and
//! the sidecar consumes them through environment variables. When present, this
//! contract is fail-closed: queue rows and manual vectorization requests must
//! match the configured provider/model and every provider embedding must have
//! the declared destination dimension before it is written.

// FEATURE: A8

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorizerRuntimeContract {
    pub provider: String,
    pub model: String,
    pub dimensions: usize,
}

impl VectorizerRuntimeContract {
    pub fn new(provider: impl Into<String>, model: impl Into<String>, dimensions: usize) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            dimensions,
        }
    }

    pub fn validate(&self) -> Result<(), VectorizerContractError> {
        if self.provider.trim().is_empty() {
            return Err(VectorizerContractError::MissingField("provider"));
        }
        if self.model.trim().is_empty() {
            return Err(VectorizerContractError::MissingField("model"));
        }
        if self.dimensions == 0 {
            return Err(VectorizerContractError::InvalidDimensions);
        }
        if !is_safe_provider_name(&self.provider) {
            return Err(VectorizerContractError::InvalidProviderName(
                self.provider.clone(),
            ));
        }
        if self
            .model
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace())
        {
            return Err(VectorizerContractError::InvalidModelName(
                self.model.clone(),
            ));
        }
        Ok(())
    }

    pub fn assert_route(&self, provider: &str, model: &str) -> Result<(), VectorizerContractError> {
        self.validate()?;
        if provider == self.provider && model == self.model {
            return Ok(());
        }
        Err(VectorizerContractError::RouteMismatch {
            expected_provider: self.provider.clone(),
            expected_model: self.model.clone(),
            got_provider: provider.to_string(),
            got_model: model.to_string(),
        })
    }

    pub fn assert_embeddings(
        &self,
        embeddings: &[Vec<f32>],
    ) -> Result<(), VectorizerContractError> {
        self.validate()?;
        for (index, embedding) in embeddings.iter().enumerate() {
            if embedding.len() != self.dimensions {
                return Err(VectorizerContractError::EmbeddingDimensionMismatch {
                    expected: self.dimensions,
                    got: embedding.len(),
                    index,
                });
            }
        }
        Ok(())
    }

    pub fn description(&self) -> String {
        format!(
            "provider={} model={} dimensions={}",
            self.provider, self.model, self.dimensions
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum VectorizerContractError {
    EmbeddingDimensionMismatch {
        expected: usize,
        got: usize,
        index: usize,
    },
    InvalidDimensions,
    InvalidModelName(String),
    InvalidProviderName(String),
    MissingField(&'static str),
    RouteMismatch {
        expected_provider: String,
        expected_model: String,
        got_provider: String,
        got_model: String,
    },
}

impl fmt::Display for VectorizerContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmbeddingDimensionMismatch {
                expected,
                got,
                index,
            } => write!(
                formatter,
                "embedding dimension mismatch at index {index}: expected {expected}, got {got}"
            ),
            Self::InvalidDimensions => write!(formatter, "dimensions must be greater than zero"),
            Self::InvalidModelName(model) => {
                write!(formatter, "model contains whitespace or control characters: {model}")
            }
            Self::InvalidProviderName(provider) => write!(
                formatter,
                "provider must contain only ASCII letters, digits, '_', or '-': {provider}"
            ),
            Self::MissingField(field) => write!(formatter, "{field} must not be empty"),
            Self::RouteMismatch {
                expected_provider,
                expected_model,
                got_provider,
                got_model,
            } => write!(
                formatter,
                "vectorizer contract mismatch: expected provider={expected_provider} model={expected_model}, got provider={got_provider} model={got_model}"
            ),
        }
    }
}

impl Error for VectorizerContractError {}

fn is_safe_provider_name(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_contract_route_and_embedding_dimensions() {
        let contract = VectorizerRuntimeContract::new("mock", "embed-v1", 8);

        assert_eq!(contract.validate(), Ok(()));
        assert_eq!(contract.assert_route("mock", "embed-v1"), Ok(()));
        assert_eq!(
            contract.assert_embeddings(&[vec![0.0; 8], vec![1.0; 8]]),
            Ok(())
        );
    }

    #[test]
    fn rejects_route_mismatch() {
        let contract = VectorizerRuntimeContract::new("mock", "embed-v1", 8);

        assert!(matches!(
            contract.assert_route("mock", "embed-v2"),
            Err(VectorizerContractError::RouteMismatch { .. })
        ));
    }

    #[test]
    fn rejects_embedding_dimension_mismatch() {
        let contract = VectorizerRuntimeContract::new("mock", "embed-v1", 8);

        assert_eq!(
            contract.assert_embeddings(&[vec![0.0; 7]]),
            Err(VectorizerContractError::EmbeddingDimensionMismatch {
                expected: 8,
                got: 7,
                index: 0,
            })
        );
    }

    #[test]
    fn rejects_unsafe_provider_and_model_names() {
        assert!(matches!(
            VectorizerRuntimeContract::new("mock;drop", "embed-v1", 8).validate(),
            Err(VectorizerContractError::InvalidProviderName(_))
        ));
        assert!(matches!(
            VectorizerRuntimeContract::new("mock", "embed v1", 8).validate(),
            Err(VectorizerContractError::InvalidModelName(_))
        ));
    }
}
