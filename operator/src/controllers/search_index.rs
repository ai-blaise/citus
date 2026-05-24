//! `SearchIndex` controller.

use super::{Context, ControllerError};
use crate::crds::search_index::{
    SearchColumnKind, SearchColumnSpec, SearchIndexSpec, SearchScorer,
};
use crate::reconcile::search_index::SearchIndexReconcilePlan;
use futures::StreamExt;
use kube::{
    api::Api,
    runtime::{controller::Action, watcher, Controller},
    CustomResource,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Kube-rs typed resource for the SearchIndex CRD.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "SearchIndex",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexCrSpec {
    pub table: String,
    #[serde(default)]
    pub columns: Vec<SearchColumnCr>,
    #[serde(default = "default_scorer")]
    pub scorer: String,
    pub analyzer: String,
    #[serde(default)]
    pub distributed: bool,
}

fn default_scorer() -> String {
    "bm25".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchColumnCr {
    pub name: String,
    pub kind: String,
}

impl SearchIndexCrSpec {
    pub fn to_authoritative(&self) -> Result<SearchIndexSpec, String> {
        Ok(SearchIndexSpec {
            table: self.table.clone(),
            columns: self
                .columns
                .iter()
                .map(|column| {
                    Ok(SearchColumnSpec {
                        name: column.name.clone(),
                        kind: parse_column_kind(&column.kind)?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
            scorer: parse_scorer(&self.scorer)?,
            analyzer: self.analyzer.clone(),
            distributed: self.distributed,
        })
    }
}

fn parse_column_kind(value: &str) -> Result<SearchColumnKind, String> {
    match normalize_token(value).as_str() {
        "text" => Ok(SearchColumnKind::Text),
        "vector" => Ok(SearchColumnKind::Vector),
        other => Err(format!("unsupported search column kind: {other}")),
    }
}

fn parse_scorer(value: &str) -> Result<SearchScorer, String> {
    match normalize_token(value).as_str() {
        "bm25" => Ok(SearchScorer::Bm25),
        "bm25vector" => Ok(SearchScorer::Bm25Vector),
        other => Err(format!("unsupported search scorer: {other}")),
    }
}

fn normalize_token(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|character| !matches!(character, '-' | '_' | '+' | ' '))
        .collect::<String>()
        .to_ascii_lowercase()
}

pub async fn run(ctx: Arc<Context>) -> Result<(), ControllerError> {
    let api: Api<SearchIndex> = Api::default_namespaced(ctx.client.clone());
    info!("SearchIndex controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((object, _action)) => debug!(?object, "reconciled SearchIndex"),
                Err(error) => error!(?error, "SearchIndex reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(
    search_index: Arc<SearchIndex>,
    ctx: Arc<Context>,
) -> Result<Action, ControllerError> {
    let resource_name = search_index
        .metadata
        .name
        .as_deref()
        .unwrap_or("search-index");
    let authoritative = search_index
        .spec
        .to_authoritative()
        .map_err(ControllerError::InvalidSpec)?;
    let plan = SearchIndexReconcilePlan::from_spec(resource_name, &authoritative)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    info!(
        search_index = ?search_index.metadata.name,
        distributed = plan.distributed,
        hybrid = plan.is_hybrid(),
        apply_steps = plan.apply_plan().steps.len(),
        "SearchIndex reconciled"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn error_policy(
    _search_index: Arc<SearchIndex>,
    error: &ControllerError,
    ctx: Arc<Context>,
) -> Action {
    error!(?error, "SearchIndex controller backoff");
    Action::requeue(ctx.default_requeue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_spec_round_trips_to_authoritative_spec() {
        let cr = SearchIndexCrSpec {
            table: "public.documents".to_string(),
            columns: vec![
                SearchColumnCr {
                    name: "body".to_string(),
                    kind: "text".to_string(),
                },
                SearchColumnCr {
                    name: "embedding".to_string(),
                    kind: "vector".to_string(),
                },
            ],
            scorer: "bm25+vector".to_string(),
            analyzer: "english".to_string(),
            distributed: true,
        };
        let spec = cr.to_authoritative().expect("valid search index");
        spec.validate().expect("spec valid");
        let plan =
            SearchIndexReconcilePlan::from_spec("documents-search", &spec).expect("plan valid");
        assert!(plan.is_hybrid());
        assert!(plan.distributed);
    }

    #[test]
    fn unsupported_scorer_is_rejected() {
        assert_eq!(
            parse_scorer("tf-idf"),
            Err("unsupported search scorer: tfidf".to_string())
        );
    }
}
