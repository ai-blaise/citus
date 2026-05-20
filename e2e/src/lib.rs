//! ai-blaise Citus critical-path acceptance harness.

pub mod operator_catalog;
pub mod release_gates;
pub mod runtime_contracts;
pub mod timescale_on_citus;

pub use operator_catalog::{
    OperatorCatalogGate, V2OperatorCatalogAcceptance, V2OperatorCatalogAcceptanceError,
    V2OperatorCatalogPlan,
};
pub use release_gates::{
    V2ReleaseGateAcceptance, V2ReleaseGateError, V2ReleaseGateReport, UPSTREAM_RELEASE_REF,
};
pub use runtime_contracts::{
    RuntimeContractGate, V2RuntimeContractAcceptance, V2RuntimeContractAcceptanceError,
    V2RuntimeContractPlan,
};
pub use timescale_on_citus::{
    AcceptanceGate, CohabitationPreloadConfig, TimescaleOnCitusAcceptance,
    TimescaleOnCitusAcceptanceError, TimescaleOnCitusPlan,
};
