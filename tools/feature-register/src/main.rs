use ai_blaise_feature_register::{
    default_repo_from_current_dir, load_and_validate, validate_legacy_coverage,
    validate_source_coverage,
};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process;

const USAGE: &str =
    "usage: ai_blaise_feature_register <check|summary|check-legacy-coverage|check-source-coverage|release-gaps> [--repo PATH]";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Command {
    Check,
    Summary,
    CheckLegacyCoverage,
    CheckSourceCoverage,
    ReleaseGaps,
}

struct CommandOutput {
    stdout: String,
    exit_code: i32,
}

impl CommandOutput {
    fn success(stdout: String) -> Self {
        Self {
            stdout,
            exit_code: 0,
        }
    }

    fn blocked(stdout: String) -> Self {
        Self {
            stdout,
            exit_code: 1,
        }
    }
}

fn main() {
    match run(std::env::args_os().skip(1).collect()) {
        Ok(output) => {
            print!("{}", output.stdout);
            if output.exit_code != 0 {
                process::exit(output.exit_code);
            }
        }
        Err(error) => {
            eprintln!("feature-register: {error}");
            process::exit(2);
        }
    }
}

fn run(arguments: Vec<OsString>) -> Result<CommandOutput, String> {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("--help" | "-h" | "help")) {
        return Ok(CommandOutput::success(format!("{USAGE}\n")));
    }

    let command = match arguments.first().and_then(|argument| argument.to_str()) {
        Some("check") => Command::Check,
        Some("summary") => Command::Summary,
        Some("check-legacy-coverage") => Command::CheckLegacyCoverage,
        Some("check-source-coverage") => Command::CheckSourceCoverage,
        Some("release-gaps") => Command::ReleaseGaps,
        Some(other) => return Err(format!("unknown command `{other}`; {USAGE}")),
        None if arguments.is_empty() => return Err(USAGE.to_owned()),
        None => return Err(format!("command is not valid UTF-8; {USAGE}")),
    };

    let repo = match arguments.as_slice() {
        [_] => default_repo_from_current_dir().map_err(|error| error.to_string())?,
        [_, flag, path] if flag == "--repo" => PathBuf::from(path),
        [_, flag] if flag == "--repo" => {
            return Err(format!("--repo requires a path; {USAGE}"));
        }
        _ => return Err(format!("invalid arguments; {USAGE}")),
    };

    let register = load_and_validate(&repo).map_err(|error| error.to_string())?;
    Ok(match command {
        Command::Check => CommandOutput::success(register.render_check()),
        Command::Summary => CommandOutput::success(register.render_summary()),
        Command::CheckLegacyCoverage => CommandOutput::success(
            validate_legacy_coverage(&repo, &register)
                .map_err(|error| error.to_string())?
                .render(),
        ),
        Command::CheckSourceCoverage => CommandOutput::success(
            validate_source_coverage(&repo, &register)
                .map_err(|error| error.to_string())?
                .render(),
        ),
        Command::ReleaseGaps => CommandOutput::blocked(register.render_release_gaps()),
    })
}
