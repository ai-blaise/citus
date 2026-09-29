//! Source marker coverage is an identity check, not implementation evidence.

// FEATURE: D10

use super::{FeatureRegister, RegisterError};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE_ROOTS: [&str; 10] = [
    "companion",
    "sidecar",
    "pool",
    "operator",
    "e2e",
    "tools",
    "patches",
    "deploy",
    "images",
    "scripts",
];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SourceCoverage {
    pub source_files: usize,
    pub marker_ids: usize,
    pub register_ids: usize,
}

impl SourceCoverage {
    pub fn render(&self) -> String {
        format!(
            concat!(
                "feature_register_source_coverage\tpassed\n",
                "source_files\t{}\n",
                "source_marker_ids\t{}\n",
                "register_ids\t{}\n",
                "claim_boundary\tidentity-coverage-only\n"
            ),
            self.source_files, self.marker_ids, self.register_ids
        )
    }
}

pub fn validate_source_coverage(
    repo: impl AsRef<Path>,
    register: &FeatureRegister,
) -> Result<SourceCoverage, RegisterError> {
    let repo = fs::canonicalize(repo.as_ref())
        .map_err(|error| RegisterError::new(format!("cannot resolve repository: {error}")))?;
    let output = Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
        ])
        .args(SOURCE_ROOTS)
        .output()
        .map_err(|error| {
            RegisterError::new(format!("cannot enumerate Git source files: {error}"))
        })?;
    if !output.status.success() {
        return Err(RegisterError::new(format!(
            "cannot enumerate Git source files: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let listing = String::from_utf8(output.stdout)
        .map_err(|_| RegisterError::new("Git source paths must be UTF-8"))?;
    let paths: BTreeSet<PathBuf> = listing
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| {
            !path
                .components()
                .any(|component| matches!(component.as_os_str().to_str(), Some("target" | ".git")))
        })
        .collect();
    check_paths(&repo, register, paths)
}

fn check_paths(
    repo: &Path,
    register: &FeatureRegister,
    paths: impl IntoIterator<Item = PathBuf>,
) -> Result<SourceCoverage, RegisterError> {
    let mut marker_locations = BTreeMap::new();
    let mut source_files = 0;
    for path in paths {
        let absolute = repo.join(&path);
        match fs::symlink_metadata(&absolute) {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(RegisterError::new(format!(
                    "cannot inspect source {}: {error}",
                    path.display()
                )));
            }
        }
        let resolved = fs::canonicalize(&absolute).map_err(|error| {
            RegisterError::new(format!("cannot resolve source {}: {error}", path.display()))
        })?;
        if !resolved.starts_with(repo) || !resolved.is_file() {
            return Err(RegisterError::new(format!(
                "source {} must resolve to a regular file inside the repository",
                path.display()
            )));
        }
        let bytes = fs::read(&resolved).map_err(|error| {
            RegisterError::new(format!("cannot read source {}: {error}", path.display()))
        })?;
        source_files += 1;
        for (line, id) in marker_ids(&bytes) {
            marker_locations.entry(id).or_insert((path.clone(), line));
        }
    }
    if marker_locations.is_empty() {
        return Err(RegisterError::new(
            "no source feature markers found in the overlay roots",
        ));
    }
    let register_ids: BTreeSet<&str> = register.rows().iter().map(|row| row.id.as_str()).collect();
    let missing: Vec<String> = marker_locations
        .iter()
        .filter(|(id, _)| !register_ids.contains(id.as_str()))
        .map(|(id, (path, line))| format!("{id} ({}:{line})", path.display()))
        .collect();
    if !missing.is_empty() {
        return Err(RegisterError::new(format!(
            "source feature markers missing from docs/features.tsv: {}",
            missing.join(", ")
        )));
    }
    Ok(SourceCoverage {
        source_files,
        marker_ids: marker_locations.len(),
        register_ids: register_ids.len(),
    })
}

fn marker_ids(bytes: &[u8]) -> Vec<(usize, String)> {
    let mut ids = Vec::new();
    let prefix = b"FEATURE:";
    for (offset, line) in bytes.split(|byte| *byte == b'\n').enumerate() {
        for position in 0..line.len().saturating_sub(prefix.len()) {
            if !line[position..].starts_with(prefix) {
                continue;
            }
            let tail = &line[position + prefix.len()..];
            let whitespace = tail
                .iter()
                .take_while(|byte| byte.is_ascii_whitespace())
                .count();
            if whitespace == 0 {
                continue;
            }
            let token = &tail[whitespace..];
            if !token.first().is_some_and(u8::is_ascii_alphabetic) {
                continue;
            }
            let length = token
                .iter()
                .take_while(|byte| byte.is_ascii_alphanumeric())
                .count();
            let id: String = token[..length]
                .iter()
                .map(|byte| char::from(*byte))
                .collect();
            ids.push((offset + 1, id));
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::marker_ids;

    #[test]
    fn markers_are_line_local_ascii_identities() {
        let input = format!(
            "// {}: A12\n`{}: Sec7`, {}: MR6\n{}:\nnot_an_id\n{}: `markers`\n",
            "FEATURE", "FEATURE", "FEATURE", "FEATURE", "FEATURE"
        );
        assert_eq!(
            marker_ids(input.as_bytes()),
            vec![(1, "A12".into()), (2, "Sec7".into()), (2, "MR6".into())]
        );
    }
}
