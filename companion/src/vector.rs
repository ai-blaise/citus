// FEATURE: A1
// FEATURE: A3
// FEATURE: A4
// FEATURE: A5
// FEATURE: A6

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorizerDefinition {
    pub name: String,
    pub source_table: String,
    pub source_pk: String,
    pub source_column: String,
    pub chunking: ChunkingPlan,
    pub embedding: EmbeddingPlan,
    pub destination: VectorDestinationPlan,
    pub schedule: VectorizerSchedule,
    pub tenant_budget_tokens: Option<u64>,
}

impl VectorizerDefinition {
    pub fn validate(&self) -> Result<(), VectorizerValidationError> {
        validate_required("name", &self.name)?;
        validate_required("source_table", &self.source_table)?;
        validate_required("source_pk", &self.source_pk)?;
        validate_required("source_column", &self.source_column)?;
        self.chunking.validate()?;
        self.embedding.validate()?;
        self.destination.validate()?;
        self.schedule.validate()?;
        if matches!(self.tenant_budget_tokens, Some(0)) {
            return Err(VectorizerValidationError::InvalidTokenBudget);
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<VectorizerSqlPlan, VectorizerValidationError> {
        self.validate()?;
        VectorizerSqlPlan::new(vec![
            "CREATE SCHEMA IF NOT EXISTS ai;".to_string(),
            format!(
                "SELECT ai.create_vectorizer({}, loading => ai.loading_table({}, {}, {}), chunking => ai.chunking_recursive_text({}, {}), embedding => ai.embedding_provider({}, {}, {}), destination => ai.destination_table({}, {}, {}), scheduling => ai.scheduling_interval({}));",
                sql_literal(&self.name),
                sql_literal(&self.source_table),
                sql_literal(&self.source_pk),
                sql_literal(&self.source_column),
                self.chunking.max_tokens,
                self.chunking.overlap_tokens,
                sql_literal(self.embedding.provider.as_str()),
                sql_literal(&self.embedding.model),
                sql_literal(&self.embedding.secret_ref),
                sql_literal(&self.destination.table),
                sql_literal(&self.destination.column),
                self.destination.dimensions,
                sql_literal(&self.schedule.interval)
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS ai.vectorizer_queue_{} (tenant_id text NOT NULL, source_pk text NOT NULL, source_text text NOT NULL, enqueued_at timestamptz NOT NULL DEFAULT now());",
                sanitize_identifier(&self.name)
            ),
            "CREATE TABLE IF NOT EXISTS ai.usage_log (tenant_id text NOT NULL, provider text NOT NULL, model text NOT NULL, tokens bigint NOT NULL, cost_micros bigint NOT NULL, recorded_at timestamptz NOT NULL DEFAULT now());".to_string(),
        ])
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ChunkingPlan {
    pub max_tokens: u32,
    pub overlap_tokens: u32,
}

impl ChunkingPlan {
    fn validate(&self) -> Result<(), VectorizerValidationError> {
        if self.max_tokens == 0 {
            return Err(VectorizerValidationError::InvalidChunkSize);
        }
        if self.overlap_tokens >= self.max_tokens {
            return Err(VectorizerValidationError::InvalidChunkOverlap);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EmbeddingPlan {
    pub provider: VectorProvider,
    pub model: String,
    pub secret_ref: String,
}

impl EmbeddingPlan {
    fn validate(&self) -> Result<(), VectorizerValidationError> {
        validate_required("embedding.model", &self.model)?;
        validate_required("embedding.secret_ref", &self.secret_ref)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VectorProvider {
    OpenAi,
    AzureOpenAi,
    Anthropic,
    Cohere,
    Voyage,
    Ollama,
    VertexAi,
}

impl VectorProvider {
    fn as_str(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::AzureOpenAi => "azure_openai",
            Self::Anthropic => "anthropic",
            Self::Cohere => "cohere",
            Self::Voyage => "voyage",
            Self::Ollama => "ollama",
            Self::VertexAi => "vertex_ai",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorDestinationPlan {
    pub table: String,
    pub column: String,
    pub dimensions: u32,
}

impl VectorDestinationPlan {
    fn validate(&self) -> Result<(), VectorizerValidationError> {
        validate_required("destination.table", &self.table)?;
        validate_required("destination.column", &self.column)?;
        if self.dimensions == 0 {
            return Err(VectorizerValidationError::InvalidDimensions);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorizerSchedule {
    pub interval: String,
    pub max_concurrency: u32,
}

impl VectorizerSchedule {
    fn validate(&self) -> Result<(), VectorizerValidationError> {
        validate_required("schedule.interval", &self.interval)?;
        if self.max_concurrency == 0 {
            return Err(VectorizerValidationError::InvalidConcurrency);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorizerPlan {
    pub definition: VectorizerDefinition,
    pub shard_local_queue: bool,
}

impl VectorizerPlan {
    pub fn validate(&self) -> Result<(), VectorizerValidationError> {
        self.definition.validate()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorizerSqlPlan {
    pub commands: Vec<String>,
}

impl VectorizerSqlPlan {
    fn new(commands: Vec<String>) -> Result<Self, VectorizerValidationError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(VectorizerValidationError::MissingRequiredField("commands"));
        }
        Ok(Self { commands })
    }

    pub fn script(&self) -> String {
        self.commands.join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum VectorizerValidationError {
    InvalidChunkOverlap,
    InvalidChunkSize,
    InvalidConcurrency,
    InvalidDimensions,
    InvalidTokenBudget,
    MissingRequiredField(&'static str),
}

impl fmt::Display for VectorizerValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidChunkOverlap => {
                write!(formatter, "overlap_tokens must be less than max_tokens")
            }
            Self::InvalidChunkSize => write!(formatter, "max_tokens must be greater than zero"),
            Self::InvalidConcurrency => {
                write!(formatter, "max_concurrency must be greater than zero")
            }
            Self::InvalidDimensions => write!(formatter, "dimensions must be greater than zero"),
            Self::InvalidTokenBudget => {
                write!(formatter, "tenant_budget_tokens must be greater than zero")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for VectorizerValidationError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), VectorizerValidationError> {
    if value.trim().is_empty() {
        return Err(VectorizerValidationError::MissingRequiredField(field));
    }
    Ok(())
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn sanitize_identifier(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vectorizer_definition_renders_pgai_compatible_plan() {
        let plan = valid_definition()
            .to_sql_plan()
            .expect("vectorizer SQL plan")
            .script();

        assert!(plan.contains("ai.create_vectorizer"));
        assert!(plan.contains("ai.loading_table"));
        assert!(plan.contains("ai.embedding_provider"));
        assert!(plan.contains("ai.vectorizer_queue_documents_body"));
        assert!(plan.contains("ai.usage_log"));
    }

    #[test]
    fn vectorizer_rejects_invalid_chunk_overlap() {
        let mut definition = valid_definition();
        definition.chunking.overlap_tokens = definition.chunking.max_tokens;

        assert_eq!(
            definition.validate(),
            Err(VectorizerValidationError::InvalidChunkOverlap)
        );
    }

    #[test]
    fn vectorizer_rejects_zero_budget() {
        let mut definition = valid_definition();
        definition.tenant_budget_tokens = Some(0);

        assert_eq!(
            definition.validate(),
            Err(VectorizerValidationError::InvalidTokenBudget)
        );
    }

    fn valid_definition() -> VectorizerDefinition {
        VectorizerDefinition {
            name: "documents_body".to_string(),
            source_table: "public.documents".to_string(),
            source_pk: "id".to_string(),
            source_column: "body".to_string(),
            chunking: ChunkingPlan {
                max_tokens: 800,
                overlap_tokens: 80,
            },
            embedding: EmbeddingPlan {
                provider: VectorProvider::OpenAi,
                model: "text-embedding-3-large".to_string(),
                secret_ref: "openai-embeddings".to_string(),
            },
            destination: VectorDestinationPlan {
                table: "public.document_embeddings".to_string(),
                column: "embedding".to_string(),
                dimensions: 3_072,
            },
            schedule: VectorizerSchedule {
                interval: "30 seconds".to_string(),
                max_concurrency: 8,
            },
            tenant_budget_tokens: Some(1_000_000),
        }
    }
}
