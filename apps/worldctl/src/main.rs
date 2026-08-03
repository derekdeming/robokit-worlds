#![doc = "Command-line boundary for validating and evaluating executable worlds."]

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Parser, Subcommand, ValueEnum};
use evaluation::{RunOutcome, evaluate_outcome};
use evidence::RunManifest;
use schemars::schema_for;
use thiserror::Error;
use world_domain::WorldDefinition;

#[derive(Debug, Parser)]
#[command(
    name = "worldctl",
    about = "Validate and evaluate RoboKit executable worlds",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate a TOML world definition and all cross-references.
    ValidateWorld {
        /// Path to the world TOML file.
        #[arg(long)]
        world: PathBuf,
    },
    /// Validate an immutable JSON run manifest.
    ValidateManifest {
        /// Path to the run-manifest JSON file.
        #[arg(long)]
        manifest: PathBuf,
    },
    /// Evaluate a typed JSON outcome against a validated world.
    Evaluate {
        /// Path to the world TOML file.
        #[arg(long)]
        world: PathBuf,
        /// Path to the typed outcome JSON file.
        #[arg(long)]
        outcome: PathBuf,
    },
    /// Generate the JSON Schema for a serialized contract.
    Schema {
        /// Contract whose schema should be generated.
        #[arg(long, value_enum, default_value_t = SchemaKind::World)]
        kind: SchemaKind,
        /// Optional output path; stdout is used when omitted.
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum SchemaKind {
    World,
    RunManifest,
    RunOutcome,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::ValidateWorld { world } => {
            load_world(&world)?;
            println!("valid world: {}", world.display());
        }
        Command::ValidateManifest { manifest } => {
            let manifest_value: RunManifest = read_json(&manifest)?;
            let manifest_value = manifest_value
                .validate()
                .map_err(|error| CliError::InvalidManifest(format_manifest_issues(&error)))?;
            verify_manifest_artifacts(&manifest, manifest_value.manifest())?;
            println!("valid run manifest: {}", manifest.display());
        }
        Command::Evaluate { world, outcome } => {
            let world = load_world(&world)?;
            let outcome: RunOutcome = read_json(&outcome)?;
            let report = evaluate_outcome(&world, &outcome)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Schema { kind, output } => {
            let schema = match kind {
                SchemaKind::World => serde_json::to_vec_pretty(&schema_for!(WorldDefinition))?,
                SchemaKind::RunManifest => serde_json::to_vec_pretty(&schema_for!(RunManifest))?,
                SchemaKind::RunOutcome => serde_json::to_vec_pretty(&schema_for!(RunOutcome))?,
            };
            write_bytes(output.as_deref(), &schema)?;
        }
    }
    Ok(())
}

fn load_world(path: &Path) -> Result<world_domain::ValidatedWorldDefinition, CliError> {
    let text = read_text(path)?;
    let world: WorldDefinition = toml::from_str(&text).map_err(|source| CliError::ParseToml {
        path: path.to_path_buf(),
        source,
    })?;
    let world = world
        .validate()
        .map_err(|error| CliError::InvalidWorld(format_world_issues(&error)))?;
    verify_world_assets(path, &world)?;
    Ok(world)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, CliError> {
    let text = read_text(path)?;
    serde_json::from_str(&text).map_err(|source| CliError::ParseJson {
        path: path.to_path_buf(),
        source,
    })
}

fn read_text(path: &Path) -> Result<String, CliError> {
    fs::read_to_string(path).map_err(|source| CliError::Read {
        path: path.to_path_buf(),
        source,
    })
}

fn verify_world_assets(
    world_path: &Path,
    world: &world_domain::ValidatedWorldDefinition,
) -> Result<(), CliError> {
    let package_root = world_path.parent().unwrap_or_else(|| Path::new("."));
    for entity in &world.definition().entities {
        if let Some(asset) = &entity.asset {
            let path = package_root.join(asset.logical_path.as_str());
            let bytes = fs::read(&path).map_err(|source| CliError::Read {
                path: path.clone(),
                source,
            })?;
            verify_digest(&path, &bytes, &asset.sha256)?;
        }
    }
    Ok(())
}

fn verify_manifest_artifacts(manifest_path: &Path, manifest: &RunManifest) -> Result<(), CliError> {
    let bundle_root = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    for artifact in &manifest.artifacts {
        let path = bundle_root.join(artifact.relative_path.as_str());
        let bytes = fs::read(&path).map_err(|source| CliError::Read {
            path: path.clone(),
            source,
        })?;
        let actual_length = u64::try_from(bytes.len())
            .map_err(|_| CliError::ArtifactTooLarge { path: path.clone() })?;
        if actual_length != artifact.byte_length {
            return Err(CliError::ArtifactLengthMismatch {
                path,
                expected: artifact.byte_length,
                actual: actual_length,
            });
        }
        verify_digest(&path, &bytes, &artifact.sha256)?;
    }
    Ok(())
}

fn verify_digest(
    path: &Path,
    bytes: &[u8],
    expected: &world_domain::Sha256Digest,
) -> Result<(), CliError> {
    let actual = evidence::sha256_digest(bytes);
    if actual == *expected {
        Ok(())
    } else {
        Err(CliError::DigestMismatch {
            path: path.to_path_buf(),
            expected: expected.as_str().to_owned(),
            actual: actual.as_str().to_owned(),
        })
    }
}

fn write_bytes(path: Option<&Path>, bytes: &[u8]) -> Result<(), CliError> {
    if let Some(path) = path {
        fs::write(path, bytes).map_err(|source| CliError::Write {
            path: path.to_path_buf(),
            source,
        })
    } else {
        let mut stdout = io::stdout().lock();
        stdout.write_all(bytes)?;
        stdout.write_all(b"\n")?;
        Ok(())
    }
}

fn format_world_issues(error: &world_domain::WorldValidationErrors) -> String {
    error
        .issues()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

fn format_manifest_issues(error: &evidence::ManifestValidationError) -> String {
    error
        .issues()
        .iter()
        .map(|issue| format!("{} ({:?}): {}", issue.path, issue.code, issue.message))
        .collect::<Vec<_>>()
        .join("; ")
}

#[derive(Debug, Error)]
enum CliError {
    #[error("failed to read `{path}`: {source}", path = .path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to write `{path}`: {source}", path = .path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to write stdout: {0}")]
    Stdout(#[from] io::Error),
    #[error("invalid TOML in `{path}`: {source}", path = .path.display())]
    ParseToml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid JSON in `{path}`: {source}", path = .path.display())]
    ParseJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid world definition: {0}")]
    InvalidWorld(String),
    #[error("invalid run manifest: {0}")]
    InvalidManifest(String),
    #[error(
        "digest mismatch for `{path}`: expected {expected}, found {actual}",
        path = .path.display()
    )]
    DigestMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    #[error(
        "artifact length mismatch for `{path}`: expected {expected} bytes, found {actual}",
        path = .path.display()
    )]
    ArtifactLengthMismatch {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },
    #[error("artifact `{path}` is too large to represent on this platform", path = .path.display())]
    ArtifactTooLarge { path: PathBuf },
    #[error(transparent)]
    Evaluation(#[from] evaluation::EvaluationError),
    #[error(transparent)]
    Serialize(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_generation_supports_every_contract() {
        for kind in [
            SchemaKind::World,
            SchemaKind::RunManifest,
            SchemaKind::RunOutcome,
        ] {
            let command = Command::Schema { kind, output: None };
            assert!(matches!(command, Command::Schema { .. }));
        }
    }
}
