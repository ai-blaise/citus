// FEATURE: D4
// FEATURE: M5
// FEATURE: TS8

use ai_blaise_citus_lsp::{
    canonical_analysis_request, canonical_lsp_plan, DiagnosticSeverity, LspDiagnostic,
    LspDiagnosticCode, LspQuickFixAction,
};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if !args.is_empty() && args != ["analyze-canonical"] {
        eprintln!("citus-lsp: unknown command");
        print_usage();
        process::exit(2);
    }

    let plan = canonical_lsp_plan().unwrap_or_else(|error| {
        eprintln!("citus-lsp: failed to build canonical diagnostic plan: {error}");
        process::exit(1);
    });
    let request = canonical_analysis_request();
    let analysis = plan.analyze(&request).unwrap_or_else(|error| {
        eprintln!("citus-lsp: failed to analyze canonical request: {error}");
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
    println!("usage: citus-lsp [analyze-canonical]");
    println!("emits tab-separated diagnostics for the canonical Citus/Timescale SQL scenario");
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
