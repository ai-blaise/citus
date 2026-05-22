// FEATURE: Search2
// FEATURE: Search7

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchIndexSpec {
    pub table: String,
    pub columns: Vec<SearchColumnSpec>,
    pub scorer: SearchScorer,
    pub analyzer: String,
    pub distributed: bool,
}

impl SearchIndexSpec {
    pub fn validate(&self) -> Result<(), SearchIndexSpecError> {
        validate_required("table", &self.table)?;
        validate_required("analyzer", &self.analyzer)?;
        if self.columns.is_empty() {
            return Err(SearchIndexSpecError::MissingRequiredField("columns"));
        }
        for column in &self.columns {
            column.validate()?;
        }

        let has_text = self
            .columns
            .iter()
            .any(|column| column.kind == SearchColumnKind::Text);
        let has_vector = self
            .columns
            .iter()
            .any(|column| column.kind == SearchColumnKind::Vector);

        match self.scorer {
            SearchScorer::Bm25 if !has_text => Err(SearchIndexSpecError::MissingTextColumn),
            SearchScorer::Bm25Vector if !(has_text && has_vector) => {
                Err(SearchIndexSpecError::MissingHybridColumns)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchColumnSpec {
    pub name: String,
    pub kind: SearchColumnKind,
}

impl SearchColumnSpec {
    fn validate(&self) -> Result<(), SearchIndexSpecError> {
        validate_required("columns.name", &self.name)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SearchColumnKind {
    Text,
    Vector,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SearchScorer {
    Bm25,
    Bm25Vector,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SearchIndexSpecError {
    MissingHybridColumns,
    MissingTextColumn,
    MissingRequiredField(&'static str),
}

impl fmt::Display for SearchIndexSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHybridColumns => {
                write!(
                    formatter,
                    "bm25+vector scorer requires text and vector columns"
                )
            }
            Self::MissingTextColumn => write!(formatter, "bm25 scorer requires a text column"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for SearchIndexSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), SearchIndexSpecError> {
    if value.trim().is_empty() {
        return Err(SearchIndexSpecError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_hybrid_search_index_passes() {
        let spec = SearchIndexSpec {
            table: "public.documents".to_string(),
            columns: vec![
                SearchColumnSpec {
                    name: "body".to_string(),
                    kind: SearchColumnKind::Text,
                },
                SearchColumnSpec {
                    name: "embedding".to_string(),
                    kind: SearchColumnKind::Vector,
                },
            ],
            scorer: SearchScorer::Bm25Vector,
            analyzer: "english".to_string(),
            distributed: true,
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn hybrid_search_requires_vector_column() {
        let spec = SearchIndexSpec {
            table: "public.documents".to_string(),
            columns: vec![SearchColumnSpec {
                name: "body".to_string(),
                kind: SearchColumnKind::Text,
            }],
            scorer: SearchScorer::Bm25Vector,
            analyzer: "english".to_string(),
            distributed: true,
        };

        assert_eq!(
            spec.validate(),
            Err(SearchIndexSpecError::MissingHybridColumns)
        );
    }

    #[test]
    fn search_index_rejects_empty_column_name() {
        let mut spec = minimal_spec();
        spec.columns[0].name = String::new();

        assert_eq!(
            spec.validate(),
            Err(SearchIndexSpecError::MissingRequiredField("columns.name"))
        );
    }

    fn minimal_spec() -> SearchIndexSpec {
        SearchIndexSpec {
            table: "public.documents".to_string(),
            columns: vec![SearchColumnSpec {
                name: "body".to_string(),
                kind: SearchColumnKind::Text,
            }],
            scorer: SearchScorer::Bm25,
            analyzer: "english".to_string(),
            distributed: true,
        }
    }
}
