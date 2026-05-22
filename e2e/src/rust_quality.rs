//! Workspace Rust quality contracts.
//!
//! Validates the workspace-root tooling files that pin the Rust toolchain,
//! enable strict clippy lints, gate dependency security advisories, and gate
//! transitive license compatibility. Real enforcement happens in
//! `.github/workflows/ci-rust-quality.yml` (cargo fmt --check, cargo clippy
//! --workspace --all-targets -- -D warnings, cargo audit, cargo deny check)
//! and locally via `ci/ai-blaise/license-check.sh` which now invokes
//! `cargo deny check`. This module exposes a deterministic acceptance shape
//! so the canonical-evidence pipeline can reference the same gate.

// FEATURE: RQ1
// FEATURE: RQ2
// FEATURE: RQ3
// FEATURE: RQ4

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

/// Expected tooling files at the workspace root.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RustQualityToolingPaths {
    pub rust_toolchain: PathBuf,
    pub clippy_config: PathBuf,
    pub rustfmt_config: PathBuf,
    pub cargo_config: PathBuf,
    pub deny_config: PathBuf,
    pub audit_config: PathBuf,
    pub ci_workflow: PathBuf,
}

impl RustQualityToolingPaths {
    /// Canonical paths anchored at the workspace root.
    #[must_use]
    pub fn canonical(workspace_root: &Path) -> Self {
        Self {
            rust_toolchain: workspace_root.join("rust-toolchain.toml"),
            clippy_config: workspace_root.join("clippy.toml"),
            rustfmt_config: workspace_root.join("rustfmt.toml"),
            cargo_config: workspace_root.join(".cargo/config.toml"),
            deny_config: workspace_root.join("deny.toml"),
            audit_config: workspace_root.join("audit.toml"),
            ci_workflow: workspace_root.join(".github/workflows/ci-rust-quality.yml"),
        }
    }
}

/// Expected toolchain channel pinned by `rust-toolchain.toml`.
pub const RUST_TOOLCHAIN_CHANNEL: &str = "1.95.0";

/// Required clippy components pulled in by the toolchain pin.
pub const REQUIRED_TOOLCHAIN_COMPONENTS: [&str; 4] =
    ["rustfmt", "clippy", "rust-src", "rust-analyzer"];

/// Clippy MSRV that mirrors `RUST_TOOLCHAIN_CHANNEL`.
pub const CLIPPY_MSRV: &str = "1.95.0";

/// Cognitive-complexity ceiling enforced by `clippy.toml`.
pub const CLIPPY_COGNITIVE_COMPLEXITY_THRESHOLD: u32 = 25;

/// Type-complexity ceiling enforced by `clippy.toml`.
pub const CLIPPY_TYPE_COMPLEXITY_THRESHOLD: u32 = 250;

/// Maximum line width enforced by `rustfmt.toml`.
pub const RUSTFMT_MAX_WIDTH: u32 = 100;

/// Tab spacing enforced by `rustfmt.toml`.
pub const RUSTFMT_TAB_SPACES: u32 = 4;

/// Clippy lint groups enabled workspace-wide via `.cargo/config.toml`.
pub const CLIPPY_LINT_GROUPS_ENABLED: [&str; 3] = ["all", "pedantic", "nursery"];

/// Clippy lints explicitly allowed (pedantic noise that does not add value).
pub const CLIPPY_LINTS_ALLOWED: [&str; 4] = [
    "module_name_repetitions",
    "too_many_lines",
    "missing_errors_doc",
    "missing_panics_doc",
];

/// Errors raised when the rust-quality acceptance shape fails to load.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RustQualityAcceptanceError {
    /// A required workspace-root tooling file was missing.
    MissingFile(PathBuf),
}

impl fmt::Display for RustQualityAcceptanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFile(path) => {
                write!(f, "missing rust-quality tooling file: {}", path.display())
            }
        }
    }
}

impl Error for RustQualityAcceptanceError {}

/// Deterministic acceptance shape for the rust-quality CI gate.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RustQualityAcceptance {
    pub paths: RustQualityToolingPaths,
    pub channel: String,
    pub components: Vec<String>,
    pub clippy_msrv: String,
    pub clippy_cognitive_complexity_threshold: u32,
    pub clippy_type_complexity_threshold: u32,
    pub rustfmt_max_width: u32,
    pub rustfmt_tab_spaces: u32,
    pub clippy_lint_groups_enabled: Vec<String>,
    pub clippy_lints_allowed: Vec<String>,
}

impl RustQualityAcceptance {
    /// Build the canonical acceptance shape rooted at `workspace_root`.
    #[must_use]
    pub fn canonical(workspace_root: &Path) -> Self {
        Self {
            paths: RustQualityToolingPaths::canonical(workspace_root),
            channel: RUST_TOOLCHAIN_CHANNEL.to_string(),
            components: REQUIRED_TOOLCHAIN_COMPONENTS
                .iter()
                .map(|component| (*component).to_string())
                .collect(),
            clippy_msrv: CLIPPY_MSRV.to_string(),
            clippy_cognitive_complexity_threshold: CLIPPY_COGNITIVE_COMPLEXITY_THRESHOLD,
            clippy_type_complexity_threshold: CLIPPY_TYPE_COMPLEXITY_THRESHOLD,
            rustfmt_max_width: RUSTFMT_MAX_WIDTH,
            rustfmt_tab_spaces: RUSTFMT_TAB_SPACES,
            clippy_lint_groups_enabled: CLIPPY_LINT_GROUPS_ENABLED
                .iter()
                .map(|group| (*group).to_string())
                .collect(),
            clippy_lints_allowed: CLIPPY_LINTS_ALLOWED
                .iter()
                .map(|lint| (*lint).to_string())
                .collect(),
        }
    }

    /// Verify every canonical tooling file is present on disk.
    pub fn verify_files_present(&self) -> Result<(), RustQualityAcceptanceError> {
        for path in [
            &self.paths.rust_toolchain,
            &self.paths.clippy_config,
            &self.paths.rustfmt_config,
            &self.paths.cargo_config,
            &self.paths.deny_config,
            &self.paths.audit_config,
            &self.paths.ci_workflow,
        ] {
            if !path.exists() {
                return Err(RustQualityAcceptanceError::MissingFile(path.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RustQualityAcceptance, RustQualityAcceptanceError, CLIPPY_LINTS_ALLOWED,
        CLIPPY_LINT_GROUPS_ENABLED, REQUIRED_TOOLCHAIN_COMPONENTS, RUST_TOOLCHAIN_CHANNEL,
    };
    use std::path::{Path, PathBuf};

    #[test]
    fn canonical_shape_matches_constants() {
        let workspace = workspace_root();
        let acceptance = RustQualityAcceptance::canonical(&workspace);

        assert_eq!(acceptance.channel, RUST_TOOLCHAIN_CHANNEL);
        assert_eq!(
            acceptance.components,
            REQUIRED_TOOLCHAIN_COMPONENTS
                .iter()
                .map(|c| (*c).to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            acceptance.clippy_lint_groups_enabled,
            CLIPPY_LINT_GROUPS_ENABLED
                .iter()
                .map(|g| (*g).to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            acceptance.clippy_lints_allowed,
            CLIPPY_LINTS_ALLOWED
                .iter()
                .map(|l| (*l).to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn canonical_files_are_present() {
        let workspace = workspace_root();
        let acceptance = RustQualityAcceptance::canonical(&workspace);
        acceptance
            .verify_files_present()
            .expect("workspace-root rust-quality tooling files must exist");
    }

    #[test]
    fn missing_file_is_reported() {
        let mut acceptance = RustQualityAcceptance::canonical(Path::new("/nonexistent"));
        acceptance.paths.rust_toolchain = PathBuf::from("/nonexistent/rust-toolchain.toml");
        let err = acceptance.verify_files_present().unwrap_err();
        assert!(matches!(err, RustQualityAcceptanceError::MissingFile(_)));
    }

    fn workspace_root() -> PathBuf {
        // `CARGO_MANIFEST_DIR` is the e2e crate dir; the workspace root is one level up.
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root parent")
            .to_path_buf()
    }
}
