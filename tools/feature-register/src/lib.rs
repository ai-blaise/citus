//! Structural validator for the machine-readable feature register.
//!
//! Passing validation is deliberately not a release or production-readiness
//! claim. The register keeps implementation maturity separate from release
//! evidence and promotion policy.

// FEATURE: D10

mod source_coverage;

pub use source_coverage::{validate_source_coverage, SourceCoverage};

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const REGISTER_RELATIVE_PATH: &str = "docs/features.tsv";
pub const LEGACY_REGISTER_RELATIVE_PATH: &str = "docs/ai-blaise/NEW_FEATURES.md";
pub const HEADER: &str = "id\ttitle\tkind\tdisposition\tcanonical_ids\tmaturity\tsource_paths\tcheck_paths\tevidence_paths\tclaimed_status\tscope\tblockers";

const KINDS: [&str; 8] = [
    "product",
    "integration",
    "capability",
    "tool",
    "operator",
    "runbook",
    "guard",
    "benchmark",
];
const DISPOSITIONS: [&str; 7] = [
    "active",
    "alias",
    "component-of",
    "evidence-only",
    "external-owned",
    "tombstone",
    "deferred",
];
const MATURITIES: [&str; 5] = [
    "unreviewed",
    "unimplemented",
    "model-only",
    "partial",
    "implemented",
];
const CLAIMED_STATUSES: [&str; 3] = ["alpha", "production-ready", "none"];

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegisterError {
    message: String,
}

impl RegisterError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn row(row: usize, column: &str, message: impl fmt::Display) -> Self {
        Self::new(format!("row {row}, column {column}: {message}"))
    }
}

impl fmt::Display for RegisterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RegisterError {}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FeatureRow {
    pub line: usize,
    pub id: String,
    pub title: String,
    pub kind: String,
    pub disposition: String,
    pub canonical_ids: Vec<String>,
    pub maturity: String,
    pub source_paths: Vec<String>,
    pub check_paths: Vec<String>,
    pub evidence_paths: Vec<String>,
    pub claimed_status: String,
    pub scope: String,
    pub blockers: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FeatureRegister {
    rows: Vec<FeatureRow>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct LegacyCoverage {
    pub register_ids: usize,
    pub legacy_heading_ids: usize,
}

impl LegacyCoverage {
    pub fn render(&self) -> String {
        format!(
            concat!(
                "feature_register_legacy_coverage\tpassed\n",
                "register_ids\t{}\n",
                "legacy_heading_ids\t{}\n"
            ),
            self.register_ids, self.legacy_heading_ids
        )
    }
}

impl FeatureRegister {
    pub fn rows(&self) -> &[FeatureRow] {
        &self.rows
    }

    pub fn render_check(&self) -> String {
        format!(
            concat!(
                "feature_register_check\tpassed\n",
                "rows\t{}\n",
                "canonical_edges\t{}\n",
                "source_paths\t{}\n",
                "check_paths\t{}\n",
                "evidence_paths\t{}\n"
            ),
            self.rows.len(),
            self.canonical_edge_count(),
            self.path_count(|row| &row.source_paths),
            self.path_count(|row| &row.check_paths),
            self.path_count(|row| &row.evidence_paths),
        )
    }

    pub fn render_summary(&self) -> String {
        let mut output = String::new();
        output.push_str("feature_register_summary\tvalid\n");
        output.push_str(&format!("rows\t{}\n", self.rows.len()));
        append_counts(&mut output, "kind", &KINDS, &self.rows, |row| {
            row.kind.as_str()
        });
        append_counts(
            &mut output,
            "disposition",
            &DISPOSITIONS,
            &self.rows,
            |row| row.disposition.as_str(),
        );
        append_counts(&mut output, "maturity", &MATURITIES, &self.rows, |row| {
            row.maturity.as_str()
        });
        append_counts(
            &mut output,
            "claimed_status",
            &CLAIMED_STATUSES,
            &self.rows,
            |row| row.claimed_status.as_str(),
        );
        output.push_str(&format!(
            concat!(
                "canonical_edges\t{}\n",
                "source_paths\t{}\n",
                "check_paths\t{}\n",
                "evidence_paths\t{}\n"
            ),
            self.canonical_edge_count(),
            self.path_count(|row| &row.source_paths),
            self.path_count(|row| &row.check_paths),
            self.path_count(|row| &row.evidence_paths),
        ));
        output
    }

    /// Render the complete, structurally validated release-gap inventory.
    ///
    /// This report is deliberately unable to qualify a release. In particular,
    /// implementation maturity, historical claimed status, and the presence of
    /// source, check, or evidence paths are not release certification.
    pub fn render_release_gaps(&self) -> String {
        let mut output = String::from(concat!(
            "feature_release_gaps\tblocked\n",
            "release_evidence_verifier\tunimplemented\n",
            "feature_release_gap_columns\tid\ttitle\tkind\tdisposition\tcanonical_ids\tmaturity\thistorical_claimed_status\tscope\tblockers\n",
        ));
        for row in &self.rows {
            let canonical_ids = if row.canonical_ids.is_empty() {
                "-".to_owned()
            } else {
                row.canonical_ids.join(";")
            };
            output.push_str(&format!(
                "feature_release_gap\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                row.id,
                row.title,
                row.kind,
                row.disposition,
                canonical_ids,
                row.maturity,
                row.claimed_status,
                row.scope,
                row.blockers,
            ));
        }
        output
    }

    fn canonical_edge_count(&self) -> usize {
        self.rows.iter().map(|row| row.canonical_ids.len()).sum()
    }

    fn path_count(&self, select: impl Fn(&FeatureRow) -> &[String]) -> usize {
        self.rows.iter().map(|row| select(row).len()).sum()
    }
}

fn append_counts(
    output: &mut String,
    category: &str,
    values: &[&str],
    rows: &[FeatureRow],
    select: impl Fn(&FeatureRow) -> &str,
) {
    for value in values {
        let count = rows.iter().filter(|row| select(row) == *value).count();
        output.push_str(&format!("{category}\t{value}\t{count}\n"));
    }
}

pub fn load_and_validate(repo: impl AsRef<Path>) -> Result<FeatureRegister, RegisterError> {
    let repo = repo.as_ref();
    let canonical_repo = fs::canonicalize(repo).map_err(|error| {
        RegisterError::new(format!(
            "cannot resolve repository {}: {error}",
            repo.display()
        ))
    })?;
    if !canonical_repo.is_dir() {
        return Err(RegisterError::new(format!(
            "repository path is not a directory: {}",
            repo.display()
        )));
    }

    let register_path = canonical_repo.join(REGISTER_RELATIVE_PATH);
    let canonical_register = fs::canonicalize(&register_path).map_err(|error| {
        RegisterError::new(format!("cannot resolve {REGISTER_RELATIVE_PATH}: {error}"))
    })?;
    if !canonical_register.starts_with(&canonical_repo) {
        return Err(RegisterError::new(format!(
            "{REGISTER_RELATIVE_PATH} resolves outside the repository"
        )));
    }
    if !canonical_register.is_file() {
        return Err(RegisterError::new(format!(
            "{REGISTER_RELATIVE_PATH} does not resolve to a regular file"
        )));
    }
    let contents = fs::read_to_string(&canonical_register).map_err(|error| {
        RegisterError::new(format!("cannot read {REGISTER_RELATIVE_PATH}: {error}"))
    })?;
    parse_and_validate(&contents, &canonical_repo)
}

pub fn validate_legacy_coverage(
    repo: impl AsRef<Path>,
    register: &FeatureRegister,
) -> Result<LegacyCoverage, RegisterError> {
    let repo = repo.as_ref();
    let canonical_repo = fs::canonicalize(repo).map_err(|error| {
        RegisterError::new(format!(
            "cannot resolve repository {}: {error}",
            repo.display()
        ))
    })?;
    if !canonical_repo.is_dir() {
        return Err(RegisterError::new(format!(
            "repository path is not a directory: {}",
            repo.display()
        )));
    }

    let legacy_path = canonical_repo.join(LEGACY_REGISTER_RELATIVE_PATH);
    let canonical_legacy = fs::canonicalize(&legacy_path).map_err(|error| {
        RegisterError::new(format!(
            "cannot resolve {LEGACY_REGISTER_RELATIVE_PATH}: {error}"
        ))
    })?;
    if !canonical_legacy.starts_with(&canonical_repo) {
        return Err(RegisterError::new(format!(
            "{LEGACY_REGISTER_RELATIVE_PATH} resolves outside the repository"
        )));
    }
    if !canonical_legacy.is_file() {
        return Err(RegisterError::new(format!(
            "{LEGACY_REGISTER_RELATIVE_PATH} does not resolve to a regular file"
        )));
    }
    let contents = fs::read_to_string(canonical_legacy).map_err(|error| {
        RegisterError::new(format!(
            "cannot read {LEGACY_REGISTER_RELATIVE_PATH}: {error}"
        ))
    })?;

    let mut legacy_ids = BTreeMap::new();
    for (offset, line) in contents.split_terminator('\n').enumerate() {
        let line_number = offset + 1;
        let Some(heading) = line.strip_prefix("### ") else {
            continue;
        };
        let Some((id, title)) = heading.split_once(": ") else {
            return Err(RegisterError::new(format!(
                "legacy line {line_number}: expected heading `### <ASCII_ID>: <title>`"
            )));
        };
        if !is_valid_feature_id(id) {
            return Err(RegisterError::new(format!(
                "legacy line {line_number}: invalid stable ID `{id}`; expected ASCII [A-Za-z][A-Za-z0-9]*"
            )));
        }
        if title.is_empty() || title.chars().any(char::is_control) {
            return Err(RegisterError::new(format!(
                "legacy line {line_number}: feature heading title must be nonempty and contain no control characters"
            )));
        }
        if let Some(first_line) = legacy_ids.insert(id.to_owned(), line_number) {
            return Err(RegisterError::new(format!(
                "legacy line {line_number}: duplicate feature ID `{id}`; first occurrence is legacy line {first_line}"
            )));
        }
    }
    if legacy_ids.is_empty() {
        return Err(RegisterError::new(format!(
            "{LEGACY_REGISTER_RELATIVE_PATH} contains no feature headings"
        )));
    }

    let register_ids: BTreeSet<&str> = register.rows.iter().map(|row| row.id.as_str()).collect();
    let legacy_id_set: BTreeSet<&str> = legacy_ids.keys().map(String::as_str).collect();
    let missing_from_legacy: Vec<&str> = register_ids.difference(&legacy_id_set).copied().collect();
    let extra_in_legacy: Vec<&str> = legacy_id_set.difference(&register_ids).copied().collect();
    if !missing_from_legacy.is_empty() || !extra_in_legacy.is_empty() {
        return Err(RegisterError::new(format!(
            "legacy ID coverage mismatch: missing_from_legacy={}; extra_in_legacy={}",
            render_id_set(&missing_from_legacy),
            render_id_set(&extra_in_legacy)
        )));
    }

    Ok(LegacyCoverage {
        register_ids: register_ids.len(),
        legacy_heading_ids: legacy_id_set.len(),
    })
}

fn render_id_set(ids: &[&str]) -> String {
    if ids.is_empty() {
        "-".to_owned()
    } else {
        ids.join(",")
    }
}

fn parse_and_validate(
    contents: &str,
    canonical_repo: &Path,
) -> Result<FeatureRegister, RegisterError> {
    let mut lines = contents.split_terminator('\n');
    let header = lines
        .next()
        .ok_or_else(|| RegisterError::new("register is empty; exact header is required"))?;
    if header != HEADER {
        return Err(RegisterError::new(format!(
            "header mismatch: expected exactly `{HEADER}`"
        )));
    }

    let mut rows = Vec::new();
    for (offset, line) in lines.enumerate() {
        let line_number = offset + 2;
        let columns: Vec<&str> = line.split('\t').collect();
        if columns.len() != 12 {
            return Err(RegisterError::new(format!(
                "row {line_number}: expected 12 tab-separated columns, found {}",
                columns.len()
            )));
        }

        const COLUMN_NAMES: [&str; 12] = [
            "id",
            "title",
            "kind",
            "disposition",
            "canonical_ids",
            "maturity",
            "source_paths",
            "check_paths",
            "evidence_paths",
            "claimed_status",
            "scope",
            "blockers",
        ];
        for (name, value) in COLUMN_NAMES.iter().zip(columns.iter()) {
            validate_required_text(line_number, name, value)?;
        }

        validate_feature_id(line_number, "id", columns[0])?;
        validate_enum(line_number, "kind", columns[2], &KINDS)?;
        validate_enum(line_number, "disposition", columns[3], &DISPOSITIONS)?;
        validate_enum(line_number, "maturity", columns[5], &MATURITIES)?;
        validate_enum(line_number, "claimed_status", columns[9], &CLAIMED_STATUSES)?;

        let canonical_ids = parse_list(line_number, "canonical_ids", columns[4])?;
        for canonical_id in &canonical_ids {
            validate_feature_id(line_number, "canonical_ids", canonical_id)?;
        }
        let source_paths =
            parse_path_list(line_number, "source_paths", columns[6], canonical_repo)?;
        let check_paths = parse_path_list(line_number, "check_paths", columns[7], canonical_repo)?;
        let evidence_paths =
            parse_path_list(line_number, "evidence_paths", columns[8], canonical_repo)?;

        let row = FeatureRow {
            line: line_number,
            id: columns[0].to_owned(),
            title: columns[1].to_owned(),
            kind: columns[2].to_owned(),
            disposition: columns[3].to_owned(),
            canonical_ids,
            maturity: columns[5].to_owned(),
            source_paths,
            check_paths,
            evidence_paths,
            claimed_status: columns[9].to_owned(),
            scope: columns[10].to_owned(),
            blockers: columns[11].to_owned(),
        };
        validate_row_invariants(&row)?;
        rows.push(row);
    }

    if rows.is_empty() {
        return Err(RegisterError::new(
            "register contains no feature rows after the header",
        ));
    }

    validate_id_order(&rows)?;
    validate_successor_graph(&rows)?;
    Ok(FeatureRegister { rows })
}

fn validate_required_text(row: usize, column: &str, value: &str) -> Result<(), RegisterError> {
    if value.is_empty() {
        return Err(RegisterError::row(row, column, "value must not be empty"));
    }
    if value.trim() != value {
        return Err(RegisterError::row(
            row,
            column,
            "leading or trailing whitespace is not allowed",
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(RegisterError::row(
            row,
            column,
            "control characters are not allowed",
        ));
    }
    Ok(())
}

fn validate_feature_id(row: usize, column: &str, value: &str) -> Result<(), RegisterError> {
    if is_valid_feature_id(value) {
        return Ok(());
    }
    Err(RegisterError::row(
        row,
        column,
        format!("invalid stable ID `{value}`; expected ASCII [A-Za-z][A-Za-z0-9]*"),
    ))
}

fn is_valid_feature_id(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric())
}

fn validate_enum(
    row: usize,
    column: &str,
    value: &str,
    allowed: &[&str],
) -> Result<(), RegisterError> {
    if allowed.contains(&value) {
        return Ok(());
    }
    Err(RegisterError::row(
        row,
        column,
        format!(
            "unknown value `{value}`; expected one of {}",
            allowed.join("|")
        ),
    ))
}

fn parse_list(row: usize, column: &str, value: &str) -> Result<Vec<String>, RegisterError> {
    if value == "-" {
        return Ok(Vec::new());
    }

    let mut values = Vec::new();
    let mut seen = BTreeSet::new();
    for item in value.split(';') {
        if item.is_empty() {
            return Err(RegisterError::row(
                row,
                column,
                "list contains an empty item",
            ));
        }
        if item == "-" {
            return Err(RegisterError::row(
                row,
                column,
                "`-` may only represent the entire empty list",
            ));
        }
        if item.trim() != item {
            return Err(RegisterError::row(
                row,
                column,
                format!("list item `{item}` has surrounding whitespace"),
            ));
        }
        if !seen.insert(item) {
            return Err(RegisterError::row(
                row,
                column,
                format!("duplicate list item `{item}`"),
            ));
        }
        values.push(item.to_owned());
    }
    Ok(values)
}

fn parse_path_list(
    row: usize,
    column: &str,
    value: &str,
    canonical_repo: &Path,
) -> Result<Vec<String>, RegisterError> {
    let paths = parse_list(row, column, value)?;
    for path in &paths {
        validate_local_file(row, column, path, canonical_repo)?;
    }
    Ok(paths)
}

fn validate_local_file(
    row: usize,
    column: &str,
    value: &str,
    canonical_repo: &Path,
) -> Result<(), RegisterError> {
    if value.contains('\\') {
        return Err(RegisterError::row(
            row,
            column,
            format!("path `{value}` must use `/` separators"),
        ));
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(RegisterError::row(
            row,
            column,
            format!("path `{value}` must be relative to the repository"),
        ));
    }
    if value
        .split('/')
        .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return Err(RegisterError::row(
            row,
            column,
            format!("path `{value}` contains an empty, `.` or `..` segment"),
        ));
    }

    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(RegisterError::row(
            row,
            column,
            format!("path `{value}` contains a non-normal component"),
        ));
    }

    let candidate = canonical_repo.join(path);
    let canonical_candidate = fs::canonicalize(&candidate).map_err(|error| {
        RegisterError::row(
            row,
            column,
            format!("path `{value}` cannot be resolved: {error}"),
        )
    })?;
    if !canonical_candidate.starts_with(canonical_repo) {
        return Err(RegisterError::row(
            row,
            column,
            format!("path `{value}` resolves outside the repository"),
        ));
    }
    if !canonical_candidate.is_file() {
        return Err(RegisterError::row(
            row,
            column,
            format!("path `{value}` does not resolve to a regular file"),
        ));
    }
    Ok(())
}

fn validate_row_invariants(row: &FeatureRow) -> Result<(), RegisterError> {
    if matches!(row.disposition.as_str(), "alias" | "component-of") && row.canonical_ids.is_empty()
    {
        return Err(RegisterError::row(
            row.line,
            "canonical_ids",
            format!("disposition `{}` requires a successor ID", row.disposition),
        ));
    }
    if row.kind == "benchmark" && row.disposition != "evidence-only" {
        return Err(RegisterError::row(
            row.line,
            "disposition",
            "kind `benchmark` requires disposition `evidence-only`",
        ));
    }
    if row.disposition == "active" && row.maturity == "implemented" && row.source_paths.is_empty() {
        return Err(RegisterError::row(
            row.line,
            "source_paths",
            "active implemented rows must name at least one source file",
        ));
    }
    if row.kind == "benchmark"
        && row.maturity == "implemented"
        && row.source_paths.is_empty()
        && row.check_paths.is_empty()
    {
        return Err(RegisterError::row(
            row.line,
            "check_paths",
            "implemented benchmarks must name a source file or check fixture",
        ));
    }
    Ok(())
}

fn validate_id_order(rows: &[FeatureRow]) -> Result<(), RegisterError> {
    for pair in rows.windows(2) {
        let previous = &pair[0];
        let current = &pair[1];
        if previous.id == current.id {
            return Err(RegisterError::row(
                current.line,
                "id",
                format!(
                    "duplicate ID `{}`; first occurrence is row {}",
                    current.id, previous.line
                ),
            ));
        }
        if previous.id > current.id {
            return Err(RegisterError::row(
                current.line,
                "id",
                format!(
                    "IDs must be strictly bytewise-lexicographically sorted; `{}` follows `{}`",
                    current.id, previous.id
                ),
            ));
        }
    }
    Ok(())
}

fn validate_successor_graph(rows: &[FeatureRow]) -> Result<(), RegisterError> {
    let indexes: BTreeMap<&str, usize> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.id.as_str(), index))
        .collect();
    let mut edges = vec![Vec::new(); rows.len()];

    for (index, row) in rows.iter().enumerate() {
        for successor in &row.canonical_ids {
            let successor_index = indexes.get(successor.as_str()).ok_or_else(|| {
                RegisterError::row(
                    row.line,
                    "canonical_ids",
                    format!("successor ID `{successor}` does not exist"),
                )
            })?;
            if *successor_index == index {
                return Err(RegisterError::row(
                    row.line,
                    "canonical_ids",
                    format!("self-successor `{successor}` is not allowed"),
                ));
            }
            edges[index].push(*successor_index);
        }
    }

    let mut states = vec![VisitState::Unvisited; rows.len()];
    let mut positions = vec![None; rows.len()];
    for start in 0..rows.len() {
        if states[start] != VisitState::Unvisited {
            continue;
        }

        states[start] = VisitState::Visiting;
        positions[start] = Some(0);
        let mut stack = vec![VisitFrame {
            node: start,
            next_edge: 0,
        }];

        while let Some(frame) = stack.last_mut() {
            if frame.next_edge == edges[frame.node].len() {
                let finished = stack.pop().expect("the DFS stack is not empty").node;
                positions[finished] = None;
                states[finished] = VisitState::Visited;
                continue;
            }

            let node = frame.node;
            let successor = edges[node][frame.next_edge];
            frame.next_edge += 1;
            match states[successor] {
                VisitState::Unvisited => {
                    states[successor] = VisitState::Visiting;
                    positions[successor] = Some(stack.len());
                    stack.push(VisitFrame {
                        node: successor,
                        next_edge: 0,
                    });
                }
                VisitState::Visiting => {
                    let cycle_start = positions[successor]
                        .expect("visiting nodes have a position in the DFS stack");
                    return Err(RegisterError::row(
                        rows[node].line,
                        "canonical_ids",
                        format!(
                            "successor cycle detected: {}",
                            render_cycle(&stack[cycle_start..], successor, rows)
                        ),
                    ));
                }
                VisitState::Visited => {}
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct VisitFrame {
    node: usize,
    next_edge: usize,
}

fn render_cycle(frames: &[VisitFrame], repeated: usize, rows: &[FeatureRow]) -> String {
    const EDGE_IDS: usize = 4;

    let ids: Vec<&str> = frames
        .iter()
        .map(|frame| rows[frame.node].id.as_str())
        .collect();
    if ids.len() <= EDGE_IDS * 2 {
        let mut complete = ids;
        complete.push(rows[repeated].id.as_str());
        return complete.join(" -> ");
    }

    let mut parts = Vec::with_capacity(EDGE_IDS * 2 + 2);
    parts.extend(ids[..EDGE_IDS].iter().copied().map(str::to_owned));
    parts.push(format!("... ({} nodes) ...", ids.len()));
    parts.extend(
        ids[ids.len() - EDGE_IDS..]
            .iter()
            .copied()
            .map(str::to_owned),
    );
    parts.push(rows[repeated].id.clone());
    parts.join(" -> ")
}

pub fn default_repo_from_current_dir() -> Result<PathBuf, RegisterError> {
    std::env::current_dir()
        .map_err(|error| RegisterError::new(format!("cannot read current directory: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct TestRepo {
        path: PathBuf,
    }

    impl TestRepo {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let sequence = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ai-blaise-feature-register-{}-{nonce}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(path.join("docs")).expect("create docs");
            fs::create_dir_all(path.join("src")).expect("create src");
            fs::create_dir_all(path.join("checks")).expect("create checks");
            fs::write(path.join("src/a.rs"), "fn main() {}\n").expect("write source");
            fs::write(path.join("checks/b.sh"), "#!/bin/sh\n").expect("write check");
            Self { path }
        }

        fn write_register(&self, rows: &[&str]) {
            let mut contents = format!("{HEADER}\n");
            for row in rows {
                contents.push_str(row);
                contents.push('\n');
            }
            fs::write(self.path.join(REGISTER_RELATIVE_PATH), contents).expect("write register");
        }

        fn write_legacy(&self, contents: &str) {
            let path = self.path.join(LEGACY_REGISTER_RELATIVE_PATH);
            fs::create_dir_all(path.parent().expect("legacy parent"))
                .expect("create legacy parent");
            fs::write(path, contents).expect("write legacy register");
        }
    }

    impl Drop for TestRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn active_row(id: &str) -> String {
        format!(
            "{id}\tActive feature\tproduct\tactive\t-\timplemented\tsrc/a.rs\t-\t-\tnone\tBounded behavior\tRelease receipt absent"
        )
    }

    fn benchmark_row(id: &str, successor: &str) -> String {
        format!(
            "{id}\tBenchmark evidence\tbenchmark\tevidence-only\t{successor}\timplemented\t-\tchecks/b.sh\t-\tproduction-ready\tBenchmark fixture only\tCurrent receipt absent"
        )
    }

    #[test]
    fn valid_register_renders_deterministic_counts() {
        let repo = TestRepo::new();
        let a = active_row("A");
        let b = benchmark_row("B", "A");
        repo.write_register(&[&a, &b]);

        let register = load_and_validate(&repo.path).expect("valid register");
        assert_eq!(register.rows().len(), 2);
        assert_eq!(
            register.render_check(),
            concat!(
                "feature_register_check\tpassed\n",
                "rows\t2\n",
                "canonical_edges\t1\n",
                "source_paths\t1\n",
                "check_paths\t1\n",
                "evidence_paths\t0\n"
            )
        );
        assert!(register.render_summary().contains("kind\tproduct\t1\n"));
        assert!(register
            .render_summary()
            .contains("maturity\timplemented\t2\n"));
        assert!(register
            .render_summary()
            .contains("claimed_status\tproduction-ready\t1\n"));
    }

    #[test]
    fn rejects_non_exact_header_and_column_count() {
        let repo = TestRepo::new();
        fs::write(
            repo.path.join(REGISTER_RELATIVE_PATH),
            "id\ttitle\nA\tTitle\n",
        )
        .expect("write invalid register");
        assert!(load_and_validate(&repo.path)
            .expect_err("bad header")
            .to_string()
            .contains("header mismatch"));

        fs::write(
            repo.path.join(REGISTER_RELATIVE_PATH),
            format!("{HEADER}\nA\tTitle\n"),
        )
        .expect("write invalid row");
        assert!(load_and_validate(&repo.path)
            .expect_err("bad columns")
            .to_string()
            .contains("row 2: expected 12 tab-separated columns, found 2"));
    }

    #[test]
    fn rejects_release_status_as_implementation_maturity() {
        let repo = TestRepo::new();
        let row = active_row("A").replace("\timplemented\t", "\tproduction-ready\t");
        repo.write_register(&[&row]);
        let error = load_and_validate(&repo.path).expect_err("invalid maturity");
        assert!(error.to_string().contains("row 2, column maturity"));
        assert!(error
            .to_string()
            .contains("unknown value `production-ready`"));
    }

    #[test]
    fn rejects_duplicate_and_unsorted_ids() {
        let repo = TestRepo::new();
        let first = active_row("A");
        let duplicate = active_row("A");
        repo.write_register(&[&first, &duplicate]);
        assert!(load_and_validate(&repo.path)
            .expect_err("duplicate")
            .to_string()
            .contains("duplicate ID `A`"));

        let b = active_row("B");
        repo.write_register(&[&b, &first]);
        assert!(load_and_validate(&repo.path)
            .expect_err("unsorted")
            .to_string()
            .contains("strictly bytewise-lexicographically sorted"));
    }

    #[test]
    fn rejects_non_ascii_or_unstable_id_shapes() {
        let repo = TestRepo::new();
        for invalid in ["1A", "A B", "A-1", "A_1", "Å1"] {
            let row = active_row(invalid);
            repo.write_register(&[&row]);
            let error = load_and_validate(&repo.path).expect_err("invalid row ID");
            assert!(error.to_string().contains("row 2, column id"));
            assert!(error
                .to_string()
                .contains("expected ASCII [A-Za-z][A-Za-z0-9]*"));
        }

        let invalid_successor =
            "A\tAlias\tcapability\talias\tB-1\tpartial\t-\t-\t-\tnone\tAlias scope\tNo receipt";
        repo.write_register(&[invalid_successor]);
        let error = load_and_validate(&repo.path).expect_err("invalid canonical ID");
        assert!(error.to_string().contains("row 2, column canonical_ids"));
        assert!(error.to_string().contains("invalid stable ID `B-1`"));
    }

    #[test]
    fn rejects_missing_self_and_cyclic_successors() {
        let repo = TestRepo::new();
        let missing = benchmark_row("A", "Z");
        repo.write_register(&[&missing]);
        assert!(load_and_validate(&repo.path)
            .expect_err("missing successor")
            .to_string()
            .contains("successor ID `Z` does not exist"));

        let self_edge = benchmark_row("A", "A");
        repo.write_register(&[&self_edge]);
        assert!(load_and_validate(&repo.path)
            .expect_err("self successor")
            .to_string()
            .contains("self-successor `A`"));

        let a = "A\tFirst\tcapability\tcomponent-of\tB\timplemented\t-\t-\t-\tnone\tFirst scope\tNo receipt";
        let b = "B\tSecond\tcapability\tcomponent-of\tA\timplemented\t-\t-\t-\tnone\tSecond scope\tNo receipt";
        repo.write_register(&[a, b]);
        let error = load_and_validate(&repo.path).expect_err("cycle");
        assert!(error.to_string().contains("successor cycle detected"));
        assert!(error.to_string().contains("A -> B -> A"));
    }

    #[test]
    fn validates_a_long_chain_and_bounds_a_long_cycle_diagnostic_without_recursion() {
        const NODE_COUNT: usize = 20_000;

        let repo = TestRepo::new();
        let render = |cycle: bool| {
            let mut contents = format!("{HEADER}\n");
            for index in 0..NODE_COUNT {
                let id = format!("F{index:05}");
                let (disposition, successor) = if index + 1 < NODE_COUNT {
                    ("component-of", format!("F{:05}", index + 1))
                } else if cycle {
                    ("component-of", "F00000".to_owned())
                } else {
                    ("active", "-".to_owned())
                };
                contents.push_str(&format!(
                    "{id}\tLong graph node\tcapability\t{disposition}\t{successor}\tpartial\t-\t-\t-\tnone\tGraph validation fixture\tNo release claim\n"
                ));
            }
            contents
        };

        fs::write(repo.path.join(REGISTER_RELATIVE_PATH), render(false)).expect("write long chain");
        let register = load_and_validate(&repo.path).expect("long chain is valid");
        assert_eq!(register.rows().len(), NODE_COUNT);

        fs::write(repo.path.join(REGISTER_RELATIVE_PATH), render(true)).expect("write long cycle");
        let error = load_and_validate(&repo.path)
            .expect_err("long cycle is invalid")
            .to_string();
        assert!(error.contains("successor cycle detected: F00000 -> F00001"));
        assert!(error.contains("... (20000 nodes) ..."));
        assert!(error.ends_with("F19998 -> F19999 -> F00000"));
        assert!(error.len() < 512, "cycle diagnostic must remain bounded");
    }

    #[test]
    fn enforces_disposition_and_implemented_source_invariants() {
        let repo = TestRepo::new();
        let alias =
            "A\tAlias\tcapability\talias\t-\timplemented\t-\t-\t-\tnone\tAlias scope\tNo receipt";
        repo.write_register(&[alias]);
        assert!(load_and_validate(&repo.path)
            .expect_err("alias without successor")
            .to_string()
            .contains("disposition `alias` requires a successor ID"));

        let component = "A\tComponent\tcapability\tcomponent-of\t-\tpartial\t-\t-\t-\tnone\tComponent scope\tNo receipt";
        repo.write_register(&[component]);
        assert!(load_and_validate(&repo.path)
            .expect_err("component without successor")
            .to_string()
            .contains("disposition `component-of` requires a successor ID"));

        let active =
            "A\tActive\tproduct\tactive\t-\timplemented\t-\t-\t-\tnone\tActive scope\tNo receipt";
        repo.write_register(&[active]);
        assert!(load_and_validate(&repo.path)
            .expect_err("active without source")
            .to_string()
            .contains("active implemented rows must name at least one source file"));

        let benchmark =
            "A\tBench\tbenchmark\tactive\t-\tpartial\t-\t-\t-\tnone\tBench scope\tNo receipt";
        repo.write_register(&[benchmark]);
        assert!(load_and_validate(&repo.path)
            .expect_err("benchmark disposition")
            .to_string()
            .contains("kind `benchmark` requires disposition `evidence-only`"));

        let benchmark_without_fixture = "A\tBench\tbenchmark\tevidence-only\t-\timplemented\t-\t-\t-\tnone\tBench scope\tNo receipt";
        repo.write_register(&[benchmark_without_fixture]);
        assert!(load_and_validate(&repo.path)
            .expect_err("implemented benchmark without fixture")
            .to_string()
            .contains("implemented benchmarks must name a source file or check fixture"));
    }

    #[test]
    fn rejects_malformed_lists_and_missing_paths() {
        let repo = TestRepo::new();
        let empty_item = active_row("A").replace("src/a.rs", "src/a.rs;");
        repo.write_register(&[&empty_item]);
        assert!(load_and_validate(&repo.path)
            .expect_err("empty list item")
            .to_string()
            .contains("list contains an empty item"));

        let duplicate = active_row("A").replace("src/a.rs", "src/a.rs;src/a.rs");
        repo.write_register(&[&duplicate]);
        assert!(load_and_validate(&repo.path)
            .expect_err("duplicate path")
            .to_string()
            .contains("duplicate list item `src/a.rs`"));

        let missing = active_row("A").replace("src/a.rs", "src/missing.rs");
        repo.write_register(&[&missing]);
        assert!(load_and_validate(&repo.path)
            .expect_err("missing path")
            .to_string()
            .contains("path `src/missing.rs` cannot be resolved"));
    }

    #[test]
    fn rejects_absolute_non_normal_directory_and_escape_paths() {
        let repo = TestRepo::new();
        let absolute = active_row("A").replace("src/a.rs", "/tmp/a.rs");
        repo.write_register(&[&absolute]);
        assert!(load_and_validate(&repo.path)
            .expect_err("absolute path")
            .to_string()
            .contains("must be relative to the repository"));

        let traversal = active_row("A").replace("src/a.rs", "src/../src/a.rs");
        repo.write_register(&[&traversal]);
        assert!(load_and_validate(&repo.path)
            .expect_err("path traversal")
            .to_string()
            .contains("contains an empty, `.` or `..` segment"));

        let internal_dot = active_row("A").replace("src/a.rs", "src/./a.rs");
        repo.write_register(&[&internal_dot]);
        assert!(load_and_validate(&repo.path)
            .expect_err("internal dot segment")
            .to_string()
            .contains("path `src/./a.rs` contains an empty, `.` or `..` segment"));

        let directory = active_row("A").replace("src/a.rs", "src");
        repo.write_register(&[&directory]);
        assert!(load_and_validate(&repo.path)
            .expect_err("directory")
            .to_string()
            .contains("does not resolve to a regular file"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let outside = repo.path.with_extension("outside");
            fs::write(&outside, "outside\n").expect("write outside file");
            symlink(&outside, repo.path.join("src/escape.rs")).expect("create escape symlink");
            let escaped = active_row("A").replace("src/a.rs", "src/escape.rs");
            repo.write_register(&[&escaped]);
            assert!(load_and_validate(&repo.path)
                .expect_err("symlink escape")
                .to_string()
                .contains("resolves outside the repository"));
            fs::remove_file(outside).expect("remove outside file");
        }
    }

    #[test]
    fn rejects_empty_whitespace_and_control_text() {
        let repo = TestRepo::new();
        let empty_title = active_row("A").replace("Active feature", "");
        repo.write_register(&[&empty_title]);
        assert!(load_and_validate(&repo.path)
            .expect_err("empty title")
            .to_string()
            .contains("column title: value must not be empty"));

        let padded_title = active_row("A").replace("Active feature", " Active feature");
        repo.write_register(&[&padded_title]);
        assert!(load_and_validate(&repo.path)
            .expect_err("padded title")
            .to_string()
            .contains("leading or trailing whitespace"));

        let controlled = active_row("A").replace("Bounded behavior", "Bounded\u{7f}behavior");
        repo.write_register(&[&controlled]);
        assert!(load_and_validate(&repo.path)
            .expect_err("control text")
            .to_string()
            .contains("control characters are not allowed"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_register_symlink_that_escapes_the_repository() {
        use std::os::unix::fs::symlink;

        let repo = TestRepo::new();
        let outside = repo.path.with_extension("outside-register");
        fs::write(&outside, format!("{HEADER}\n{}\n", active_row("A")))
            .expect("write outside register");
        symlink(&outside, repo.path.join(REGISTER_RELATIVE_PATH)).expect("create register symlink");

        assert!(load_and_validate(&repo.path)
            .expect_err("register symlink escape")
            .to_string()
            .contains("docs/features.tsv resolves outside the repository"));
        fs::remove_file(outside).expect("remove outside register");
    }

    #[test]
    fn legacy_coverage_requires_exact_nonempty_unique_id_sets() {
        let repo = TestRepo::new();
        let a = active_row("A");
        let b = "B\tComponent\tcapability\tcomponent-of\tA\tpartial\t-\t-\t-\tnone\tComponent scope\tNo receipt";
        repo.write_register(&[&a, b]);
        let register = load_and_validate(&repo.path).expect("valid register");

        repo.write_legacy("# Legacy\n\n### A: A different title is allowed\n### B: Component\n");
        assert_eq!(
            validate_legacy_coverage(&repo.path, &register).expect("exact legacy coverage"),
            LegacyCoverage {
                register_ids: 2,
                legacy_heading_ids: 2,
            }
        );

        repo.write_legacy("# Legacy without feature headings\n");
        assert!(validate_legacy_coverage(&repo.path, &register)
            .expect_err("empty legacy catalog")
            .to_string()
            .contains("contains no feature headings"));

        repo.write_legacy("### A: First\n### A: Duplicate\n");
        assert!(validate_legacy_coverage(&repo.path, &register)
            .expect_err("duplicate legacy heading")
            .to_string()
            .contains(
                "legacy line 2: duplicate feature ID `A`; first occurrence is legacy line 1"
            ));

        repo.write_legacy("### B: Present\n### C: Extra\n");
        assert_eq!(
            validate_legacy_coverage(&repo.path, &register)
                .expect_err("legacy mismatch")
                .to_string(),
            "legacy ID coverage mismatch: missing_from_legacy=A; extra_in_legacy=C"
        );

        repo.write_legacy("### B-1: Invalid\n");
        assert!(validate_legacy_coverage(&repo.path, &register)
            .expect_err("invalid legacy ID")
            .to_string()
            .contains("legacy line 1: invalid stable ID `B-1`"));

        fs::remove_file(repo.path.join(LEGACY_REGISTER_RELATIVE_PATH)).expect("remove legacy doc");
        assert!(validate_legacy_coverage(&repo.path, &register)
            .expect_err("missing legacy doc")
            .to_string()
            .contains("cannot resolve docs/ai-blaise/NEW_FEATURES.md"));
    }
}
