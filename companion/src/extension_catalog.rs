// FEATURE: A7
// FEATURE: A12
// FEATURE: C11
// FEATURE: C12
// FEATURE: C13
// FEATURE: EF6
// FEATURE: F2
// FEATURE: F5
// FEATURE: G1
// FEATURE: Geo1
// FEATURE: IA1
// FEATURE: IA2
// FEATURE: JS1
// FEATURE: L11
// FEATURE: M6
// FEATURE: M10
// FEATURE: M12
// FEATURE: MR7
// FEATURE: O7
// FEATURE: O8
// FEATURE: O9
// FEATURE: O11
// FEATURE: O12
// FEATURE: PM1
// FEATURE: PM2
// FEATURE: R6
// FEATURE: R11
// FEATURE: Search1
// FEATURE: Search4
// FEATURE: Search5
// FEATURE: Search6
// FEATURE: Sec3
// FEATURE: Sec4
// FEATURE: Sec10
// FEATURE: Sec11
// FEATURE: Sec14
// FEATURE: Sec15
// FEATURE: WF1

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub const EXTENSION_CATALOG_FEATURE_IDS: &[&str] = &[
    "A7", "A12", "C11", "C12", "C13", "EF6", "F2", "F5", "G1", "Geo1", "IA1", "IA2", "JS1", "L11",
    "M6", "M10", "M12", "MR7", "O7", "O8", "O9", "O11", "O12", "PM1", "PM2", "R6", "R11",
    "Search1", "Search4", "Search5", "Search6", "Sec3", "Sec4", "Sec10", "Sec11", "Sec14", "Sec15",
    "WF1",
];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ExtensionTier {
    Required,
    Optional,
    IntegrationTarget,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExtensionContract {
    pub name: &'static str,
    pub tier: ExtensionTier,
    pub feature_ids: &'static [&'static str],
    pub requires_preload: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExtensionCatalogSummary {
    pub required: usize,
    pub optional: usize,
    pub integration_targets: usize,
    pub preloaded: usize,
}

pub fn v2_extension_contracts() -> Vec<ExtensionContract> {
    vec![
        required("pgvector", &["A7"], false),
        optional("vchord", &["A12"], false),
        optional("pgl_ddl_deploy", &["C11", "M6"], false),
        required("pg_failover_slots", &["C12"], true),
        optional("pg_subscription_pg_failover", &["C13"], false),
        required("plrust", &["EF6"], true),
        required("plv8", &["EF6"], true),
        optional("oracle_fdw", &["F2"], false),
        optional("mysql_fdw", &["F2"], false),
        optional("mongo_fdw", &["F2"], false),
        optional("tds_fdw", &["F2"], false),
        optional("pgsql-http", &["F5"], false),
        optional("pg_net", &["F5"], false),
        required("age", &["G1"], true),
        required("postgis", &["Geo1"], false),
        optional("hypopg", &["IA1"], false),
        optional("pg_qualstats", &["IA2"], false),
        required("pg_jsonschema", &["JS1"], false),
        optional("pg_parquet", &["L11"], false),
        optional("pg_track_settings", &["M10"], false),
        required("pg_uuidv7", &["M12"], false),
        optional("pgactive", &["MR7"], true),
        optional("pg_wait_sampling", &["O7"], true),
        optional("pgsentinel", &["O7"], true),
        required("pgnodemx", &["O8"], false),
        optional("pg_stat_kcache", &["O9"], true),
        optional("pg_stat_monitor", &["O11"], true),
        optional("pg_show_plans", &["O12"], true),
        optional("pg_hint_plan", &["PM1"], true),
        optional("sr_plan", &["PM2"], true),
        optional("pgmq", &["R6"], false),
        optional("pgque", &["R6"], false),
        required("pg_warm", &["R11"], true),
        required("pg_search", &["Search1"], true),
        required("rum", &["Search4"], false),
        required("pg_trgm", &["Search5"], false),
        required("citext", &["Search6"], false),
        required("pgaudit", &["Sec3"], true),
        required("pgauditlogtofile", &["Sec3"], true),
        required("pgsodium", &["Sec4", "Sec15"], true),
        optional("pg_safeupdate", &["Sec10"], true),
        optional("anon", &["Sec11"], false),
        required("pgcrypto", &["Sec14"], false),
        optional("pg_walinspect", &["WF1"], false),
        integration_target("omnigres", &["F5"], false),
    ]
}

pub fn validate_extension_contracts(
    contracts: &[ExtensionContract],
) -> Result<ExtensionCatalogSummary, ExtensionCatalogError> {
    let mut names = BTreeSet::new();
    let mut features = BTreeSet::new();
    let mut summary = ExtensionCatalogSummary {
        required: 0,
        optional: 0,
        integration_targets: 0,
        preloaded: 0,
    };

    for contract in contracts {
        validate_required("extension.name", contract.name)?;
        if !names.insert(contract.name) {
            return Err(ExtensionCatalogError::DuplicateExtension(
                contract.name.to_string(),
            ));
        }
        if contract.feature_ids.is_empty() {
            return Err(ExtensionCatalogError::MissingRequiredField(
                "extension.feature_ids",
            ));
        }
        for feature_id in contract.feature_ids {
            validate_required("extension.feature_id", feature_id)?;
            features.insert(*feature_id);
        }
        match contract.tier {
            ExtensionTier::Required => summary.required += 1,
            ExtensionTier::Optional => summary.optional += 1,
            ExtensionTier::IntegrationTarget => summary.integration_targets += 1,
        }
        if contract.requires_preload {
            summary.preloaded += 1;
        }
    }

    for feature_id in EXTENSION_CATALOG_FEATURE_IDS {
        if !features.contains(feature_id) {
            return Err(ExtensionCatalogError::MissingFeature(feature_id));
        }
    }

    Ok(summary)
}

fn required(
    name: &'static str,
    feature_ids: &'static [&'static str],
    requires_preload: bool,
) -> ExtensionContract {
    ExtensionContract {
        name,
        tier: ExtensionTier::Required,
        feature_ids,
        requires_preload,
    }
}

fn optional(
    name: &'static str,
    feature_ids: &'static [&'static str],
    requires_preload: bool,
) -> ExtensionContract {
    ExtensionContract {
        name,
        tier: ExtensionTier::Optional,
        feature_ids,
        requires_preload,
    }
}

fn integration_target(
    name: &'static str,
    feature_ids: &'static [&'static str],
    requires_preload: bool,
) -> ExtensionContract {
    ExtensionContract {
        name,
        tier: ExtensionTier::IntegrationTarget,
        feature_ids,
        requires_preload,
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExtensionCatalogError {
    DuplicateExtension(String),
    MissingFeature(&'static str),
    MissingRequiredField(&'static str),
}

impl fmt::Display for ExtensionCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateExtension(name) => write!(formatter, "duplicate extension {name}"),
            Self::MissingFeature(feature_id) => {
                write!(formatter, "extension catalog missing feature {feature_id}")
            }
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for ExtensionCatalogError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), ExtensionCatalogError> {
    if value.trim().is_empty() {
        return Err(ExtensionCatalogError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_catalog_covers_required_plan_features() {
        let summary = validate_extension_contracts(&v2_extension_contracts()).unwrap();

        assert!(summary.required >= 10);
        assert!(summary.optional >= 20);
        assert!(summary.preloaded >= 10);
    }

    #[test]
    fn extension_catalog_rejects_duplicate_names() {
        let contracts = vec![
            required("pgvector", &["A7"], false),
            optional("pgvector", &["A12"], false),
        ];

        assert_eq!(
            validate_extension_contracts(&contracts),
            Err(ExtensionCatalogError::DuplicateExtension(
                "pgvector".to_string()
            ))
        );
    }
}
