use std::{convert::Infallible, path::PathBuf, time::Duration};

use evaluation::{MassMeasurement, RunOutcome, evaluate_outcome};
use evidence::{
    ArtifactMediaType, ArtifactReference, ArtifactRole, RunKind, RunManifest,
    RunManifestSchemaVersion, RuntimeIdentity, WorldRevision, sha256_digest,
};
use mujoco_adapter::{MujocoAdapter, MujocoBinding, MujocoProcessCommand};
use runtime_api::{
    ExecutionLimits, ExecutionRequest, RuntimeAdapter, RuntimeEvent, RuntimeObserver,
};
use time::OffsetDateTime;
use world_domain::{PackagePath, RunId, Seconds};

use crate::{
    error::CliError,
    files::{
        StagedDirectory, canonicalize, load_world, pretty_json, read_bytes, read_text, write_file,
    },
};

const OUTCOME_FILE_NAME: &str = "outcome.json";
const EVALUATION_FILE_NAME: &str = "evaluation.json";
const MANIFEST_FILE_NAME: &str = "manifest.json";

/// Fully explicit options for one local simulation delivery.
pub(crate) struct SimulationOptions {
    pub(crate) world_path: PathBuf,
    pub(crate) binding_path: Option<PathBuf>,
    pub(crate) adapter_project: PathBuf,
    pub(crate) output_directory: PathBuf,
    pub(crate) seed: u64,
    pub(crate) attempt: u32,
    pub(crate) maximum_steps: u64,
    pub(crate) timeout: Duration,
}

pub(crate) fn simulate_mujoco(options: SimulationOptions) -> Result<(), CliError> {
    let output = StagedDirectory::create(options.output_directory)?;
    let world = load_world(&options.world_path)?;
    let world_bytes = read_bytes(&options.world_path)?;
    let package_root = canonicalize(
        options
            .world_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new(".")),
    )?;
    let binding_path = options
        .binding_path
        .unwrap_or_else(|| package_root.join("mujoco.toml"));
    let binding = MujocoBinding::from_toml(&read_text(&binding_path)?)?;
    let process = MujocoProcessCommand::uv(&canonicalize(&options.adapter_project)?);
    let mut adapter = MujocoAdapter::new(&package_root, binding, process);

    let run_id = RunId::new();
    let wall_clock_timeout = options.timeout.as_secs_f64();
    let request = ExecutionRequest {
        run_id,
        attempt: options.attempt,
        seed: options.seed,
        limits: ExecutionLimits {
            maximum_steps: options.maximum_steps,
            wall_clock_timeout: Seconds::new(wall_clock_timeout),
        },
    }
    .validate()?;
    let started_at = OffsetDateTime::now_utc();
    let mut collector = OutcomeCollector::default();
    let summary = adapter.execute(&world, &request, &mut collector)?;
    let finished_at = OffsetDateTime::now_utc();
    let outcome = collector.into_outcome(summary.logical_duration);
    let evaluation = evaluate_outcome(&world, &outcome)?;

    let outcome_bytes = pretty_json(&outcome)?;
    let evaluation_bytes = pretty_json(&evaluation)?;
    write_file(&output.path().join(OUTCOME_FILE_NAME), &outcome_bytes)?;
    write_file(&output.path().join(EVALUATION_FILE_NAME), &evaluation_bytes)?;

    let descriptor = &adapter.capabilities().descriptor;
    let manifest = RunManifest {
        schema_version: RunManifestSchemaVersion::V2,
        run_id,
        run_kind: RunKind::Simulation,
        world: WorldRevision {
            id: world.definition().id,
            version: world.definition().version.clone(),
            definition_sha256: sha256_digest(&world_bytes),
        },
        runtime: RuntimeIdentity {
            adapter_name: descriptor.adapter_name.clone(),
            adapter_version: descriptor.adapter_version.clone(),
            engine_name: descriptor.engine_name.clone(),
            engine_version: descriptor.engine_version.clone(),
        },
        seed: options.seed,
        started_at,
        finished_at,
        artifacts: vec![
            artifact_reference(OUTCOME_FILE_NAME, ArtifactRole::Outcome, &outcome_bytes)?,
            artifact_reference(
                EVALUATION_FILE_NAME,
                ArtifactRole::Evaluation,
                &evaluation_bytes,
            )?,
        ],
    };
    manifest
        .validation_issues()
        .first()
        .map_or(Ok(()), |issue| {
            Err(CliError::InvalidManifest(format!(
                "{} ({:?}): {}",
                issue.path, issue.code, issue.message
            )))
        })?;
    write_file(
        &output.path().join(MANIFEST_FILE_NAME),
        &pretty_json(&manifest)?,
    )?;
    output.publish()
}

#[derive(Default)]
struct OutcomeCollector {
    mass_measurements: Vec<MassMeasurement>,
    events: Vec<world_domain::EventKind>,
}

impl RuntimeObserver for OutcomeCollector {
    type Error = Infallible;

    fn observe(&mut self, event: RuntimeEvent) -> Result<(), Self::Error> {
        match event {
            RuntimeEvent::MassMeasurement {
                measurement_id,
                value,
            } => self.mass_measurements.push(MassMeasurement {
                measurement_id,
                value,
            }),
            RuntimeEvent::Event { event } => self.events.push(event),
        }
        Ok(())
    }
}

impl OutcomeCollector {
    fn into_outcome(self, duration: Seconds) -> RunOutcome {
        RunOutcome {
            mass_measurements: self.mass_measurements,
            events: self.events,
            duration,
        }
    }
}

fn artifact_reference(
    relative_path: &str,
    role: ArtifactRole,
    bytes: &[u8],
) -> Result<ArtifactReference, CliError> {
    Ok(ArtifactReference {
        relative_path: PackagePath::new(relative_path),
        role,
        media_type: ArtifactMediaType::Json,
        sha256: sha256_digest(bytes),
        byte_length: u64::try_from(bytes.len()).map_err(|_| CliError::ArtifactTooLarge {
            path: PathBuf::from(relative_path),
        })?,
    })
}
