use std::{path::PathBuf, time::Duration};

use calibration::{CalibrationReport, GranularCalibrationDataset};
use clap::{Parser, Subcommand, ValueEnum};
use evaluation::{RunOutcome, evaluate_outcome};
use evidence::RunManifest;
use schemars::schema_for;
use world_domain::WorldDefinition;

use crate::{
    calibrate::{CalibrationOptions, calibrate_granular, validate_calibration_package},
    error::CliError,
    files::{load_world, read_json, verify_manifest_artifacts, write_bytes},
    simulate::{SimulationOptions, simulate_mujoco},
};

/// Validate, execute, and evaluate `RoboKit` executable worlds.
#[derive(Debug, Parser)]
#[command(
    name = "worldctl",
    about = "Validate, execute, and evaluate RoboKit executable worlds",
    version
)]
pub(crate) struct Cli {
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
    /// Validate an immutable JSON run manifest and its artifacts.
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
    /// Execute a world with the pinned `MuJoCo` worker and write an immutable run bundle.
    SimulateMujoco {
        /// Path to the canonical world TOML file.
        #[arg(long)]
        world: PathBuf,
        /// Optional binding path; defaults to `mujoco.toml` beside the world.
        #[arg(long)]
        binding: Option<PathBuf>,
        /// Directory containing the pinned Python `MuJoCo` project.
        #[arg(long, default_value = "adapters/mujoco/python")]
        adapter_project: PathBuf,
        /// New output directory for the immutable run bundle.
        #[arg(long)]
        output: PathBuf,
        /// Reproduction seed.
        #[arg(long, default_value_t = 7)]
        seed: u64,
        /// One-based delivery attempt.
        #[arg(long, default_value_t = 1)]
        attempt: u32,
        /// Maximum permitted `MuJoCo` steps.
        #[arg(long, default_value_t = 10_000)]
        maximum_steps: u64,
        /// Worker wall-clock timeout in seconds.
        #[arg(long, default_value_t = 30.0)]
        timeout_seconds: f64,
    },
    /// Fit a granular process model from physical observations and publish a new world package.
    CalibrateGranular {
        /// Path to the source world TOML file executed by physical runs.
        #[arg(long)]
        world: PathBuf,
        /// Path to the typed physical calibration dataset.
        #[arg(long)]
        dataset: PathBuf,
        /// New semantic version for the calibrated world.
        #[arg(long)]
        new_version: String,
        /// New output directory for the self-contained calibrated world package.
        #[arg(long)]
        output: PathBuf,
    },
    /// Verify calibration provenance and deterministically reproduce its fitted world.
    ValidateCalibrationPackage {
        /// Path to the calibrated world package directory.
        #[arg(long)]
        package: PathBuf,
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
    CalibrationDataset,
    CalibrationReport,
    World,
    RunManifest,
    RunOutcome,
}

pub(crate) fn run(cli: Cli) -> Result<(), CliError> {
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
        Command::SimulateMujoco {
            world,
            binding,
            adapter_project,
            output,
            seed,
            attempt,
            maximum_steps,
            timeout_seconds,
        } => {
            simulate_mujoco(SimulationOptions {
                world_path: world,
                binding_path: binding,
                adapter_project,
                output_directory: output.clone(),
                seed,
                attempt,
                maximum_steps,
                timeout: Duration::try_from_secs_f64(timeout_seconds)
                    .map_err(|error| CliError::InvalidSimulationOption(error.to_string()))?,
            })?;
            println!("completed MuJoCo run: {}", output.display());
        }
        Command::CalibrateGranular {
            world,
            dataset,
            new_version,
            output,
        } => {
            calibrate_granular(CalibrationOptions {
                world_path: world,
                dataset_path: dataset,
                new_version,
                output_directory: output.clone(),
            })?;
            println!("published calibrated world: {}", output.display());
        }
        Command::ValidateCalibrationPackage { package } => {
            validate_calibration_package(&package)?;
            println!("valid calibration package: {}", package.display());
        }
        Command::Schema { kind, output } => {
            let schema = match kind {
                SchemaKind::CalibrationDataset => {
                    serde_json::to_vec_pretty(&schema_for!(GranularCalibrationDataset))?
                }
                SchemaKind::CalibrationReport => {
                    serde_json::to_vec_pretty(&schema_for!(CalibrationReport))?
                }
                SchemaKind::World => serde_json::to_vec_pretty(&schema_for!(WorldDefinition))?,
                SchemaKind::RunManifest => serde_json::to_vec_pretty(&schema_for!(RunManifest))?,
                SchemaKind::RunOutcome => serde_json::to_vec_pretty(&schema_for!(RunOutcome))?,
            };
            write_bytes(output.as_deref(), &schema)?;
        }
    }
    Ok(())
}

fn format_manifest_issues(error: &evidence::ManifestValidationError) -> String {
    error
        .issues()
        .iter()
        .map(|issue| format!("{} ({:?}): {}", issue.path, issue.code, issue.message))
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_generation_supports_every_contract() {
        for kind in [
            SchemaKind::CalibrationDataset,
            SchemaKind::CalibrationReport,
            SchemaKind::World,
            SchemaKind::RunManifest,
            SchemaKind::RunOutcome,
        ] {
            let command = Command::Schema { kind, output: None };
            assert!(matches!(command, Command::Schema { .. }));
        }
    }
}
