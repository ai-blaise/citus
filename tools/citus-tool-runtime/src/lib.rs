//! Shared snapshot runtime for the Citus operator tools.

// FEATURE: D3
// FEATURE: D5
// FEATURE: D6
// FEATURE: D12
// FEATURE: M9
// FEATURE: O13

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ToolSnapshot {
    pub cluster_name: String,
    pub generated_at: String,
    pub workers: Vec<WorkerSnapshot>,
    pub tables: Vec<TableSnapshot>,
    pub shards: Vec<ShardSnapshot>,
    pub tenants: Vec<TenantSnapshot>,
    pub vectorizers: Vec<VectorizerSnapshot>,
    pub search_indexes: Vec<SearchIndexSnapshot>,
    pub branches: Vec<BranchSnapshot>,
    pub backups: Vec<BackupSnapshot>,
    pub realtime_streams: Vec<RealtimeStreamSnapshot>,
    pub pool: Option<PoolSnapshot>,
}

impl Default for ToolSnapshot {
    fn default() -> Self {
        Self {
            cluster_name: "citus".to_string(),
            generated_at: String::new(),
            workers: Vec::new(),
            tables: Vec::new(),
            shards: Vec::new(),
            tenants: Vec::new(),
            vectorizers: Vec::new(),
            search_indexes: Vec::new(),
            branches: Vec::new(),
            backups: Vec::new(),
            realtime_streams: Vec::new(),
            pool: None,
        }
    }
}

impl ToolSnapshot {
    pub fn validate(&self) -> Result<(), ToolRuntimeError> {
        validate_required("generated_at", &self.generated_at)?;
        if self.workers.is_empty() {
            return Err(ToolRuntimeError::MissingRequiredSection("worker"));
        }
        if self.tables.is_empty() {
            return Err(ToolRuntimeError::MissingRequiredSection("table"));
        }
        if self.shards.is_empty() {
            return Err(ToolRuntimeError::MissingRequiredSection("shard"));
        }

        let worker_names = self.worker_names();
        let table_names = self.table_names();

        for worker in &self.workers {
            worker.validate()?;
        }
        for table in &self.tables {
            table.validate()?;
        }
        for shard in &self.shards {
            shard.validate()?;
            if !table_names.contains(shard.table.as_str()) {
                return Err(ToolRuntimeError::UnknownReference {
                    row_kind: "shard",
                    field: "table",
                    value: shard.table.clone(),
                });
            }
            if !worker_names.contains(shard.worker.as_str()) {
                return Err(ToolRuntimeError::UnknownReference {
                    row_kind: "shard",
                    field: "worker",
                    value: shard.worker.clone(),
                });
            }
        }
        for tenant in &self.tenants {
            tenant.validate()?;
            if !worker_names.contains(tenant.home_worker.as_str()) {
                return Err(ToolRuntimeError::UnknownReference {
                    row_kind: "tenant",
                    field: "home_worker",
                    value: tenant.home_worker.clone(),
                });
            }
        }
        for vectorizer in &self.vectorizers {
            vectorizer.validate()?;
        }
        for index in &self.search_indexes {
            index.validate()?;
            if !table_names.contains(index.table.as_str()) {
                return Err(ToolRuntimeError::UnknownReference {
                    row_kind: "search_index",
                    field: "table",
                    value: index.table.clone(),
                });
            }
        }
        for branch in &self.branches {
            branch.validate()?;
        }
        for backup in &self.backups {
            backup.validate()?;
        }
        for stream in &self.realtime_streams {
            stream.validate()?;
            if !table_names.contains(stream.table.as_str()) {
                return Err(ToolRuntimeError::UnknownReference {
                    row_kind: "realtime",
                    field: "table",
                    value: stream.table.clone(),
                });
            }
        }
        if let Some(pool) = &self.pool {
            pool.validate()?;
        }
        Ok(())
    }

    pub fn worker_names(&self) -> BTreeSet<&str> {
        self.workers
            .iter()
            .map(|worker| worker.name.as_str())
            .collect()
    }

    pub fn table_names(&self) -> BTreeSet<&str> {
        self.tables
            .iter()
            .map(|table| table.name.as_str())
            .collect()
    }

    pub fn has_worker(&self, worker: &str) -> bool {
        self.workers.iter().any(|item| item.name == worker)
    }

    pub fn has_tenant(&self, tenant: &str) -> bool {
        self.tenants.iter().any(|item| item.tenant_id == tenant)
    }

    pub fn has_branch(&self, branch: &str) -> bool {
        self.branches.iter().any(|item| item.name == branch)
    }

    pub fn has_shard(&self, shard_id: u64) -> bool {
        self.shards.iter().any(|item| item.shard_id == shard_id)
    }

    pub fn shard_count(&self, table: &str) -> usize {
        self.shards
            .iter()
            .filter(|shard| shard.table == table)
            .count()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WorkerSnapshot {
    pub name: String,
    pub host: String,
    pub role: String,
    pub readiness: String,
}

impl WorkerSnapshot {
    fn validate(&self) -> Result<(), ToolRuntimeError> {
        validate_required("worker.name", &self.name)?;
        validate_required("worker.host", &self.host)?;
        validate_required("worker.role", &self.role)?;
        validate_required("worker.readiness", &self.readiness)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TableSnapshot {
    pub name: String,
    pub distribution_column: String,
    pub shard_count: u32,
    pub colocation_group: String,
    pub hypertable_time_column: Option<String>,
    pub chunk_interval: Option<String>,
    pub search_indexes: u32,
    pub webhook_count: u32,
}

impl TableSnapshot {
    fn validate(&self) -> Result<(), ToolRuntimeError> {
        validate_required("table.name", &self.name)?;
        validate_required("table.distribution_column", &self.distribution_column)?;
        validate_required("table.colocation_group", &self.colocation_group)?;
        if self.shard_count == 0 {
            return Err(ToolRuntimeError::InvalidNumber {
                field: "table.shard_count",
                value: self.shard_count.to_string(),
            });
        }
        if self.hypertable_time_column.is_some() && self.chunk_interval.is_none() {
            return Err(ToolRuntimeError::MissingRequiredField(
                "table.chunk_interval",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardSnapshot {
    pub table: String,
    pub shard_id: u64,
    pub worker: String,
    pub state: String,
    pub bytes: u64,
}

impl ShardSnapshot {
    fn validate(&self) -> Result<(), ToolRuntimeError> {
        validate_required("shard.table", &self.table)?;
        validate_required("shard.worker", &self.worker)?;
        validate_required("shard.state", &self.state)?;
        if self.shard_id == 0 {
            return Err(ToolRuntimeError::InvalidNumber {
                field: "shard.shard_id",
                value: self.shard_id.to_string(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantSnapshot {
    pub tenant_id: String,
    pub state: String,
    pub home_worker: String,
    pub shard_count: u32,
}

impl TenantSnapshot {
    fn validate(&self) -> Result<(), ToolRuntimeError> {
        validate_required("tenant.tenant_id", &self.tenant_id)?;
        validate_required("tenant.state", &self.state)?;
        validate_required("tenant.home_worker", &self.home_worker)?;
        if self.shard_count == 0 {
            return Err(ToolRuntimeError::InvalidNumber {
                field: "tenant.shard_count",
                value: self.shard_count.to_string(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorizerSnapshot {
    pub name: String,
    pub tenant_id: String,
    pub backlog_jobs: u32,
    pub budget_remaining_tokens: u64,
    pub state: String,
}

impl VectorizerSnapshot {
    fn validate(&self) -> Result<(), ToolRuntimeError> {
        validate_required("vectorizer.name", &self.name)?;
        validate_required("vectorizer.tenant_id", &self.tenant_id)?;
        validate_required("vectorizer.state", &self.state)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchIndexSnapshot {
    pub table: String,
    pub name: String,
    pub state: String,
    pub method: String,
}

impl SearchIndexSnapshot {
    fn validate(&self) -> Result<(), ToolRuntimeError> {
        validate_required("search_index.table", &self.table)?;
        validate_required("search_index.name", &self.name)?;
        validate_required("search_index.state", &self.state)?;
        validate_required("search_index.method", &self.method)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BranchSnapshot {
    pub name: String,
    pub state: String,
    pub lsn: String,
}

impl BranchSnapshot {
    fn validate(&self) -> Result<(), ToolRuntimeError> {
        validate_required("branch.name", &self.name)?;
        validate_required("branch.state", &self.state)?;
        validate_required("branch.lsn", &self.lsn)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupSnapshot {
    pub name: String,
    pub state: String,
    pub completed_at: String,
}

impl BackupSnapshot {
    fn validate(&self) -> Result<(), ToolRuntimeError> {
        validate_required("backup.name", &self.name)?;
        validate_required("backup.state", &self.state)?;
        validate_required("backup.completed_at", &self.completed_at)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeStreamSnapshot {
    pub tenant_id: String,
    pub table: String,
    pub subscribers: u32,
    pub confirmed_lsn: String,
}

impl RealtimeStreamSnapshot {
    fn validate(&self) -> Result<(), ToolRuntimeError> {
        validate_required("realtime.tenant_id", &self.tenant_id)?;
        validate_required("realtime.table", &self.table)?;
        validate_required("realtime.confirmed_lsn", &self.confirmed_lsn)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolSnapshot {
    pub state: String,
    pub active_clients: u32,
    pub waiting_clients: u32,
    pub upstream_errors: u32,
}

impl PoolSnapshot {
    pub fn is_ready(&self) -> bool {
        self.state == "ready" && self.upstream_errors == 0
    }

    fn validate(&self) -> Result<(), ToolRuntimeError> {
        validate_required("pool.state", &self.state)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ToolRuntimeError {
    InvalidNumber {
        field: &'static str,
        value: String,
    },
    MissingRequiredField(&'static str),
    MissingRequiredSection(&'static str),
    UnknownReference {
        row_kind: &'static str,
        field: &'static str,
        value: String,
    },
    UnknownRowKind {
        line: usize,
        kind: String,
    },
    WrongFieldCount {
        line: usize,
        kind: String,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for ToolRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNumber { field, value } => {
                write!(formatter, "{field} has invalid numeric value {value}")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::MissingRequiredSection(section) => {
                write!(formatter, "snapshot requires at least one {section} row")
            }
            Self::UnknownReference {
                row_kind,
                field,
                value,
            } => write!(
                formatter,
                "{row_kind}.{field} references unknown value {value}"
            ),
            Self::UnknownRowKind { line, kind } => {
                write!(formatter, "line {line} has unknown row kind {kind}")
            }
            Self::WrongFieldCount {
                line,
                kind,
                expected,
                actual,
            } => write!(
                formatter,
                "line {line} {kind} row has {actual} fields; expected {expected}"
            ),
        }
    }
}

impl Error for ToolRuntimeError {}

pub fn parse_snapshot_tsv(input: &str) -> Result<ToolSnapshot, ToolRuntimeError> {
    let mut snapshot = ToolSnapshot::default();

    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }

        let fields = line.split('\t').map(str::trim).collect::<Vec<_>>();
        let kind = fields.first().copied().unwrap_or_default();
        match kind {
            "meta" => {
                require_fields(&fields, 3, line_number, kind)?;
                match fields[1] {
                    "cluster_name" => snapshot.cluster_name = fields[2].to_string(),
                    "generated_at" => snapshot.generated_at = fields[2].to_string(),
                    other => {
                        return Err(ToolRuntimeError::UnknownReference {
                            row_kind: "meta",
                            field: "key",
                            value: other.to_string(),
                        })
                    }
                }
            }
            "worker" => {
                require_fields(&fields, 5, line_number, kind)?;
                snapshot.workers.push(WorkerSnapshot {
                    name: fields[1].to_string(),
                    host: fields[2].to_string(),
                    role: fields[3].to_string(),
                    readiness: fields[4].to_string(),
                });
            }
            "table" => {
                require_fields(&fields, 9, line_number, kind)?;
                snapshot.tables.push(TableSnapshot {
                    name: fields[1].to_string(),
                    distribution_column: fields[2].to_string(),
                    shard_count: parse_u32(fields[3], "table.shard_count")?,
                    colocation_group: fields[4].to_string(),
                    hypertable_time_column: non_empty(fields[5]),
                    chunk_interval: non_empty(fields[6]),
                    search_indexes: parse_u32(fields[7], "table.search_indexes")?,
                    webhook_count: parse_u32(fields[8], "table.webhook_count")?,
                });
            }
            "shard" => {
                require_fields(&fields, 6, line_number, kind)?;
                snapshot.shards.push(ShardSnapshot {
                    table: fields[1].to_string(),
                    shard_id: parse_u64(fields[2], "shard.shard_id")?,
                    worker: fields[3].to_string(),
                    state: fields[4].to_string(),
                    bytes: parse_u64(fields[5], "shard.bytes")?,
                });
            }
            "tenant" => {
                require_fields(&fields, 5, line_number, kind)?;
                snapshot.tenants.push(TenantSnapshot {
                    tenant_id: fields[1].to_string(),
                    state: fields[2].to_string(),
                    home_worker: fields[3].to_string(),
                    shard_count: parse_u32(fields[4], "tenant.shard_count")?,
                });
            }
            "vectorizer" => {
                require_fields(&fields, 6, line_number, kind)?;
                snapshot.vectorizers.push(VectorizerSnapshot {
                    name: fields[1].to_string(),
                    tenant_id: fields[2].to_string(),
                    backlog_jobs: parse_u32(fields[3], "vectorizer.backlog_jobs")?,
                    budget_remaining_tokens: parse_u64(
                        fields[4],
                        "vectorizer.budget_remaining_tokens",
                    )?,
                    state: fields[5].to_string(),
                });
            }
            "search_index" => {
                require_fields(&fields, 5, line_number, kind)?;
                snapshot.search_indexes.push(SearchIndexSnapshot {
                    table: fields[1].to_string(),
                    name: fields[2].to_string(),
                    state: fields[3].to_string(),
                    method: fields[4].to_string(),
                });
            }
            "branch" => {
                require_fields(&fields, 4, line_number, kind)?;
                snapshot.branches.push(BranchSnapshot {
                    name: fields[1].to_string(),
                    state: fields[2].to_string(),
                    lsn: fields[3].to_string(),
                });
            }
            "backup" => {
                require_fields(&fields, 4, line_number, kind)?;
                snapshot.backups.push(BackupSnapshot {
                    name: fields[1].to_string(),
                    state: fields[2].to_string(),
                    completed_at: fields[3].to_string(),
                });
            }
            "realtime" => {
                require_fields(&fields, 5, line_number, kind)?;
                snapshot.realtime_streams.push(RealtimeStreamSnapshot {
                    tenant_id: fields[1].to_string(),
                    table: fields[2].to_string(),
                    subscribers: parse_u32(fields[3], "realtime.subscribers")?,
                    confirmed_lsn: fields[4].to_string(),
                });
            }
            "pool" => {
                require_fields(&fields, 5, line_number, kind)?;
                snapshot.pool = Some(PoolSnapshot {
                    state: fields[1].to_string(),
                    active_clients: parse_u32(fields[2], "pool.active_clients")?,
                    waiting_clients: parse_u32(fields[3], "pool.waiting_clients")?,
                    upstream_errors: parse_u32(fields[4], "pool.upstream_errors")?,
                });
            }
            other => {
                return Err(ToolRuntimeError::UnknownRowKind {
                    line: line_number,
                    kind: other.to_string(),
                });
            }
        }
    }

    snapshot.validate()?;
    Ok(snapshot)
}

pub fn canonical_snapshot() -> ToolSnapshot {
    parse_snapshot_tsv(CANONICAL_SNAPSHOT_TSV).expect("canonical tools snapshot must validate")
}

pub const CANONICAL_SNAPSHOT_TSV: &str = "\
meta\tcluster_name\tprod-east\n\
meta\tgenerated_at\t2026-05-23T22:00:00Z\n\
worker\tworker-1\t10.0.0.11\tprimary\tready\n\
worker\tworker-2\t10.0.0.12\treplica\tready\n\
table\tpublic.events\ttenant_id\t32\ttenant\tcreated_at\t1 day\t1\t2\n\
shard\tpublic.events\t102008\tworker-1\tactive\t1048576\n\
shard\tpublic.events\t102009\tworker-2\tactive\t2097152\n\
tenant\ttenant-a\tactive\tworker-1\t2\n\
tenant\ttenant-b\tmoving\tworker-2\t2\n\
vectorizer\tdocuments-body\ttenant-a\t128\t250000\tok\n\
search_index\tpublic.events\tevents_search\tready\tbm25\n\
branch\tbranch-main\tactive\t0/16B6C50\n\
backup\tbackup-20260523\tcompleted\t2026-05-23T21:55:00Z\n\
realtime\ttenant-a\tpublic.events\t3\t0/16B6C50\n\
pool\tready\t42\t2\t0\n";

pub fn escape_html(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub fn terminal_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths = headers
        .iter()
        .map(|header| header.len())
        .collect::<Vec<_>>();
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            if index >= widths.len() {
                widths.push(0);
            }
            widths[index] = widths[index].max(value.len());
        }
    }

    let mut out = String::new();
    push_table_row(&mut out, headers.iter().copied(), &widths);
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>();
    push_table_row(&mut out, separator.iter().map(String::as_str), &widths);
    for row in rows {
        push_table_row(&mut out, row.iter().map(String::as_str), &widths);
    }
    out
}

fn push_table_row<'a>(out: &mut String, values: impl Iterator<Item = &'a str>, widths: &[usize]) {
    out.push('|');
    for (index, value) in values.enumerate() {
        out.push(' ');
        out.push_str(value);
        for _ in value.len()..widths[index] {
            out.push(' ');
        }
        out.push(' ');
        out.push('|');
    }
    out.push('\n');
}

fn require_fields(
    fields: &[&str],
    expected: usize,
    line: usize,
    kind: &str,
) -> Result<(), ToolRuntimeError> {
    if fields.len() != expected {
        return Err(ToolRuntimeError::WrongFieldCount {
            line,
            kind: kind.to_string(),
            expected,
            actual: fields.len(),
        });
    }
    Ok(())
}

fn parse_u32(value: &str, field: &'static str) -> Result<u32, ToolRuntimeError> {
    value
        .parse::<u32>()
        .map_err(|_| ToolRuntimeError::InvalidNumber {
            field,
            value: value.to_string(),
        })
}

fn parse_u64(value: &str, field: &'static str) -> Result<u64, ToolRuntimeError> {
    value
        .parse::<u64>()
        .map_err(|_| ToolRuntimeError::InvalidNumber {
            field,
            value: value.to_string(),
        })
}

fn non_empty(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), ToolRuntimeError> {
    if value.trim().is_empty() {
        return Err(ToolRuntimeError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_snapshot_validates_references() {
        let snapshot = canonical_snapshot();

        assert_eq!(snapshot.cluster_name, "prod-east");
        assert_eq!(snapshot.workers.len(), 2);
        assert_eq!(snapshot.shard_count("public.events"), 2);
        assert!(snapshot.has_tenant("tenant-a"));
        assert!(snapshot.pool.as_ref().unwrap().is_ready());
    }
}
