// FEATURE: Auth1
// FEATURE: Auth2
// FEATURE: Auth4
// FEATURE: Auth5

use ai_blaise_citus_sidecar_auth::{canonical_auth_report, SigningAlgorithm};
use ai_blaise_citus_sidecar_shared::run_probe_server;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }
    if args == ["serve"] {
        run_server("auth", "0.0.0.0:8080");
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("auth: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_auth_report().unwrap_or_else(|error| {
        eprintln!("auth: canonical report failed: {error}");
        process::exit(1);
    });
    let oidc = &report.sidecar.oidc_providers[0];

    println!(
        "issuer\tsubject\ttenant_id\trole\taudience\talgorithm\tttl_seconds\tintrospection_cache_ttl\toidc_provider\tmfa_totp\tmfa_webauthn"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.issue_plan.issuer,
        report.issue_plan.subject,
        report.issue_plan.tenant_id,
        report.issue_plan.role,
        report.issue_plan.audience,
        algorithm_name(&report.issue_plan.algorithm),
        report.issue_plan.ttl_seconds,
        report.introspection.cache_ttl_seconds,
        oidc.name,
        report
            .sidecar
            .mfa
            .as_ref()
            .map_or_else(|| "false".to_string(), |mfa| mfa.totp_enabled.to_string(),),
        report.sidecar.mfa.as_ref().map_or_else(
            || "false".to_string(),
            |mfa| mfa.webauthn_enabled.to_string(),
        ),
    );
}

fn print_usage() {
    println!("usage: auth [serve|run-canonical]");
    println!("runs the deterministic canonical auth sidecar plan and emits TSV");
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
}

fn algorithm_name(algorithm: &SigningAlgorithm) -> &'static str {
    match algorithm {
        SigningAlgorithm::Rs256 => "rs256",
        SigningAlgorithm::Hs256 => "hs256",
    }
}
