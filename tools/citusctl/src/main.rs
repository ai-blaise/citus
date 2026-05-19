use ai_blaise_citusctl::parse_request;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_request(&args).and_then(|request| request.plan()) {
        Ok(plan) => {
            println!(
                "citusctl {} destructive={} requires_plan_id={} steps={}",
                plan.command_name,
                plan.destructive,
                plan.requires_plan_id,
                plan.steps.len()
            );
        }
        Err(error) => {
            eprintln!("citusctl: {error}");
            process::exit(2);
        }
    }
}
