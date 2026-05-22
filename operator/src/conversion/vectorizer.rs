//! Conversion handler for the `Vectorizer` CRD.
//!
//! v1alpha1 and v1beta1 carry the same spec today, so the handler is an
//! identity copy. When the v1beta1 schema diverges, replace the body of
//! `identity` (or split it into `v1alpha1_to_v1beta1` and
//! `v1beta1_to_v1alpha1` halves) with the typed field translation; the
//! round-trip test below will start failing on lossy mappings and must be
//! updated alongside.

use crate::crds::vectorizer::v1alpha1::VectorizerSpec as V1Alpha1VectorizerSpec;
use crate::crds::vectorizer::v1beta1::VectorizerSpec as V1Beta1VectorizerSpec;

/// Identity conversion between v1alpha1 and v1beta1 of the `Vectorizer` spec.
/// Today the two versions resolve to the same Rust type via the v1beta1
/// re-export, so this is a clone; the function signature is the seam where
/// future divergence will be expressed.
pub fn identity(spec: &V1Alpha1VectorizerSpec) -> V1Beta1VectorizerSpec {
    spec.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::canonical_vectorizer_spec;

    #[test]
    fn round_trip_is_identity() {
        let original = canonical_vectorizer_spec();
        let beta = identity(&original);
        let back = identity(&beta);
        assert_eq!(back, original);
    }
}
