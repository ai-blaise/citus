//! ai-blaise Citus critical-path acceptance harness.

pub mod timescale_on_citus;

pub use timescale_on_citus::{
    AcceptanceGate, CohabitationPreloadConfig, TimescaleOnCitusAcceptance,
    TimescaleOnCitusAcceptanceError, TimescaleOnCitusPlan,
};
