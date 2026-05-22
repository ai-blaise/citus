//! `v1beta1` view of the `ScheduledRepack` CRD.
//!
//! Today the spec is byte-identical to v1alpha1, so this module is a thin
//! re-export with the version label attached. The conversion webhook for this
//! resource performs an identity round-trip; when a breaking field change
//! lands, copy the v1alpha1 implementation into this file, edit the diverging
//! types, and update the matching converter in `operator/src/conversion/scheduled_repack.rs`.

pub use super::v1alpha1::*;

/// API version label advertised by the v1beta1 surface.
pub const API_VERSION: &str = "v1beta1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_label_matches_module() {
        assert_eq!(API_VERSION, "v1beta1");
    }
}
