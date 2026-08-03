use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use world_domain::{PackagePath, Sha256Digest, WorldId, WorldVersion};

use crate::GranularFit;

/// Version of the immutable calibration-report contract.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub enum CalibrationReportSchemaVersion {
    /// Initial granular ordinary-least-squares report.
    #[serde(rename = "1.0.0")]
    V1,
}

/// Stable calibration algorithm identity.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationMethod {
    /// Two-parameter linear flow/delay fit with residual scale-noise estimation.
    GranularLinearFlowV1,
}

/// Exact calibrated-world revision produced by a report.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct CalibratedWorldRevision {
    /// Stable world identity.
    pub id: WorldId,
    /// New semantic version.
    pub version: WorldVersion,
    /// SHA-256 of the calibrated world definition bytes.
    pub definition_sha256: Sha256Digest,
}

/// Immutable provenance and quantitative result for one calibration.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct CalibrationReport {
    /// Serialized-contract version.
    pub schema_version: CalibrationReportSchemaVersion,
    /// Exact algorithm implementation.
    pub method: CalibrationMethod,
    /// Source world used by physical runs.
    pub source_world: crate::CalibrationWorldRevision,
    /// Source world definition path relative to the calibrated package.
    pub source_world_path: PackagePath,
    /// Newly published calibrated world.
    pub calibrated_world: CalibratedWorldRevision,
    /// Dataset path relative to the calibrated world package.
    pub source_dataset_path: PackagePath,
    /// Exact dataset byte digest.
    pub source_dataset_sha256: Sha256Digest,
    /// Material batch whose observations were pooled.
    pub material_batch_id: String,
    /// Number of independent physical runs used.
    pub observation_count: u64,
    /// Parameter estimates and goodness-of-fit metrics.
    pub fit: GranularFit,
}
