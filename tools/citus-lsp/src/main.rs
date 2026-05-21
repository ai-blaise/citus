// FEATURE: D4
// FEATURE: M5
// FEATURE: TS8

use ai_blaise_citus_lsp::{
    all_lsp_rules, canonical_analysis_request, canonical_lsp_plan, parse_metadata_tsv,
    parse_sql_document, CitusLspPlan, DiagnosticSeverity, LspDiagnostic, LspDiagnosticCode,
    LspQuickFixAction,
};
use std::env;
use std::fs;
use std::io::{self, Read};
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
    println!(
        "emits tab-separated diagnostics for supported Citus/Timescale SQL migration statements"
    );
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
