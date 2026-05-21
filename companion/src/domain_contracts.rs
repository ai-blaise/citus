//! Deterministic companion domain-contract evidence.

use crate::{
    ChunkingPlan, CohabitPreflightPlan, DbDoctorPlan, DoctorRule, EmbeddingPlan,
    GeoDistributionPlan, GeoGrid, GeoPruningPlan, GraphDistributionPlan, HmacAlgorithm,
    HybridRankPlan, IndexAdvisorPlan, IndexCandidate, IndexMethod, JsonSchemaDistributedPlan,
    JwtVerificationPlan, LedgerChain, LedgerChainEntry, LedgerSealPlan, LedgerTransferPlan,
    LocalPlacementCheck, MigrationOperation, MigrationPlan, PlanFreezePlan, PlanPromotionPolicy,
    PlanRegressionPolicy, PlanRegressionSample, RerankerPlan, SearchColumnPlan,
    SearchIndexDistributedPlan, SessionClaims, ShardForValuePlan, ShardRoutingStrategy,
    TenantArchivePlan, TenantMovePlan, TenantRlsPolicyPlan, ToolkitAggregateKind,
    ToolkitDistributedPlan, ValidationTiming, VectorDestinationPlan, VectorProvider,
    VectorizerDefinition, VectorizerSchedule, WebhookEvent, WebhookHeader, WebhookRegistrationPlan,
};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DomainContractExecutionReport {
    pub feature_ids: Vec<&'static str>,
    pub sql_plan_count: usize,
    pub validation_count: usize,
    pub command_count: usize,
}

impl DomainContractExecutionReport {
    fn new() -> Self {
        Self {
            feature_ids: Vec::new(),
            sql_plan_count: 0,
            validation_count: 0,
            command_count: 0,
        }
    }

    fn add_validation(&mut self, feature_ids: &[&'static str]) {
        self.validation_count += 1;
        self.add_features(feature_ids);
    }

    fn add_sql_plan(&mut self, feature_ids: &[&'static str], command_count: usize) {
        self.sql_plan_count += 1;
        self.command_count += command_count;
        self.add_features(feature_ids);
    }

    fn add_features(&mut self, feature_ids: &[&'static str]) {
        self.feature_ids.extend(feature_ids.iter().copied());
    }

    fn deduplicate_features(&mut self) {
        self.feature_ids = self
            .feature_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DomainContractError {
    detail: String,
}

impl DomainContractError {
    fn from_error(error: impl fmt::Display) -> Self {
        Self {
            detail: error.to_string(),
        }
    }
}

impl fmt::Display for DomainContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "domain contract evidence failed: {}",
            self.detail
        )
    }
}

impl Error for DomainContractError {}

pub fn canonical_domain_contracts_report(
) -> Result<DomainContractExecutionReport, DomainContractError> {
    let mut report = DomainContractExecutionReport::new();

    record_vector_contract(&mut report)?;
    record_auth_contracts(&mut report)?;
    record_graph_contracts(&mut report)?;
    record_geo_contracts(&mut report)?;
    record_index_advisor_contract(&mut report)?;
    record_jsonschema_contract(&mut report)?;
    record_migration_contracts(&mut report)?;
    record_schema_job_contract(&mut report)?;
    record_db_doctor_contracts(&mut report)?;
    record_plan_freeze_contracts(&mut report)?;
    record_router_assist_contracts(&mut report)?;
    record_search_contracts(&mut report)?;
    record_ledger_contracts(&mut report)?;
    record_toolkit_contracts(&mut report)?;
    record_tenant_contracts(&mut report)?;
    record_webhook_contract(&mut report)?;

    report.deduplicate_features();
    Ok(report)
}

fn record_vector_contract(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let plan = VectorizerDefinition {
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
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["A1"], plan.commands.len());
    Ok(())
}

fn record_auth_contracts(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let claims = SessionClaims {
        uid: "user-123".to_string(),
        role: "authenticated".to_string(),
        tenant_id: "tenant-a".to_string(),
        jwt_id: Some("jti-123".to_string()),
    };
    claims.validate().map_err(DomainContractError::from_error)?;
    report.add_validation(&["Auth2"]);

    let jwt = JwtVerificationPlan {
        issuer: "https://auth.example.com".to_string(),
        audience: "citus".to_string(),
        jwks_secret_ref: "secret://jwt/jwks".to_string(),
    };
    jwt.validate().map_err(DomainContractError::from_error)?;
    report.add_validation(&["Sec2"]);

    let rls = TenantRlsPolicyPlan {
        table: "tenant_a.orders".to_string(),
        tenant_column: "tenant_id".to_string(),
        claims,
    };
    rls.validate().map_err(DomainContractError::from_error)?;
    report.add_validation(&["Sec1"]);
    Ok(())
}

fn record_graph_contracts(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let plan = GraphDistributionPlan {
        graph_name: "tenant_graph".to_string(),
        vertex_table: "public.vertices".to_string(),
        edge_table: "public.edges".to_string(),
        vertex_key: "tenant_id".to_string(),
        edge_source_key: "source_id".to_string(),
        edge_target_key: "target_id".to_string(),
        colocation_group: "tenant".to_string(),
    }
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["API4", "G2", "G3"], plan.commands.len());
    Ok(())
}

fn record_geo_contracts(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let grid = GeoGrid {
        precision: 7,
        srid: 4326,
    };
    let distribution = GeoDistributionPlan {
        table: "public.places".to_string(),
        geometry_column: "geom".to_string(),
        distribution_column: "geo_hash".to_string(),
        grid: grid.clone(),
        shard_count: 32,
    }
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["Geo2"], distribution.commands.len());

    let pruning = GeoPruningPlan {
        table: "public.places".to_string(),
        geometry_column: "geom".to_string(),
        grid,
    }
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["Geo3"], pruning.commands.len());
    Ok(())
}

fn record_index_advisor_contract(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
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
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["IA3"], plan.commands.len());
    Ok(())
}

fn record_jsonschema_contract(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let plan = JsonSchemaDistributedPlan {
        table: "tenant_a.events".to_string(),
        json_column: "payload".to_string(),
        schema_name: "event_schema".to_string(),
        schema_document: r#"{"type":"object"}"#.to_string(),
        timing: ValidationTiming::BeforeInsertOrUpdate,
    }
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["JS2", "M13"], plan.commands.len());
    Ok(())
}

fn record_migration_contracts(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let plan = MigrationPlan {
        name: "orders-total-bigint".to_string(),
        table: "public.orders".to_string(),
        operations: vec![
            MigrationOperation::AddColumn {
                column: "total_cents_v2".to_string(),
                sql_type: "bigint".to_string(),
                default_expression: None,
            },
            MigrationOperation::AlterColumnType {
                column: "total_cents".to_string(),
                from_type: "integer".to_string(),
                to_type: "bigint".to_string(),
                cast_expression: "total_cents::bigint".to_string(),
            },
        ],
        lock_timeout_ms: 500,
        backfill_batch_size: 1000,
    }
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["M1", "M11"], plan.commands.len());
    Ok(())
}

fn record_schema_job_contract(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let plan = crate::SchemaJobPlan {
        name: "users-add-display-name".to_string(),
        table: "public.users".to_string(),
        state: crate::SchemaJobState::DeleteOnly,
        operations: vec![crate::SchemaJobOperation::AddColumn {
            column: "display_name".to_string(),
            sql_type: "text".to_string(),
        }],
        lease_seconds: 30,
    };
    plan.validate().map_err(DomainContractError::from_error)?;
    if !plan.can_advance_to(crate::SchemaJobState::WriteOnly) {
        return Err(DomainContractError::from_error(
            "schema job canonical state cannot advance",
        ));
    }
    report.add_validation(&["M2"]);
    Ok(())
}

fn record_db_doctor_contracts(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let doctor = DbDoctorPlan {
        schemas: vec!["public".to_string(), "tenant_a".to_string()],
        rules: vec![
            DoctorRule::CohabitExtensions,
            DoctorRule::NonColocatedJoin,
            DoctorRule::MissingDistributionColumn,
            DoctorRule::HypertableBridge,
            DoctorRule::ChunkIntervalOutOfBand,
        ],
    }
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["TS9"], doctor.commands.len());

    let preflight = CohabitPreflightPlan {
        shared_preload_libraries: vec!["citus".to_string(), "timescaledb".to_string()],
        required_extensions: vec!["timescaledb".to_string()],
    }
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["M7"], preflight.commands.len());
    Ok(())
}

fn record_plan_freeze_contracts(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let policy = PlanRegressionPolicy {
        max_latency_regression_percent: 10,
        max_cost_regression_percent: 20,
    };
    let plan = PlanFreezePlan {
        query_hash: "abc123".to_string(),
        plan_xml: "<Plan />".to_string(),
        hint_set_name: "stable_orders_plan".to_string(),
        promotion: PlanPromotionPolicy {
            min_executions: 100,
            stable_days: 7,
        },
        regression: policy.clone(),
    }
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["PM3"], plan.commands.len());

    let sample = PlanRegressionSample {
        query_hash: "abc123".to_string(),
        baseline_p95_ms: 100,
        candidate_p95_ms: 112,
        baseline_cost: 1000,
        candidate_cost: 1000,
    };
    if !sample
        .violates(&policy)
        .map_err(DomainContractError::from_error)?
    {
        return Err(DomainContractError::from_error(
            "plan regression canonical sample did not violate policy",
        ));
    }
    report.add_validation(&["PM4"]);
    Ok(())
}

fn record_router_assist_contracts(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let range = ShardForValuePlan {
        table: "public.events".to_string(),
        distribution_column: "created_at".to_string(),
        value_hash: 42,
        shard_count: 16,
        strategy: ShardRoutingStrategy::Range {
            lower_bound: "2026-01-01".to_string(),
            upper_bound: "2026-02-01".to_string(),
        },
    };
    if range
        .target_shard_index()
        .map_err(DomainContractError::from_error)?
        != 10
    {
        return Err(DomainContractError::from_error(
            "range routing canonical shard mismatch",
        ));
    }
    report.add_validation(&["S13"]);

    let placement = LocalPlacementCheck {
        shard_id: 10_240,
        worker_name: "worker-1".to_string(),
    };
    placement
        .validate()
        .map_err(DomainContractError::from_error)?;
    report.add_validation(&["S6"]);
    Ok(())
}

fn record_search_contracts(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let index = SearchIndexDistributedPlan::new(
        "public.docs",
        "docs_bm25",
        vec![
            SearchColumnPlan::text("title"),
            SearchColumnPlan::text("body"),
            SearchColumnPlan::vector("embedding"),
        ],
        "tenant_id",
    )
    .map_err(DomainContractError::from_error)?
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["Search2"], index.commands.len());

    let hybrid = HybridRankPlan {
        table: "public.docs".to_string(),
        text_query: "database".to_string(),
        vector_column: "embedding".to_string(),
        vector_parameter: "$1".to_string(),
        text_weight: 1,
        vector_weight: 1,
        limit: 20,
    }
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["Search3"], hybrid.commands.len());

    let reranker = RerankerPlan {
        input_view: "companion.docs_hybrid".to_string(),
        provider: "openai".to_string(),
        model: "rerank-small".to_string(),
        limit: 20,
    }
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["Search9"], reranker.commands.len());
    Ok(())
}

fn record_ledger_contracts(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let transfer = LedgerTransferPlan {
        transfer_id: "tr_001".to_string(),
        debit_account: "cash".to_string(),
        credit_account: "revenue".to_string(),
        amount_cents: 5000,
        currency: "USD".to_string(),
        previous_hash: "genesis".to_string(),
    }
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["Sec5"], transfer.commands.len());

    let seal = LedgerSealPlan {
        transfer_id: "tr_001".to_string(),
        secret_ref: "vault://ledger/hmac".to_string(),
        algorithm: HmacAlgorithm::Sha256,
    }
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["Sec6"], seal.commands.len());

    let chain = LedgerChain {
        genesis_hash: "genesis".to_string(),
        entries: vec![LedgerChainEntry {
            entry_hash: "hash-1".to_string(),
            previous_hash: "genesis".to_string(),
        }],
    };
    chain.validate().map_err(DomainContractError::from_error)?;
    report.add_validation(&["Sec5"]);
    Ok(())
}

fn record_toolkit_contracts(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    for (features, aggregate, time_column, bucket_width) in [
        (
            &["L9", "TS13"][..],
            ToolkitAggregateKind::TimeBucketGapfill,
            Some("created_at"),
            Some("1 minute"),
        ),
        (
            &["L9", "TS14"][..],
            ToolkitAggregateKind::CounterAgg,
            None,
            None,
        ),
        (
            &["L9", "TS15"][..],
            ToolkitAggregateKind::PercentileAgg,
            None,
            None,
        ),
        (
            &["L9", "TS16"][..],
            ToolkitAggregateKind::AsapSmooth,
            Some("created_at"),
            None,
        ),
        (
            &["L9", "TS17"][..],
            ToolkitAggregateKind::CandlestickAgg,
            Some("created_at"),
            None,
        ),
        (
            &["L9", "T8"][..],
            ToolkitAggregateKind::HyperLogLog,
            None,
            None,
        ),
    ] {
        let mut plan = ToolkitDistributedPlan::new(
            "metrics.cpu",
            format!("companion.worker_{}", aggregate_name(aggregate)),
            format!("companion.{}", aggregate_name(aggregate)),
            "tenant_id",
            "usage_percent",
            aggregate,
        )
        .map_err(DomainContractError::from_error)?;
        if let Some(time_column) = time_column {
            plan = plan.with_time_column(time_column);
        }
        if let Some(bucket_width) = bucket_width {
            plan = plan.with_bucket_width(bucket_width);
        }
        let sql = plan
            .to_sql_plan()
            .map_err(DomainContractError::from_error)?;
        report.add_sql_plan(features, sql.commands.len());
    }
    Ok(())
}

fn record_tenant_contracts(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let move_plan = TenantMovePlan {
        tenant_name: "tenant-a".to_string(),
        source_worker: "worker-1".to_string(),
        target_worker: "worker-2".to_string(),
        region_affinity: Some("us-east-1".to_string()),
    };
    move_plan
        .validate()
        .map_err(DomainContractError::from_error)?;
    report.add_validation(&["S14", "TO3"]);

    let archive = TenantArchivePlan {
        tenant_name: "tenant-a".to_string(),
        destination_uri: "s3://archives/tenant-a".to_string(),
        retention_days: 30,
    };
    archive
        .validate()
        .map_err(DomainContractError::from_error)?;
    report.add_validation(&["TO4"]);
    Ok(())
}

fn record_webhook_contract(
    report: &mut DomainContractExecutionReport,
) -> Result<(), DomainContractError> {
    let plan = WebhookRegistrationPlan {
        name: "orders-webhook".to_string(),
        table: "public.orders".to_string(),
        events: vec![WebhookEvent::Insert, WebhookEvent::Update],
        url: "https://hooks.example.test/orders".to_string(),
        headers: vec![WebhookHeader {
            name: "Authorization".to_string(),
            value_secret_ref: "secret://webhooks/orders".to_string(),
        }],
        queue_name: "companion.webhook_queue".to_string(),
        max_retries: 8,
    }
    .to_sql_plan()
    .map_err(DomainContractError::from_error)?;
    report.add_sql_plan(&["WH2"], plan.commands.len());
    Ok(())
}

fn aggregate_name(aggregate: ToolkitAggregateKind) -> &'static str {
    match aggregate {
        ToolkitAggregateKind::TimeBucketGapfill => "gapfill",
        ToolkitAggregateKind::CounterAgg => "counter",
        ToolkitAggregateKind::GaugeAgg => "gauge",
        ToolkitAggregateKind::HeartbeatAgg => "heartbeat",
        ToolkitAggregateKind::PercentileAgg => "percentile",
        ToolkitAggregateKind::FrequencyAgg => "frequency",
        ToolkitAggregateKind::HyperLogLog => "hll",
        ToolkitAggregateKind::TDigest => "tdigest",
        ToolkitAggregateKind::AsapSmooth => "asap",
        ToolkitAggregateKind::Lttb => "lttb",
        ToolkitAggregateKind::CandlestickAgg => "candlestick",
        ToolkitAggregateKind::StateVec => "state",
        ToolkitAggregateKind::RangeAgg => "range",
        ToolkitAggregateKind::TimeWeightedAverage => "time_weight",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_contract_report_covers_all_expected_feature_ids() {
        let report = canonical_domain_contracts_report().expect("domain report");

        assert_eq!(report.sql_plan_count, 22);
        assert_eq!(report.validation_count, 10);
        assert_eq!(report.command_count, 44);
        for feature_id in [
            "A1", "API4", "Auth2", "G2", "G3", "Geo2", "Geo3", "IA3", "JS2", "L9", "M1", "M11",
            "M13", "M2", "M7", "PM3", "PM4", "S13", "S14", "S6", "Search2", "Search3", "Search9",
            "Sec1", "Sec2", "Sec5", "Sec6", "T8", "TO3", "TO4", "TS13", "TS14", "TS15", "TS16",
            "TS17", "TS9", "WH2",
        ] {
            assert!(
                report.feature_ids.contains(&feature_id),
                "missing {feature_id}"
            );
        }
    }
}
