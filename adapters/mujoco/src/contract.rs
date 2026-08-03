use std::path::PathBuf;

use runtime_api::{ExecutionLimits, ExecutionSummary, RuntimeEvent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use world_domain::{ParameterDistribution, ProtocolStep, RunId};

/// Version of the private Rust-to-Python adapter contract.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub enum AdapterContractSchemaVersion {
    /// Initial bounded execution contract.
    #[serde(rename = "1.0.0")]
    V1,
}

/// Hybrid granular-process parameters compiled from the canonical world.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct GranularProcessSpec {
    /// Effective bulk flow rate in grams per second.
    pub flow_rate: ParameterDistribution,
    /// Delay before material begins flowing.
    pub discharge_delay: ParameterDistribution,
    /// Material retained after pouring.
    pub residual_mass: ParameterDistribution,
    /// Standard deviation of scale noise.
    pub scale_noise_standard_deviation: ParameterDistribution,
    /// Required stable wait before measurement.
    pub scale_settling_time: ParameterDistribution,
    /// Initial source material mass.
    pub initial_material_mass: ParameterDistribution,
    /// Tilt angle considered a spill.
    pub spill_tilt_threshold: ParameterDistribution,
    /// Tilt angle at which flow begins.
    pub pour_onset_tilt_threshold: ParameterDistribution,
}

/// Fully compiled, bounded request consumed by the Python process.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct AdapterExecutionRequest {
    /// Private contract version.
    pub schema_version: AdapterContractSchemaVersion,
    /// Stable run identity and idempotency key.
    pub run_id: RunId,
    /// One-based delivery attempt.
    pub attempt: u32,
    /// Deterministic random seed.
    pub seed: u64,
    /// Mandatory execution bounds.
    pub limits: ExecutionLimits,
    /// Absolute path to the verified MJCF scene.
    #[schemars(with = "String")]
    pub scene_path: PathBuf,
    /// Name of the controlled `MuJoCo` mocap body.
    pub controlled_body: String,
    /// Geometry names whose contact with the controlled body is a collision.
    pub forbidden_contact_geometries: Vec<String>,
    /// Ordered, engine-neutral protocol compiled for this adapter.
    pub protocol: Vec<ProtocolStep>,
    /// Scientific process dynamics separate from rigid-body physics.
    pub process: GranularProcessSpec,
}

/// Successful response produced atomically by the Python process.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct AdapterExecutionResponse {
    /// Private contract version.
    pub schema_version: AdapterContractSchemaVersion,
    /// Exact `MuJoCo` package version used by the process.
    pub engine_version: String,
    /// Ordered typed runtime events.
    pub events: Vec<RuntimeEvent>,
    /// Bounded execution accounting.
    pub summary: ExecutionSummary,
}
