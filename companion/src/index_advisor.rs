// FEATURE: IA3

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IndexAdvisorPlan {
    pub workload_window: String,
    pub candidates: Vec<IndexCandidate>,
    pub min_improvement_percent: u32,
}

impl IndexAdvisorPlan {
    pub fn validate(&self) -> Result<(), IndexAdvisorError> {
        validate_required("workload_window", &self.workload_window)?;
        if self.candidates.is_empty() {
            return Err(IndexAdvisorError::MissingRequiredField("candidates"));
        }
        for candidate in &self.candidates {
            candidate.validate()?;
        }
        Ok(())
    }

    pub fn ranked_candidates(&self) -> Result<Vec<IndexCandidate>, IndexAdvisorError> {
        self.validate()?;
        let mut candidates = self
            .candidates
            .iter()
            .filter(|candidate| candidate.improvement_percent() >= self.min_improvement_percent)
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            right
                .improvement_percent()
                .cmp(&left.improvement_percent())
                .then_with(|| right.qual_count.cmp(&left.qual_count))
        });
        Ok(candidates)
    }

    pub fn to_sql_plan(&self) -> Result<IndexAdvisorSqlPlan, IndexAdvisorError> {
        let commands = self
            .ranked_candidates()?
            .into_iter()
            .map(|candidate| {
                format!(
                    "CREATE INDEX CONCURRENTLY IF NOT EXISTS {} ON {} USING {} ({});",
                    candidate.index_name,
                    candidate.table,
                    candidate.method.as_sql(),
                    candidate.columns.join(", ")
                )
            })
            .collect::<Vec<_>>();
        IndexAdvisorSqlPlan::new("IA3", commands)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IndexCandidate {
    pub table: String,
    pub index_name: String,
    pub columns: Vec<String>,
    pub method: IndexMethod,
    pub estimated_cost_before: u64,
    pub estimated_cost_after: u64,
    pub qual_count: u64,
}

impl IndexCandidate {
    pub fn validate(&self) -> Result<(), IndexAdvisorError> {
        validate_required("candidate.table", &self.table)?;
        validate_required("candidate.index_name", &self.index_name)?;
        validate_required_list("candidate.columns", &self.columns)?;
        if self.estimated_cost_before == 0 {
            return Err(IndexAdvisorError::InvalidCost("estimated_cost_before"));
        }
        if self.estimated_cost_after >= self.estimated_cost_before {
            return Err(IndexAdvisorError::NoEstimatedImprovement);
        }
        if self.qual_count == 0 {
            return Err(IndexAdvisorError::InvalidQualCount);
        }
        Ok(())
    }

    pub fn improvement_percent(&self) -> u32 {
        if self.estimated_cost_before == 0
            || self.estimated_cost_after >= self.estimated_cost_before
        {
            return 0;
        }
        // The dividend is bounded by `estimated_cost_before * 100` and divided by
        // `estimated_cost_before`, so the result is always in 0..=100 and fits in u32.
        #[allow(clippy::cast_possible_truncation)]
        {
            (((self.estimated_cost_before - self.estimated_cost_after) * 100)
                / self.estimated_cost_before) as u32
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum IndexMethod {
    Btree,
    Gin,
    Gist,
    Brin,
    Rum,
    Hnsw,
}

impl IndexMethod {
    fn as_sql(self) -> &'static str {
        match self {
            Self::Btree => "btree",
            Self::Gin => "gin",
            Self::Gist => "gist",
            Self::Brin => "brin",
            Self::Rum => "rum",
            Self::Hnsw => "hnsw",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IndexAdvisorSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl IndexAdvisorSqlPlan {
    fn new(feature_id: &'static str, commands: Vec<String>) -> Result<Self, IndexAdvisorError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(IndexAdvisorError::NoRankedCandidates);
        }
        Ok(Self {
            feature_id,
            commands,
        })
    }

    pub fn script(&self) -> String {
        self.commands.join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum IndexAdvisorError {
    InvalidCost(&'static str),
    InvalidQualCount,
    MissingRequiredField(&'static str),
    NoEstimatedImprovement,
    NoRankedCandidates,
}

impl fmt::Display for IndexAdvisorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCost(field) => write!(formatter, "{field} must be greater than zero"),
            Self::InvalidQualCount => write!(formatter, "qual_count must be greater than zero"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::NoEstimatedImprovement => write!(
                formatter,
                "estimated_cost_after must be lower than estimated_cost_before"
            ),
            Self::NoRankedCandidates => write!(formatter, "no candidates met advisor threshold"),
        }
    }
}

impl Error for IndexAdvisorError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), IndexAdvisorError> {
    if value.trim().is_empty() {
        return Err(IndexAdvisorError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(field: &'static str, values: &[String]) -> Result<(), IndexAdvisorError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(IndexAdvisorError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advisor_ranks_highest_improvement_first() {
        let plan = IndexAdvisorPlan {
            workload_window: "15 minutes".to_string(),
            min_improvement_percent: 5,
            candidates: vec![
                IndexCandidate {
                    table: "public.events".to_string(),
                    index_name: "events_tenant_created_idx".to_string(),
                    columns: vec!["tenant_id".to_string(), "created_at".to_string()],
                    method: IndexMethod::Btree,
                    estimated_cost_before: 1000,
                    estimated_cost_after: 800,
                    qual_count: 10,
                },
                IndexCandidate {
                    table: "public.events".to_string(),
                    index_name: "events_payload_gin_idx".to_string(),
                    columns: vec!["payload".to_string()],
                    method: IndexMethod::Gin,
                    estimated_cost_before: 1000,
                    estimated_cost_after: 500,
                    qual_count: 4,
                },
            ],
        };

        let ranked = plan.ranked_candidates().unwrap();
        assert_eq!(ranked[0].index_name, "events_payload_gin_idx");
    }

    #[test]
    fn advisor_renders_create_index_scripts() {
        let plan = IndexAdvisorPlan {
            workload_window: "1 hour".to_string(),
            min_improvement_percent: 10,
            candidates: vec![IndexCandidate {
                table: "public.events".to_string(),
                index_name: "events_tenant_created_idx".to_string(),
                columns: vec!["tenant_id".to_string(), "created_at".to_string()],
                method: IndexMethod::Btree,
                estimated_cost_before: 1000,
                estimated_cost_after: 700,
                qual_count: 12,
            }],
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "IA3");
        assert!(plan.script().contains("CREATE INDEX CONCURRENTLY"));
    }
}
