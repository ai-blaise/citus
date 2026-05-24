use ai_blaise_citus_schema_designer::{
    canonical_schema_designer_model, schema_designer_model_from_snapshot,
};
use ai_blaise_citus_tool_runtime::parse_snapshot_tsv;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    let result = match args.first().map(String::as_str) {
        None | Some("run-canonical") => run_canonical(),
        Some("render-svg") => render_svg(&args[1..]),
        Some(_) => {
            eprintln!("citus-schema-designer: unknown command");
            print_usage();
            process::exit(2);
        }
    };

    if let Err(error) = result {
        eprintln!("citus-schema-designer: {error}");
        process::exit(1);
    }
}

fn run_canonical() -> Result<(), String> {
    let model = canonical_schema_designer_model();
    let layers = model
        .overlay_layers()
        .map_err(|error| format!("canonical model failed: {error}"))?;

    println!("tables\trelationships\tshard_placements\toverlay_layers");
    println!(
        "{}\t{}\t{}\t{}",
        model.tables.len(),
        model.relationships.len(),
        model.shard_map.len(),
        layers.len(),
    );
    Ok(())
}

fn render_svg(args: &[String]) -> Result<(), String> {
    let path = required_value(args, "--snapshot")?;
    let input = fs::read_to_string(&path).map_err(|error| format!("read {path}: {error}"))?;
    let snapshot = parse_snapshot_tsv(&input).map_err(|error| format!("parse {path}: {error}"))?;
    let model =
        schema_designer_model_from_snapshot(&snapshot).map_err(|error| error.to_string())?;
    println!("{}", model.render_svg().map_err(|error| error.to_string())?);
    Ok(())
}

fn required_value(args: &[String], flag: &str) -> Result<String, String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
        .ok_or_else(|| format!("missing required {flag}"))
}

fn print_usage() {
    println!("usage: citus-schema-designer [run-canonical]");
    println!("       citus-schema-designer render-svg --snapshot <snapshot.tsv>");
    println!("runs the canonical TSV report or renders snapshot-backed SVG");
}
