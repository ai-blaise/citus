use ai_blaise_citus_tool_runtime::parse_snapshot_tsv;
use ai_blaise_citus_watch::{canonical_watch_dashboard, WatchRuntime, WatchRuntimeError};
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
        Some("render-frame") => render_frame(&args[1..]),
        Some(_) => {
            eprintln!("citus-watch: unknown command");
            print_usage();
            process::exit(2);
        }
    };

    if let Err(error) = result {
        eprintln!("citus-watch: {error}");
        process::exit(1);
    }
}

fn run_canonical() -> Result<(), String> {
    let plan = canonical_watch_dashboard();
    let queries = plan
        .queries()
        .map_err(|error| format!("canonical dashboard failed: {error}"))?;

    println!("data_sources\tpanels\tqueries\trefresh_interval_seconds");
    println!(
        "{}\t{}\t{}\t{}",
        plan.data_sources.len(),
        plan.panels.len(),
        queries.len(),
        plan.refresh_interval_seconds,
    );
    Ok(())
}

fn render_frame(args: &[String]) -> Result<(), String> {
    let path = required_value(args, "--snapshot")?;
    let input = fs::read_to_string(&path).map_err(|error| format!("read {path}: {error}"))?;
    let snapshot = parse_snapshot_tsv(&input).map_err(|error| format!("parse {path}: {error}"))?;
    let runtime = WatchRuntime::new(snapshot).map_err(to_string)?;
    println!("{}", runtime.render_frame().map_err(to_string)?);
    Ok(())
}

fn required_value(args: &[String], flag: &str) -> Result<String, String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
        .ok_or_else(|| format!("missing required {flag}"))
}

fn to_string(error: WatchRuntimeError) -> String {
    error.to_string()
}

fn print_usage() {
    println!("usage: citus-watch [run-canonical]");
    println!("       citus-watch render-frame --snapshot <snapshot.tsv>");
    println!("runs the canonical TSV report or snapshot-backed dashboard frame runtime");
}
