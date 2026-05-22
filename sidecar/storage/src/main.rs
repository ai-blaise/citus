// FEATURE: Sto1
// FEATURE: Sto3
// FEATURE: Sto4
// FEATURE: Sto5

use ai_blaise_citus_sidecar_shared::run_probe_server;
use ai_blaise_citus_sidecar_storage::{
    canonical_storage_report, canonical_storage_runtime_report, AntivirusVerdict, BucketAcl,
    ObjectStoreProvider, PresignedMethod,
};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["serve"] {
        run_server("storage", "0.0.0.0:8080");
        return;
    }

    if args == ["run-runtime-canonical"] {
        run_runtime_canonical();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("storage: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_storage_report().unwrap_or_else(|error| {
        eprintln!("storage: canonical report failed: {error}");
        process::exit(1);
    });
    let bucket = &report.plan.buckets[0];
    let antivirus = report.plan.antivirus.as_ref();

    println!(
        "provider\tbucket\ttenant_id\tacl\tobject_key\tcontent_type\tsize_bytes\tmethod\tttl_seconds\tantivirus_fail_closed\tscanner_endpoint"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        provider_name(&report.plan.provider),
        bucket.bucket,
        bucket.tenant_id,
        acl_name(&bucket.acl),
        report.metadata.object_key,
        report.metadata.content_type,
        report.metadata.size_bytes,
        method_name(&report.presigned_url.method),
        report.presigned_url.ttl_seconds,
        antivirus.map_or_else(|| "false".to_string(), |plan| plan.fail_closed.to_string(),),
        antivirus.map_or("none", |plan| plan.scanner_endpoint.as_str()),
    );
}

fn run_runtime_canonical() {
    let report = canonical_storage_runtime_report().unwrap_or_else(|error| {
        eprintln!("storage: canonical runtime report failed: {error}");
        process::exit(1);
    });

    println!(
        "bucket\ttenant_id\tobject_key\tcontent_type\tsize_bytes\tcontent_digest\tstored_objects\tquarantined_objects\tscanned_objects\tissued_urls\tantivirus_verdict\tpresigned_method\tpresigned_ttl\tpresigned_url"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.upload.metadata.bucket,
        report.upload.metadata.tenant_id,
        report.upload.metadata.object_key,
        report.upload.metadata.content_type,
        report.upload.metadata.size_bytes,
        report.upload.content_digest,
        report.state.stored_objects,
        report.state.quarantined_objects,
        report.state.scanned_objects,
        report.state.issued_urls,
        verdict_name(&report.upload.antivirus_verdict),
        method_name(&report.presigned_url.plan.method),
        report.presigned_url.expires_in_seconds,
        report.presigned_url.url,
    );
}

fn print_usage() {
    println!("usage: storage [serve|run-canonical|run-runtime-canonical]");
    println!("runs deterministic canonical storage sidecar plan/runtime reports and emits TSV");
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
}

fn provider_name(provider: &ObjectStoreProvider) -> &'static str {
    match provider {
        ObjectStoreProvider::S3 => "s3",
        ObjectStoreProvider::Gcs => "gcs",
        ObjectStoreProvider::AzureBlob => "azure_blob",
        ObjectStoreProvider::Minio => "minio",
    }
}

fn acl_name(acl: &BucketAcl) -> &'static str {
    match acl {
        BucketAcl::Private => "private",
        BucketAcl::TenantRead => "tenant_read",
        BucketAcl::TenantReadWrite => "tenant_read_write",
        BucketAcl::PublicRead => "public_read",
    }
}

fn method_name(method: &PresignedMethod) -> &'static str {
    match method {
        PresignedMethod::Get => "get",
        PresignedMethod::Put => "put",
        PresignedMethod::Delete => "delete",
    }
}

fn verdict_name(verdict: &AntivirusVerdict) -> &'static str {
    match verdict {
        AntivirusVerdict::Clean => "clean",
        AntivirusVerdict::Infected => "infected",
        AntivirusVerdict::NotScanned => "not_scanned",
    }
}
