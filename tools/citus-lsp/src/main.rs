// FEATURE: D4
// FEATURE: M5
// FEATURE: TS8

use ai_blaise_citus_lsp::{
    all_lsp_rules, canonical_analysis_request, canonical_lsp_plan, parse_metadata_tsv,
    parse_sql_document, CitusLspPlan, DiagnosticSeverity, LspDiagnostic, LspDiagnosticCode,
    LspQuickFixAction,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    match args.first().map(String::as_str) {
        None | Some("analyze-canonical") => run_canonical(),
        Some("analyze") => run_file_analysis(&args[1..]),
        Some("serve-stdio") => run_lsp_stdio(&args[1..]),
        Some(_) => {
            eprintln!("citus-lsp: unknown command");
            print_usage();
            process::exit(2);
        }
    }
}

fn run_canonical() {
    let plan = canonical_lsp_plan().unwrap_or_else(|error| {
        eprintln!("citus-lsp: failed to build canonical diagnostic plan: {error}");
        process::exit(1);
    });
    let request = canonical_analysis_request();
    emit_analysis(&plan, &request);
}

fn run_file_analysis(args: &[String]) {
    let mut metadata_path = None;
    let mut sql_path = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--metadata" => {
                index += 1;
                metadata_path = args.get(index).map(String::as_str);
            }
            "--sql" => {
                index += 1;
                sql_path = args.get(index).map(String::as_str);
            }
            unknown => {
                eprintln!("citus-lsp: unknown analyze option {unknown}");
                print_usage();
                process::exit(2);
            }
        }
        index += 1;
    }

    let Some(metadata_path) = metadata_path else {
        eprintln!("citus-lsp: analyze requires --metadata <path>");
        process::exit(2);
    };
    let Some(sql_path) = sql_path else {
        eprintln!("citus-lsp: analyze requires --sql <path|->");
        process::exit(2);
    };

    let metadata = read_input(metadata_path).unwrap_or_else(|error| {
        eprintln!("citus-lsp: failed to read metadata {metadata_path}: {error}");
        process::exit(1);
    });
    let sql = read_input(sql_path).unwrap_or_else(|error| {
        eprintln!("citus-lsp: failed to read SQL {sql_path}: {error}");
        process::exit(1);
    });

    let metadata = parse_metadata_tsv(&metadata).unwrap_or_else(|error| {
        eprintln!("citus-lsp: failed to parse metadata {metadata_path}: {error}");
        process::exit(1);
    });
    let request = parse_sql_document(input_uri(sql_path), &sql).unwrap_or_else(|error| {
        eprintln!("citus-lsp: failed to parse SQL {sql_path}: {error}");
        process::exit(1);
    });
    let plan = CitusLspPlan::new(metadata, all_lsp_rules()).unwrap_or_else(|error| {
        eprintln!("citus-lsp: failed to build diagnostic plan: {error}");
        process::exit(1);
    });

    emit_analysis(&plan, &request);
}

fn emit_analysis(plan: &CitusLspPlan, request: &ai_blaise_citus_lsp::SqlAnalysisRequest) {
    let analysis = plan.analyze(request).unwrap_or_else(|error| {
        eprintln!("citus-lsp: failed to analyze request: {error}");
        process::exit(1);
    });

    println!("uri\tcode\tseverity\tmessage\tquick_fix");
    for diagnostic in &analysis.diagnostics {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            escape_field(&analysis.uri),
            diagnostic_code(diagnostic.code),
            diagnostic_severity(diagnostic.severity),
            escape_field(&diagnostic.message),
            escape_field(&quick_fix_action(diagnostic))
        );
    }
}

fn print_usage() {
    println!("usage: citus-lsp analyze-canonical");
    println!("       citus-lsp analyze --metadata <metadata.tsv> --sql <migration.sql|->");
    println!("       citus-lsp serve-stdio --metadata <metadata.tsv>");
    println!("emits tab-separated diagnostics or runs a file-backed LSP stdio diagnostic server");
}

fn run_lsp_stdio(args: &[String]) {
    let metadata_path = required_option(args, "--metadata").unwrap_or_else(|error| {
        eprintln!("citus-lsp: {error}");
        process::exit(2);
    });
    let metadata = read_input(&metadata_path).unwrap_or_else(|error| {
        eprintln!("citus-lsp: failed to read metadata {metadata_path}: {error}");
        process::exit(1);
    });
    let metadata = parse_metadata_tsv(&metadata).unwrap_or_else(|error| {
        eprintln!("citus-lsp: failed to parse metadata {metadata_path}: {error}");
        process::exit(1);
    });
    let plan = CitusLspPlan::new(metadata, all_lsp_rules()).unwrap_or_else(|error| {
        eprintln!("citus-lsp: failed to build diagnostic plan: {error}");
        process::exit(1);
    });

    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(error) = serve_lsp_stdio(plan, stdin.lock(), stdout.lock()) {
        eprintln!("citus-lsp: LSP stdio failed: {error}");
        process::exit(1);
    }
}

fn read_input(path: &str) -> io::Result<String> {
    if path == "-" {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        Ok(input)
    } else {
        fs::read_to_string(path)
    }
}

fn input_uri(path: &str) -> String {
    if path == "-" {
        return "stdin://migration.sql".to_string();
    }

    let path = Path::new(path);
    match fs::canonicalize(path) {
        Ok(path) => format!("file://{}", path.display()),
        Err(_) => format!("file://{}", path.display()),
    }
}

fn required_option(args: &[String], flag: &str) -> Result<String, String> {
    let mut value = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            candidate if candidate == flag => {
                index += 1;
                value = args.get(index).cloned();
            }
            unknown => return Err(format!("unknown serve-stdio option {unknown}")),
        }
        index += 1;
    }
    value.ok_or_else(|| format!("serve-stdio requires {flag} <path>"))
}

struct LspRuntimeState {
    plan: CitusLspPlan,
    documents: BTreeMap<String, String>,
    shutdown_requested: bool,
}

fn serve_lsp_stdio<R: Read, W: Write>(
    plan: CitusLspPlan,
    reader: R,
    mut writer: W,
) -> io::Result<()> {
    let mut reader = BufReader::new(reader);
    let mut state = LspRuntimeState {
        plan,
        documents: BTreeMap::new(),
        shutdown_requested: false,
    };

    while let Some(body) = read_lsp_frame(&mut reader)? {
        let messages = match serde_json::from_str::<Value>(&body) {
            Ok(message) => handle_lsp_message(&mut state, message),
            Err(error) => vec![jsonrpc_error(
                Value::Null,
                -32700,
                format!("parse error: {error}"),
            )],
        };
        let should_exit = messages
            .iter()
            .any(|message| message.as_str() == Some("__exit__"));
        for message in messages
            .into_iter()
            .filter(|message| message.as_str() != Some("__exit__"))
        {
            write_lsp_frame(&mut writer, &message)?;
        }
        writer.flush()?;
        if should_exit {
            break;
        }
    }
    Ok(())
}

fn read_lsp_frame<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut content_length = None;
    loop {
        let mut header = String::new();
        let read = reader.read_line(&mut header)?;
        if read == 0 {
            return Ok(None);
        }
        let header = header.trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            break;
        }
        let Some((name, value)) = header.split_once(':') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid LSP header {header}"),
            ));
        };
        if name.eq_ignore_ascii_case("content-length") {
            let parsed = value.trim().parse::<usize>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Content-Length must be numeric")
            })?;
            content_length = Some(parsed);
        }
    }

    let Some(content_length) = content_length else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing Content-Length header",
        ));
    };
    let mut body = vec![0_u8; content_length];
    reader.read_exact(&mut body)?;
    String::from_utf8(body).map(Some).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("LSP body must be UTF-8: {error}"),
        )
    })
}

fn write_lsp_frame<W: Write>(writer: &mut W, message: &Value) -> io::Result<()> {
    let body = message.to_string();
    write!(writer, "Content-Length: {}\r\n\r\n{}", body.len(), body)
}

fn handle_lsp_message(state: &mut LspRuntimeState, message: Value) -> Vec<Value> {
    let id = message.get("id").cloned();
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        return vec![jsonrpc_error(
            id.unwrap_or(Value::Null),
            -32600,
            "method must be a string",
        )];
    };

    match method {
        "initialize" => vec![jsonrpc_result(
            id.unwrap_or(Value::Null),
            json!({
                "serverInfo": {
                    "name": "ai-blaise-citus-lsp",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": {
                    "textDocumentSync": {
                        "openClose": true,
                        "change": 1,
                    },
                    "diagnosticProvider": {
                        "interFileDependencies": false,
                        "workspaceDiagnostics": false,
                    },
                },
            }),
        )],
        "initialized" => Vec::new(),
        "shutdown" => {
            state.shutdown_requested = true;
            vec![jsonrpc_result(id.unwrap_or(Value::Null), Value::Null)]
        }
        "exit" => vec![Value::String("__exit__".to_string())],
        "textDocument/didOpen" => match did_open_document(state, message.get("params")) {
            Ok(notification) => vec![notification],
            Err(error) => notification_error(format!("didOpen rejected: {error}")),
        },
        "textDocument/didChange" => match did_change_document(state, message.get("params")) {
            Ok(notification) => vec![notification],
            Err(error) => notification_error(format!("didChange rejected: {error}")),
        },
        "textDocument/diagnostic" => {
            let Some(id) = id else {
                return notification_error(
                    "textDocument/diagnostic requires a request id".to_string(),
                );
            };
            match diagnostic_report(state, message.get("params")) {
                Ok(report) => vec![jsonrpc_result(id, report)],
                Err(error) => vec![jsonrpc_error(id, -32602, error)],
            }
        }
        _ => match id {
            Some(id) => vec![jsonrpc_error(
                id,
                -32601,
                format!("unknown method: {method}"),
            )],
            None => Vec::new(),
        },
    }
}

fn did_open_document(state: &mut LspRuntimeState, params: Option<&Value>) -> Result<Value, String> {
    let document = params
        .and_then(|params| params.get("textDocument"))
        .ok_or_else(|| "textDocument is required".to_string())?;
    let uri = json_string(document, "uri")?;
    let text = json_string(document, "text")?;
    state.documents.insert(uri.clone(), text);
    publish_diagnostics(state, &uri)
}

fn did_change_document(
    state: &mut LspRuntimeState,
    params: Option<&Value>,
) -> Result<Value, String> {
    let params = params.ok_or_else(|| "params are required".to_string())?;
    let uri = params
        .get("textDocument")
        .and_then(|document| document.get("uri"))
        .and_then(Value::as_str)
        .ok_or_else(|| "textDocument.uri is required".to_string())?
        .to_string();
    let changes = params
        .get("contentChanges")
        .and_then(Value::as_array)
        .ok_or_else(|| "contentChanges must be an array".to_string())?;
    let text = changes
        .last()
        .and_then(|change| change.get("text"))
        .and_then(Value::as_str)
        .ok_or_else(|| "contentChanges[].text is required".to_string())?
        .to_string();
    state.documents.insert(uri.clone(), text);
    publish_diagnostics(state, &uri)
}

fn diagnostic_report(state: &LspRuntimeState, params: Option<&Value>) -> Result<Value, String> {
    let uri = params
        .and_then(|params| params.get("textDocument"))
        .and_then(|document| document.get("uri"))
        .and_then(Value::as_str)
        .ok_or_else(|| "textDocument.uri is required".to_string())?;
    let diagnostics = diagnostics_for_document(state, uri)?;
    Ok(json!({
        "kind": "full",
        "items": diagnostics,
    }))
}

fn publish_diagnostics(state: &LspRuntimeState, uri: &str) -> Result<Value, String> {
    let diagnostics = diagnostics_for_document(state, uri)?;
    Ok(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "diagnostics": diagnostics,
        },
    }))
}

fn diagnostics_for_document(state: &LspRuntimeState, uri: &str) -> Result<Vec<Value>, String> {
    let text = state
        .documents
        .get(uri)
        .ok_or_else(|| format!("document is not open: {uri}"))?;
    let request = parse_sql_document(uri.to_string(), text).map_err(|error| error.to_string())?;
    let analysis = state
        .plan
        .analyze(&request)
        .map_err(|error| error.to_string())?;
    Ok(analysis
        .diagnostics
        .iter()
        .map(lsp_diagnostic_json)
        .collect())
}

fn lsp_diagnostic_json(diagnostic: &LspDiagnostic) -> Value {
    json!({
        "range": {
            "start": { "line": 0, "character": 0 },
            "end": { "line": 0, "character": 1 },
        },
        "severity": lsp_severity_number(diagnostic.severity),
        "code": diagnostic_code(diagnostic.code),
        "source": "ai-blaise-citus-lsp",
        "message": diagnostic.message,
        "data": {
            "quickFix": quick_fix_action(diagnostic),
        },
    })
}

fn lsp_severity_number(severity: DiagnosticSeverity) -> u8 {
    match severity {
        DiagnosticSeverity::Error => 1,
        DiagnosticSeverity::Warning => 2,
        DiagnosticSeverity::Information => 3,
    }
}

fn json_string(value: &Value, field: &'static str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("{field} must be a string"))
}

fn notification_error(message: String) -> Vec<Value> {
    vec![json!({
        "jsonrpc": "2.0",
        "method": "window/logMessage",
        "params": {
            "type": 1,
            "message": message,
        },
    })]
}

fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn jsonrpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into(),
        },
    })
}

fn diagnostic_code(code: LspDiagnosticCode) -> &'static str {
    match code {
        LspDiagnosticCode::NonColocatedJoin => "non_colocated_join",
        LspDiagnosticCode::DistributionColumnAlter => "distribution_column_alter",
        LspDiagnosticCode::HypertableInvariant => "hypertable_invariant",
        LspDiagnosticCode::MissingTenantFilter => "missing_tenant_filter",
        LspDiagnosticCode::MissingSearchAnalyzer => "missing_search_analyzer",
        LspDiagnosticCode::MissingDistributionColumn => "missing_distribution_column",
    }
}

fn diagnostic_severity(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Information => "information",
    }
}

fn quick_fix_action(diagnostic: &LspDiagnostic) -> String {
    let Some(quick_fix) = &diagnostic.quick_fix else {
        return String::new();
    };

    match &quick_fix.action {
        LspQuickFixAction::AddDistributionColumn { table, column } => {
            format!("add_distribution_column table={table} column={column}")
        }
        LspQuickFixAction::AlignColocation {
            left_table,
            right_table,
            distribution_column,
        } => format!(
            "align_colocation left_table={left_table} right_table={right_table} distribution_column={distribution_column}"
        ),
        LspQuickFixAction::UseDistributedHypertableBridge { table, time_column } => {
            format!("use_distributed_hypertable_bridge table={table} time_column={time_column}")
        }
        LspQuickFixAction::AddTenantFilter {
            table,
            tenant_column,
        } => format!("add_tenant_filter table={table} tenant_column={tenant_column}"),
        LspQuickFixAction::SetSearchAnalyzer {
            index_name,
            analyzer,
        } => format!("set_search_analyzer index_name={index_name} analyzer={analyzer}"),
    }
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}
