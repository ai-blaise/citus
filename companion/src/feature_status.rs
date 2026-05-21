use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FeatureStatus {
    pub feature_id: &'static str,
    pub feature_name: &'static str,
    pub status: &'static str,
}

pub const COMPANION_FEATURE_STATUSES: &[FeatureStatus] = &[
    FeatureStatus {
        feature_id: "TS1",
        feature_name: "distributed hypertable bridge",
        status: "sql-plan",
    },
    FeatureStatus {
        feature_id: "TS2",
        feature_name: "distributed compression policy",
        status: "sql-plan",
    },
    FeatureStatus {
        feature_id: "TS3",
        feature_name: "distributed continuous aggregates",
        status: "sql-plan",
    },
    FeatureStatus {
        feature_id: "TS4",
        feature_name: "distributed retention policy",
        status: "sql-plan",
    },
    FeatureStatus {
        feature_id: "TS5",
        feature_name: "time-range shard pruner",
        status: "sql-plan",
    },
    FeatureStatus {
        feature_id: "TS8",
        feature_name: "LSP hypertable invariants",
        status: "sql-plan",
    },
    FeatureStatus {
        feature_id: "TS9",
        feature_name: "doctor rules for cohabitation",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "TS12",
        feature_name: "distributed reorder policy",
        status: "sql-plan",
    },
    FeatureStatus {
        feature_id: "TS13",
        feature_name: "distributed time_bucket_gapfill",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "TS14",
        feature_name: "distributed metric toolkit aggregates",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "TS15",
        feature_name: "distributed approximate toolkit aggregates",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "TS16",
        feature_name: "distributed downsampler toolkit aggregates",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "TS17",
        feature_name: "distributed state toolkit aggregates",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "A1",
        feature_name: "pgai-compatible vectorizer DSL",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "Search2",
        feature_name: "distributed BM25 search index",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "Search3",
        feature_name: "hybrid BM25 and vector ranking",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "Search9",
        feature_name: "reranker UDF plan",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "G2",
        feature_name: "distributed graph bridge",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "G3",
        feature_name: "graph colocation policy",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "API4",
        feature_name: "GraphQL distributed graph metadata",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "JS2",
        feature_name: "distributed JSON Schema validation",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "M13",
        feature_name: "JSON Schema validation triggers",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "Geo2",
        feature_name: "geo-aware distribution",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "Geo3",
        feature_name: "geo shard pruning",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "T8",
        feature_name: "toolkit two-step aggregate pushdown",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "L9",
        feature_name: "worker partial aggregate pushdown",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "M7",
        feature_name: "pre-flight cohabit-extension check",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "PM3",
        feature_name: "plan freeze companion module",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "PM4",
        feature_name: "plan regression detection",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "IA3",
        feature_name: "companion index advisor",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "Sec5",
        feature_name: "immutable ledger",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "Sec6",
        feature_name: "ledger HMAC tamper evidence",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "M1",
        feature_name: "pgroll-style expand-contract migrations",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "M11",
        feature_name: "online column-type migration",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "WH2",
        feature_name: "companion webhook helpers",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "O1",
        feature_name: "query percentile views",
        status: "runtime-contract",
    },
    FeatureStatus {
        feature_id: "O2",
        feature_name: "local activity stats view",
        status: "runtime-contract",
    },
    FeatureStatus {
        feature_id: "O3",
        feature_name: "replication lag view",
        status: "runtime-contract",
    },
    FeatureStatus {
        feature_id: "R4",
        feature_name: "idle transaction detector",
        status: "runtime-contract",
    },
    FeatureStatus {
        feature_id: "Auth2",
        feature_name: "tenant-aware claims",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "Sec1",
        feature_name: "RLS helpers",
        status: "runtime-contract",
    },
    FeatureStatus {
        feature_id: "Sec2",
        feature_name: "JWT verification UDF",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "S6",
        feature_name: "placement generation helpers",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "S13",
        feature_name: "range routing helpers",
        status: "sql-runtime",
    },
    FeatureStatus {
        feature_id: "C10",
        feature_name: "online schema job state machine",
        status: "runtime-contract",
    },
    FeatureStatus {
        feature_id: "M2",
        feature_name: "gh-ost-style online DDL",
        status: "runtime-contract",
    },
    FeatureStatus {
        feature_id: "S14",
        feature_name: "tenant migration online",
        status: "runtime-contract",
    },
    FeatureStatus {
        feature_id: "TO3",
        feature_name: "tenant migration online",
        status: "runtime-contract",
    },
    FeatureStatus {
        feature_id: "TO4",
        feature_name: "tenant archive",
        status: "runtime-contract",
    },
    FeatureStatus {
        feature_id: "TO5",
        feature_name: "tenant region affinity",
        status: "runtime-contract",
    },
    FeatureStatus {
        feature_id: "D4",
        feature_name: "citus-lsp metadata views",
        status: "sql-plan",
    },
    FeatureStatus {
        feature_id: "M5",
        feature_name: "LSP migration quick-fix metadata",
        status: "sql-plan",
    },
    FeatureStatus {
        feature_id: "A7",
        feature_name: "pgvector cohabitation",
        status: "image-contract",
    },
    FeatureStatus {
        feature_id: "Search1",
        feature_name: "pg_search bundled",
        status: "image-contract",
    },
    FeatureStatus {
        feature_id: "G1",
        feature_name: "Apache AGE bundled",
        status: "image-contract",
    },
    FeatureStatus {
        feature_id: "JS1",
        feature_name: "pg_jsonschema bundled",
        status: "image-contract",
    },
    FeatureStatus {
        feature_id: "PM1",
        feature_name: "pg_hint_plan bundled",
        status: "image-contract",
    },
    FeatureStatus {
        feature_id: "IA1",
        feature_name: "hypopg bundled",
        status: "image-contract",
    },
    FeatureStatus {
        feature_id: "O7",
        feature_name: "wait-event sampling",
        status: "image-contract",
    },
    FeatureStatus {
        feature_id: "Sec3",
        feature_name: "pgaudit and file audit",
        status: "image-contract",
    },
    FeatureStatus {
        feature_id: "WF1",
        feature_name: "WAL inspection from SQL",
        status: "image-contract",
    },
    FeatureStatus {
        feature_id: "D7",
        feature_name: "Helm one-line install",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "D8",
        feature_name: "infrastructure deploy wrapper",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "D9",
        feature_name: "canary upgrade runbook",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "D10",
        feature_name: "release hardening runbook",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "D11",
        feature_name: "MCP developer workflow",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "MR9",
        feature_name: "region survival runbook",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "RT5",
        feature_name: "Phoenix-channel-compatible realtime client",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "S7",
        feature_name: "cross-region replication via pgactive",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "A9",
        feature_name: "secret binding via External Secrets",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "Sec7",
        feature_name: "External Secrets integration",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "Sec8",
        feature_name: "TLS everywhere",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "Sec9",
        feature_name: "SBOM and cosign attestation",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "Sec13",
        feature_name: "CIDR access control",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "T6",
        feature_name: "PG18 io_uring default",
        status: "ops-contract",
    },
    FeatureStatus {
        feature_id: "T7",
        feature_name: "pipelined client protocol in pool",
        status: "ops-contract",
    },
];

pub fn companion_feature_statuses() -> &'static [FeatureStatus] {
    COMPANION_FEATURE_STATUSES
}

pub fn validate_companion_feature_statuses() -> Result<(), FeatureStatusError> {
    let mut feature_ids = BTreeSet::new();
    for feature in COMPANION_FEATURE_STATUSES {
        if feature.feature_id.trim().is_empty() {
            return Err(FeatureStatusError::MissingField("feature_id"));
        }
        if feature.feature_name.trim().is_empty() {
            return Err(FeatureStatusError::MissingField("feature_name"));
        }
        if feature.status.trim().is_empty() {
            return Err(FeatureStatusError::MissingField("status"));
        }
        if feature.status == "planned" {
            return Err(FeatureStatusError::PlannedStatus(feature.feature_id));
        }
        if !feature_ids.insert(feature.feature_id) {
            return Err(FeatureStatusError::DuplicateFeature(feature.feature_id));
        }
    }

    for feature_id in CRITICAL_FEATURE_IDS {
        if !feature_ids.contains(feature_id) {
            return Err(FeatureStatusError::MissingCriticalFeature(feature_id));
        }
    }

    Ok(())
}

const CRITICAL_FEATURE_IDS: &[&str] = &[
    "TS1", "TS2", "TS3", "TS4", "TS5", "TS9", "M7", "A1", "Search3", "G2", "JS2", "Geo2", "O1",
    "Sec1", "S6", "C10", "TO3", "D7", "D11", "T7",
];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FeatureStatusError {
    DuplicateFeature(&'static str),
    MissingCriticalFeature(&'static str),
    MissingField(&'static str),
    PlannedStatus(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn companion_status_catalog_is_ready_to_expose() {
        validate_companion_feature_statuses().unwrap();
    }

    #[test]
    fn companion_status_catalog_has_no_planned_statuses() {
        assert!(COMPANION_FEATURE_STATUSES
            .iter()
            .all(|feature| feature.status != "planned"));
    }
}
