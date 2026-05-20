use ai_blaise_citus_schema_designer::canonical_schema_designer_model;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("citus-schema-designer: unknown command");
        print_usage();
        process::exit(2);
    }

    let model = canonical_schema_designer_model();
    let layers = model.overlay_layers().unwrap_or_else(|error| {
        eprintln!("citus-schema-designer: canonical model failed: {error}");
        process::exit(1);
    });

    println!("tables\trelationships\tshard_placements\toverlay_layers");
    println!(
        "{}\t{}\t{}\t{}",
        model.tables.len(),
        model.relationships.len(),
        model.shard_map.len(),
        layers.len(),
    );
}

fn print_usage() {
    println!("usage: citus-schema-designer [run-canonical]");
    println!("runs the deterministic canonical schema-designer overlay report and emits TSV");
}
