use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use ai_blaise_feature_register::HEADER;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct TestRepo {
    path: PathBuf,
}

impl TestRepo {
    fn valid() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let sequence = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "ai-blaise-feature-register-cli-{}-{nonce}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("docs")).expect("create docs");
        fs::create_dir_all(path.join("src")).expect("create src");
        fs::write(path.join("src/feature.rs"), "pub fn feature() {}\n").expect("write source");
        let register = format!(
            "{HEADER}\n{}{}",
            "A\tFeature A\tproduct\tactive\t-\timplemented\tsrc/feature.rs\t-\t-\tproduction-ready\tBounded A\tNo current release receipt\n",
            "B\tFeature B\tcapability\talias\tA\tpartial\t-\t-\t-\talpha\tBounded B\tCanonical feature owns runtime\n"
        );
        fs::write(path.join("docs/features.tsv"), register).expect("write register");
        fs::create_dir_all(path.join("docs/ai-blaise")).expect("create legacy docs");
        fs::write(
            path.join("docs/ai-blaise/NEW_FEATURES.md"),
            "# Legacy feature register\n\n### A: Historical A\n### B: Historical B\n",
        )
        .expect("write legacy register");
        Self { path }
    }

    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ai_blaise_feature_register"))
            .args(arguments)
            .arg("--repo")
            .arg(&self.path)
            .output()
            .expect("run validator")
    }

    fn mutate_register_rows(&self, mut mutate: impl FnMut(&mut [String])) {
        let path = self.path.join("docs/features.tsv");
        let contents = fs::read_to_string(&path).expect("read register");
        let mut lines = contents.lines();
        let mut rewritten = format!("{}\n", lines.next().expect("register header"));
        for line in lines {
            let mut columns: Vec<String> = line.split('\t').map(str::to_owned).collect();
            mutate(&mut columns);
            rewritten.push_str(&columns.join("\t"));
            rewritten.push('\n');
        }
        fs::write(path, rewritten).expect("write mutated register");
    }
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn check_command_prints_structural_counts() {
    let repo = TestRepo::valid();
    let output = repo.run(&["check"]);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("UTF-8 stdout"),
        concat!(
            "feature_register_check\tpassed\n",
            "rows\t2\n",
            "canonical_edges\t1\n",
            "source_paths\t1\n",
            "check_paths\t0\n",
            "evidence_paths\t0\n"
        )
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn summary_command_prints_every_enum_bucket_in_stable_order() {
    let repo = TestRepo::valid();
    let output = repo.run(&["summary"]);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    assert!(stdout.starts_with(concat!(
        "feature_register_summary\tvalid\n",
        "rows\t2\n",
        "kind\tproduct\t1\n",
        "kind\tintegration\t0\n",
        "kind\tcapability\t1\n"
    )));
    assert!(stdout.contains("disposition\tactive\t1\n"));
    assert!(stdout.contains("disposition\talias\t1\n"));
    assert!(stdout.contains("maturity\tpartial\t1\n"));
    assert!(stdout.contains("maturity\timplemented\t1\n"));
    assert!(stdout.contains("claimed_status\talpha\t1\n"));
    assert!(stdout.contains("claimed_status\tproduction-ready\t1\n"));
    assert!(stdout.ends_with("evidence_paths\t0\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn validation_failure_is_closed_and_row_specific() {
    let repo = TestRepo::valid();
    let path = repo.path.join("docs/features.tsv");
    let invalid = fs::read_to_string(&path)
        .expect("read register")
        .replace("\tpartial\t", "\tproduction-ready\t");
    fs::write(path, invalid).expect("write invalid register");

    let output = repo.run(&["check"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert!(stderr.starts_with("feature-register: row 3, column maturity:"));
    assert!(stderr.contains("unknown value `production-ready`"));
}

#[test]
fn invalid_cli_shape_fails_with_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_ai_blaise_feature_register"))
        .args(["check", "--unknown"])
        .output()
        .expect("run validator");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert!(stderr.contains("invalid arguments"));
    assert!(stderr.contains("usage: ai_blaise_feature_register"));
}

#[test]
fn legacy_coverage_command_validates_then_reports_exact_counts() {
    let repo = TestRepo::valid();
    let output = repo.run(&["check-legacy-coverage"]);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("UTF-8 stdout"),
        concat!(
            "feature_register_legacy_coverage\tpassed\n",
            "register_ids\t2\n",
            "legacy_heading_ids\t2\n"
        )
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn legacy_coverage_mismatch_fails_deterministically() {
    let repo = TestRepo::valid();
    fs::write(
        repo.path.join("docs/ai-blaise/NEW_FEATURES.md"),
        "### B: Present\n### C: Extra\n",
    )
    .expect("write mismatched legacy register");
    let output = repo.run(&["check-legacy-coverage"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("UTF-8 stderr"),
        "feature-register: legacy ID coverage mismatch: missing_from_legacy=A; extra_in_legacy=C\n"
    );
}

#[test]
fn legacy_coverage_validates_tsv_before_reading_legacy_document() {
    let repo = TestRepo::valid();
    fs::remove_file(repo.path.join("docs/ai-blaise/NEW_FEATURES.md"))
        .expect("remove legacy register");
    let register_path = repo.path.join("docs/features.tsv");
    let invalid = fs::read_to_string(&register_path)
        .expect("read register")
        .replace("\tpartial\t", "\tproduction-ready\t");
    fs::write(register_path, invalid).expect("write invalid register");

    let output = repo.run(&["check-legacy-coverage"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert!(stderr.contains("row 3, column maturity"));
    assert!(!stderr.contains("NEW_FEATURES.md"));
}

#[test]
fn release_gaps_is_complete_deterministic_and_fail_closed() {
    let repo = TestRepo::valid();
    let output = repo.run(&["release-gaps"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines[0], "feature_release_gaps\tblocked");
    assert_eq!(lines[1], "release_evidence_verifier\tunimplemented");
    assert_eq!(
        lines[2],
        "feature_release_gap_columns\tid\ttitle\tkind\tdisposition\tcanonical_ids\tmaturity\thistorical_claimed_status\tscope\tblockers"
    );
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[2].split('\t').count(), 10);
    assert_eq!(lines[3].split('\t').count(), 10);
    assert_eq!(lines[4].split('\t').count(), 10);
    assert_eq!(
        lines[3],
        "feature_release_gap\tA\tFeature A\tproduct\tactive\t-\timplemented\tproduction-ready\tBounded A\tNo current release receipt"
    );
    assert_eq!(
        lines[4],
        "feature_release_gap\tB\tFeature B\tcapability\talias\tA\tpartial\talpha\tBounded B\tCanonical feature owns runtime"
    );
}

#[test]
fn historical_claims_cannot_authorize_a_release() {
    let repo = TestRepo::valid();
    repo.mutate_register_rows(|columns| columns[9] = "production-ready".to_owned());

    let output = repo.run(&["release-gaps"]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    assert!(stdout.starts_with(concat!(
        "feature_release_gaps\tblocked\n",
        "release_evidence_verifier\tunimplemented\n"
    )));
    assert_eq!(stdout.matches("\tproduction-ready\t").count(), 2);
}

#[test]
fn implementation_maturity_cannot_authorize_a_release() {
    let repo = TestRepo::valid();
    repo.mutate_register_rows(|columns| columns[5] = "implemented".to_owned());

    let output = repo.run(&["release-gaps"]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    assert_eq!(stdout.matches("\timplemented\t").count(), 2);
    assert!(stdout.contains("release_evidence_verifier\tunimplemented\n"));
}

#[test]
fn arbitrary_local_evidence_cannot_authorize_a_release() {
    let repo = TestRepo::valid();
    repo.mutate_register_rows(|columns| columns[8] = "src/feature.rs".to_owned());

    let output = repo.run(&["release-gaps"]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    assert!(stdout.contains("release_evidence_verifier\tunimplemented\n"));
    assert_eq!(stdout.matches("feature_release_gap\t").count(), 2);
}

#[test]
fn release_gaps_invalid_or_missing_register_exits_two_without_a_blocked_report() {
    let repo = TestRepo::valid();
    let path = repo.path.join("docs/features.tsv");
    let invalid = fs::read_to_string(&path)
        .expect("read register")
        .replace("\tpartial\t", "\tproduction-ready\t");
    fs::write(&path, invalid).expect("write invalid register");

    let invalid_output = repo.run(&["release-gaps"]);
    assert_eq!(invalid_output.status.code(), Some(2));
    assert!(invalid_output.stdout.is_empty());
    assert!(String::from_utf8(invalid_output.stderr)
        .expect("UTF-8 stderr")
        .contains("row 3, column maturity"));

    fs::remove_file(path).expect("remove register");
    let missing_output = repo.run(&["release-gaps"]);
    assert_eq!(missing_output.status.code(), Some(2));
    assert!(missing_output.stdout.is_empty());
    assert!(String::from_utf8(missing_output.stderr)
        .expect("UTF-8 stderr")
        .contains("cannot resolve docs/features.tsv"));
}

impl TestRepo {
    fn init_source_tree(&self, id: &str) {
        let output = Command::new("git")
            .args(["init", "--quiet"])
            .arg(&self.path)
            .output()
            .expect("initialize fixture Git repository");
        assert!(output.status.success());
        self.write_source_marker("tools/example/src/lib.rs", id);
    }

    fn write_source_marker(&self, path: &str, id: &str) {
        let path = self.path.join(path);
        fs::create_dir_all(path.parent().expect("source parent")).expect("create source directory");
        fs::write(
            path,
            format!("// {}: {id}\npub fn example() {{}}\n", "FEATURE"),
        )
        .expect("write source marker");
    }
}

#[test]
fn source_coverage_checks_identities_without_requiring_executable_targets() {
    let repo = TestRepo::valid();
    repo.init_source_tree("A");
    let output = repo.run(&["check-source-coverage"]);
    assert!(output.status.success(), "{:?}", output);
    assert_eq!(
        String::from_utf8(output.stdout).expect("UTF-8 stdout"),
        concat!(
            "feature_register_source_coverage\tpassed\n",
            "source_files\t1\n",
            "source_marker_ids\t1\n",
            "register_ids\t2\n",
            "claim_boundary\tidentity-coverage-only\n"
        )
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn source_coverage_rejects_unknown_ids_with_source_location() {
    let repo = TestRepo::valid();
    repo.init_source_tree("Z7");
    let output = repo.run(&["check-source-coverage"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .expect("UTF-8 stderr")
        .contains("Z7 (tools/example/src/lib.rs:1)"));
}

#[test]
fn source_coverage_ignores_build_outputs_and_untracked_git_ignored_files() {
    let repo = TestRepo::valid();
    repo.init_source_tree("A");
    repo.write_source_marker("tools/example/target/generated.rs", "MissingBuildArtifact");
    repo.write_source_marker("tools/example/ignored.rs", "MissingIgnoredArtifact");
    fs::write(repo.path.join(".gitignore"), "tools/example/ignored.rs\n")
        .expect("write Git ignore rules");
    let output = repo.run(&["check-source-coverage"]);
    assert!(output.status.success(), "{:?}", output);
    assert!(String::from_utf8(output.stdout)
        .expect("UTF-8 stdout")
        .contains("source_files\t1\n"));
}

#[test]
fn source_coverage_cannot_pass_without_git_or_markers() {
    let repo = TestRepo::valid();
    let not_git = repo.run(&["check-source-coverage"]);
    assert_eq!(not_git.status.code(), Some(2));
    assert!(String::from_utf8(not_git.stderr)
        .expect("UTF-8 stderr")
        .contains("cannot enumerate Git source files"));

    repo.init_source_tree("A");
    fs::write(
        repo.path.join("tools/example/src/lib.rs"),
        "pub fn no_marker() {}\n",
    )
    .expect("remove source marker");
    let no_markers = repo.run(&["check-source-coverage"]);
    assert_eq!(no_markers.status.code(), Some(2));
    assert!(String::from_utf8(no_markers.stderr)
        .expect("UTF-8 stderr")
        .contains("no source feature markers found"));
}

#[cfg(unix)]
#[test]
fn source_coverage_rejects_a_symlink_outside_the_repository() {
    use std::os::unix::fs::symlink;

    let repo = TestRepo::valid();
    repo.init_source_tree("A");
    let outside = TestRepo::valid();
    symlink(
        outside.path.join("src/feature.rs"),
        repo.path.join("tools/example/escape.rs"),
    )
    .expect("create escaping source symlink");
    let output = repo.run(&["check-source-coverage"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr)
        .expect("UTF-8 stderr")
        .contains(
            "source tools/example/escape.rs must resolve to a regular file inside the repository"
        ));
}
