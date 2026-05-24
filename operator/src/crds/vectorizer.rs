// FEATURE: A8

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorizerSpec {
    pub source_table: String,
    pub source_column: String,
    pub embedding_provider: EmbeddingProvider,
    pub embedding_model: String,
    pub destination: VectorDestinationSpec,
    pub chunking: ChunkingSpec,
    pub scheduling: VectorizerSchedulingSpec,
    pub secret_ref: String,
}

impl VectorizerSpec {
    pub fn validate(&self) -> Result<(), VectorizerSpecError> {
        validate_required("source_table", &self.source_table)?;
        validate_required("source_column", &self.source_column)?;
        validate_required("embedding_model", &self.embedding_model)?;
        validate_required("secret_ref", &self.secret_ref)?;
        self.destination.validate()?;
        self.chunking.validate()?;
        self.scheduling.validate()?;
        self.validate_runtime_dimension_contract()
    }

    pub fn runtime_contract(&self) -> Result<VectorizerRuntimeContract, VectorizerSpecError> {
        validate_required("source_table", &self.source_table)?;
        validate_required("source_column", &self.source_column)?;
        validate_required("embedding_model", &self.embedding_model)?;
        validate_required("secret_ref", &self.secret_ref)?;
        self.destination.validate()?;
        self.chunking.validate()?;
        self.scheduling.validate()?;
        self.validate_runtime_dimension_contract()?;
        Ok(VectorizerRuntimeContract {
            provider: self.embedding_provider.runtime_name()?.to_string(),
            model: self.embedding_model.clone(),
            dimensions: self.destination.dimensions,
        })
    }

    fn validate_runtime_dimension_contract(&self) -> Result<(), VectorizerSpecError> {
        let provider = self.embedding_provider.runtime_name()?;
        let expected =
            known_model_dimensions(provider, &self.embedding_model).ok_or_else(|| {
                VectorizerSpecError::UnsupportedModelDimension {
                    provider: provider.to_string(),
                    model: self.embedding_model.clone(),
                }
            })?;
        if self.destination.dimensions != expected {
            return Err(VectorizerSpecError::DimensionModelMismatch {
                provider: provider.to_string(),
                model: self.embedding_model.clone(),
                expected,
                got: self.destination.dimensions,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EmbeddingProvider {
    OpenAi,
    AzureOpenAi,
    Anthropic,
    Cohere,
    Voyage,
    Ollama,
    Vertex,
}

impl EmbeddingProvider {
    fn runtime_name(self) -> Result<&'static str, VectorizerSpecError> {
        match self {
            Self::OpenAi => Ok("openai"),
            Self::AzureOpenAi => Ok("azure_openai"),
            Self::Cohere => Ok("cohere"),
            Self::Voyage => Ok("voyage"),
            Self::Ollama => Ok("ollama"),
            Self::Anthropic | Self::Vertex => {
                Err(VectorizerSpecError::UnsupportedRuntimeProvider(self))
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorizerRuntimeContract {
    pub provider: String,
    pub model: String,
    pub dimensions: u32,
}

impl VectorizerRuntimeContract {
    pub fn env_vars(&self) -> Vec<(String, String)> {
        vec![
            (
                "AI_BLAISE_VECTORIZER_CONTRACT_PROVIDER".to_string(),
                self.provider.clone(),
            ),
            (
                "AI_BLAISE_VECTORIZER_CONTRACT_MODEL".to_string(),
                self.model.clone(),
            ),
            (
                "AI_BLAISE_VECTORIZER_CONTRACT_DIMENSIONS".to_string(),
                self.dimensions.to_string(),
            ),
        ]
    }
}

fn known_model_dimensions(provider: &str, model: &str) -> Option<u32> {
    match (provider, model) {
        ("openai" | "azure_openai", "text-embedding-3-large") => Some(3_072),
        ("openai" | "azure_openai", "text-embedding-3-small" | "text-embedding-ada-002") => {
            Some(1_536)
        }
        ("voyage", "voyage-3-large" | "voyage-3.5" | "voyage-3.5-lite") => Some(1_024),
        ("cohere", "embed-english-v3.0" | "embed-multilingual-v3.0") => Some(1_024),
        ("ollama", "nomic-embed-text") => Some(768),
        _ => None,
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorDestinationSpec {
    pub table: String,
    pub column: String,
    pub dimensions: u32,
}

impl VectorDestinationSpec {
    fn validate(&self) -> Result<(), VectorizerSpecError> {
        validate_required("destination.table", &self.table)?;
        validate_required("destination.column", &self.column)?;
        if self.dimensions == 0 {
            return Err(VectorizerSpecError::InvalidDimension);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ChunkingSpec {
    pub strategy: ChunkingStrategy,
    pub max_tokens: u32,
    pub overlap_tokens: u32,
}

impl ChunkingSpec {
    fn validate(&self) -> Result<(), VectorizerSpecError> {
        if self.max_tokens == 0 {
            return Err(VectorizerSpecError::InvalidChunkSize);
        }
        if self.overlap_tokens >= self.max_tokens {
            return Err(VectorizerSpecError::InvalidChunkOverlap);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ChunkingStrategy {
    None,
    RecursiveText,
    Markdown,
    Sentence,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorizerSchedulingSpec {
    pub mode: VectorizerScheduleMode,
    pub interval: Option<String>,
    pub max_concurrency: u32,
}

impl VectorizerSchedulingSpec {
    fn validate(&self) -> Result<(), VectorizerSpecError> {
        if self.max_concurrency == 0 {
            return Err(VectorizerSpecError::InvalidConcurrency);
        }

        match self.mode {
            VectorizerScheduleMode::OnWrite => {
                validate_optional("scheduling.interval", &self.interval)
            }
            VectorizerScheduleMode::Interval | VectorizerScheduleMode::Cron => {
                validate_required_option("scheduling.interval", &self.interval)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VectorizerScheduleMode {
    OnWrite,
    Interval,
    Cron,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum VectorizerSpecError {
    InvalidChunkOverlap,
    InvalidChunkSize,
    InvalidConcurrency,
    DimensionModelMismatch {
        provider: String,
        model: String,
        expected: u32,
        got: u32,
    },
    InvalidDimension,
    MissingRequiredField(&'static str),
    UnsupportedModelDimension {
        provider: String,
        model: String,
    },
    UnsupportedRuntimeProvider(EmbeddingProvider),
}

impl fmt::Display for VectorizerSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidChunkOverlap => {
                write!(formatter, "overlap_tokens must be less than max_tokens")
            }
            Self::InvalidChunkSize => write!(formatter, "max_tokens must be greater than zero"),
            Self::InvalidConcurrency => {
                write!(formatter, "max_concurrency must be greater than zero")
            }
            Self::DimensionModelMismatch {
                provider,
                model,
                expected,
                got,
            } => write!(
                formatter,
                "{provider}/{model} requires {expected} dimensions, got {got}"
            ),
            Self::InvalidDimension => write!(formatter, "dimensions must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
            Self::UnsupportedModelDimension { provider, model } => write!(
                formatter,
                "unsupported vectorizer provider/model dimension contract: {provider}/{model}"
            ),
            Self::UnsupportedRuntimeProvider(provider) => {
                write!(
                    formatter,
                    "unsupported vectorizer runtime provider: {provider:?}"
                )
            }
        }
    }
}

impl Error for VectorizerSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), VectorizerSpecError> {
    if value.trim().is_empty() {
        return Err(VectorizerSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(
    field: &'static str,
    value: &Option<String>,
) -> Result<(), VectorizerSpecError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(VectorizerSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_option(
    field: &'static str,
    value: &Option<String>,
) -> Result<(), VectorizerSpecError> {
    match value {
        Some(value) => validate_required(field, value),
        None => Err(VectorizerSpecError::MissingRequiredField(field)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_vectorizer_spec_passes() {
        let spec = VectorizerSpec {
            source_table: "public.documents".to_string(),
            source_column: "body".to_string(),
            embedding_provider: EmbeddingProvider::OpenAi,
            embedding_model: "text-embedding-3-large".to_string(),
            destination: VectorDestinationSpec {
                table: "public.document_embeddings".to_string(),
                column: "embedding".to_string(),
                dimensions: 3_072,
            },
            chunking: ChunkingSpec {
                strategy: ChunkingStrategy::RecursiveText,
                max_tokens: 800,
                overlap_tokens: 80,
            },
            scheduling: VectorizerSchedulingSpec {
                mode: VectorizerScheduleMode::Interval,
                interval: Some("30 seconds".to_string()),
                max_concurrency: 8,
            },
            secret_ref: "openai-embeddings".to_string(),
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn runtime_contract_emits_sidecar_env() {
        let spec = minimal_spec();
        let contract = spec.runtime_contract().expect("runtime contract");

        assert_eq!(contract.provider, "voyage");
        assert_eq!(contract.model, "voyage-3-large");
        assert_eq!(contract.dimensions, 1_024);
        assert_eq!(
            contract.env_vars(),
            vec![
                (
                    "AI_BLAISE_VECTORIZER_CONTRACT_PROVIDER".to_string(),
                    "voyage".to_string(),
                ),
                (
                    "AI_BLAISE_VECTORIZER_CONTRACT_MODEL".to_string(),
                    "voyage-3-large".to_string(),
                ),
                (
                    "AI_BLAISE_VECTORIZER_CONTRACT_DIMENSIONS".to_string(),
                    "1024".to_string(),
                ),
            ]
        );
    }

    #[test]
    fn vectorizer_rejects_dimension_model_mismatch() {
        let mut spec = minimal_spec();
        spec.destination.dimensions = 768;

        assert_eq!(
            spec.validate(),
            Err(VectorizerSpecError::DimensionModelMismatch {
                provider: "voyage".to_string(),
                model: "voyage-3-large".to_string(),
                expected: 1_024,
                got: 768,
            })
        );
    }

    #[test]
    fn vectorizer_rejects_unsupported_runtime_provider() {
        let mut spec = minimal_spec();
        spec.embedding_provider = EmbeddingProvider::Anthropic;
        spec.embedding_model = "claude-embedding".to_string();

        assert_eq!(
            spec.validate(),
            Err(VectorizerSpecError::UnsupportedRuntimeProvider(
                EmbeddingProvider::Anthropic,
            ))
        );
    }

    #[test]
    fn vectorizer_rejects_invalid_overlap() {
        let mut spec = minimal_spec();
        spec.chunking.overlap_tokens = spec.chunking.max_tokens;

        assert_eq!(
            spec.validate(),
            Err(VectorizerSpecError::InvalidChunkOverlap)
        );
    }

    #[test]
    fn cron_schedule_requires_interval() {
        let mut spec = minimal_spec();
        spec.scheduling.mode = VectorizerScheduleMode::Cron;
        spec.scheduling.interval = None;

        assert_eq!(
            spec.validate(),
            Err(VectorizerSpecError::MissingRequiredField(
                "scheduling.interval"
            ))
        );
    }

    fn minimal_spec() -> VectorizerSpec {
        VectorizerSpec {
            source_table: "public.documents".to_string(),
            source_column: "body".to_string(),
            embedding_provider: EmbeddingProvider::Voyage,
            embedding_model: "voyage-3-large".to_string(),
            destination: VectorDestinationSpec {
                table: "public.document_embeddings".to_string(),
                column: "embedding".to_string(),
                dimensions: 1_024,
            },
            chunking: ChunkingSpec {
                strategy: ChunkingStrategy::Markdown,
                max_tokens: 512,
                overlap_tokens: 64,
            },
            scheduling: VectorizerSchedulingSpec {
                mode: VectorizerScheduleMode::OnWrite,
                interval: None,
                max_concurrency: 4,
            },
            secret_ref: "voyage-embeddings".to_string(),
        }
    }
}
