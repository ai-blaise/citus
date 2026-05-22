//! Multi-version module for the `ScheduledRepack` CRD.
//!
//! `v1alpha1` carries the served and stored spec. `v1beta1` is a forward
//! placeholder that today re-exports the same types; the conversion webhook
//! treats v1alpha1 <-> v1beta1 as an identity round-trip until the schema
//! actually diverges. When a breaking change lands, copy the v1alpha1 file
//! into v1beta1.rs, edit the v1beta1 types, then promote the conversion
//! handler to do the real translation.

pub mod v1alpha1;
pub mod v1beta1;

// Default re-export keeps the original `crate::crds::scheduled_repack::*` import
// surface working for every downstream consumer (reconcilers, main.rs, the
// lib.rs re-export wall) without churn.
pub use v1alpha1::*;

/// Resource kind name as it appears in the CustomResourceDefinition manifest.
pub const KIND: &str = "ScheduledRepack";

/// Plural form used in the resource's fully-qualified name.
pub const PLURAL: &str = "scheduledrepacks";

/// Singular form, matches `spec.names.singular` in the CRD YAML.
pub const SINGULAR: &str = "scheduledrepack";
