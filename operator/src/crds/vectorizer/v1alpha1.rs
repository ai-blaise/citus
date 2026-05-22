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
        self.scheduling.validate()
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
    InvalidDimension,
    MissingRequiredField(&'static str),
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
            Self::InvalidDimension => write!(formatter, "dimensions must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
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
