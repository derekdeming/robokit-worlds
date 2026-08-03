use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use world_domain::{PackagePath, RunId, Sha256Digest, WorldId, WorldVersion};

/// Version of the serialized run-manifest contract.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub enum RunManifestSchemaVersion {
    /// Initial immutable run-manifest contract.
    #[serde(rename = "1.0.0")]
    V1,
}

/// Whether evidence came from a simulator or physical system.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    /// Simulator execution.
    Simulation,
    /// Physical robot or instrument execution.
    Physical,
}

/// Exact world revision used for a run.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct WorldRevision {
    /// Stable world identity.
    pub id: WorldId,
    /// Semantic world version.
    pub version: WorldVersion,
    /// Digest of the serialized canonical world definition.
    pub definition_sha256: Sha256Digest,
}

/// Exact runtime implementation used for execution.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct RuntimeIdentity {
    /// Adapter implementation name.
    pub adapter_name: String,
    /// Simulator, robot controller, or instrument runtime name.
    pub engine_name: String,
    /// Semantic runtime version.
    pub engine_version: String,
}

/// Semantic role of an external run artifact.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRole {
    /// Time-ordered command and sensor telemetry.
    Telemetry,
    /// Visual recording.
    Video,
    /// Derived typed run outcome.
    Outcome,
    /// Verifier execution report.
    Evaluation,
    /// Calibration or runtime diagnostics.
    Diagnostic,
}

/// Permitted artifact content types.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactMediaType {
    /// MCAP robotics time series.
    Mcap,
    /// MPEG-4 video.
    Mp4,
    /// Apache Parquet table.
    Parquet,
    /// JSON document.
    Json,
}

/// Immutable external artifact produced by one run.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct ArtifactReference {
    /// Path relative to the run bundle root.
    pub relative_path: PackagePath,
    /// Semantic artifact role.
    pub role: ArtifactRole,
    /// Concrete content type.
    pub media_type: ArtifactMediaType,
    /// Exact content digest.
    pub sha256: Sha256Digest,
    /// Exact byte length.
    pub byte_length: u64,
}

/// Immutable provenance for one execution.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct RunManifest {
    /// Serialized-contract version.
    pub schema_version: RunManifestSchemaVersion,
    /// Globally unique run identifier.
    pub run_id: RunId,
    /// Physical or simulated source.
    pub run_kind: RunKind,
    /// Exact world revision.
    pub world: WorldRevision,
    /// Exact runtime implementation.
    pub runtime: RuntimeIdentity,
    /// Reproduction seed used by the runtime.
    pub seed: u64,
    /// Inclusive start timestamp.
    #[schemars(with = "String")]
    pub started_at: OffsetDateTime,
    /// Inclusive completion timestamp.
    #[schemars(with = "String")]
    pub finished_at: OffsetDateTime,
    /// Content-addressed outputs.
    pub artifacts: Vec<ArtifactReference>,
}

impl RunManifest {
    /// Validates provenance and path invariants.
    ///
    /// # Errors
    ///
    /// Returns every detected manifest issue.
    pub fn validate(self) -> Result<ValidatedRunManifest, ManifestValidationError> {
        let issues = self.validation_issues();
        if issues.is_empty() {
            Ok(ValidatedRunManifest(self))
        } else {
            Err(ManifestValidationError { issues })
        }
    }

    /// Reports every validation issue without consuming the manifest.
    pub fn validation_issues(&self) -> Vec<ManifestValidationIssue> {
        let mut issues = Vec::new();
        if self.finished_at < self.started_at {
            issues.push(ManifestValidationIssue::new(
                "finished_at",
                ManifestValidationCode::InvalidTimeRange,
                "finished_at must not precede started_at",
            ));
        }
        if !self.world.version.is_valid_semver() {
            issues.push(ManifestValidationIssue::new(
                "world.version",
                ManifestValidationCode::InvalidVersion,
                "world version must be valid semantic versioning",
            ));
        }
        if !self.world.definition_sha256.is_well_formed() {
            issues.push(ManifestValidationIssue::new(
                "world.definition_sha256",
                ManifestValidationCode::InvalidDigest,
                "world digest must be canonical lowercase SHA-256",
            ));
        }
        if self.runtime.adapter_name.trim().is_empty() || self.runtime.engine_name.trim().is_empty()
        {
            issues.push(ManifestValidationIssue::new(
                "runtime",
                ManifestValidationCode::EmptyValue,
                "adapter and engine names must not be empty",
            ));
        }
        if semver::Version::parse(&self.runtime.engine_version).is_err() {
            issues.push(ManifestValidationIssue::new(
                "runtime.engine_version",
                ManifestValidationCode::InvalidVersion,
                "engine version must be valid semantic versioning",
            ));
        }

        let mut paths = HashSet::new();
        for (index, artifact) in self.artifacts.iter().enumerate() {
            let base = format!("artifacts[{index}]");
            if !artifact.relative_path.is_well_formed() {
                issues.push(ManifestValidationIssue::new(
                    format!("{base}.relative_path"),
                    ManifestValidationCode::UnsafePath,
                    "artifact path must remain inside the run bundle",
                ));
            }
            if !paths.insert(artifact.relative_path.as_str()) {
                issues.push(ManifestValidationIssue::new(
                    format!("{base}.relative_path"),
                    ManifestValidationCode::DuplicatePath,
                    "artifact path is duplicated",
                ));
            }
            if !artifact.sha256.is_well_formed() {
                issues.push(ManifestValidationIssue::new(
                    format!("{base}.sha256"),
                    ManifestValidationCode::InvalidDigest,
                    "artifact digest must be canonical lowercase SHA-256",
                ));
            }
        }
        issues
    }
}

/// Manifest whose provenance and path invariants have been checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedRunManifest(RunManifest);

impl ValidatedRunManifest {
    /// Borrows the validated manifest.
    pub const fn manifest(&self) -> &RunManifest {
        &self.0
    }

    /// Consumes the wrapper.
    pub fn into_manifest(self) -> RunManifest {
        self.0
    }
}

/// Stable category for an invalid run manifest.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestValidationCode {
    /// Completion precedes start.
    InvalidTimeRange,
    /// Version does not use semantic versioning.
    InvalidVersion,
    /// Required string is empty.
    EmptyValue,
    /// Digest is not canonical SHA-256.
    InvalidDigest,
    /// Path is absolute or escapes the bundle.
    UnsafePath,
    /// Artifact path is repeated.
    DuplicatePath,
}

/// One invalid-manifest diagnostic.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct ManifestValidationIssue {
    /// Dot-separated location.
    pub path: String,
    /// Stable failure category.
    pub code: ManifestValidationCode,
    /// Human-readable explanation.
    pub message: String,
}

impl ManifestValidationIssue {
    fn new(
        path: impl Into<String>,
        code: ManifestValidationCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            code,
            message: message.into(),
        }
    }
}

/// Aggregated run-manifest validation failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("run manifest has {count} validation issue(s)", count = .issues.len())]
pub struct ManifestValidationError {
    issues: Vec<ManifestValidationIssue>,
}

impl ManifestValidationError {
    /// Returns every detected issue in deterministic traversal order.
    pub fn issues(&self) -> &[ManifestValidationIssue] {
        &self.issues
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;
    use uuid::Uuid;

    use super::*;

    fn manifest() -> RunManifest {
        RunManifest {
            schema_version: RunManifestSchemaVersion::V1,
            run_id: RunId::from_uuid(Uuid::nil()),
            run_kind: RunKind::Simulation,
            world: WorldRevision {
                id: WorldId::from_uuid(Uuid::nil()),
                version: WorldVersion::new("0.1.0"),
                definition_sha256: crate::sha256_digest(b"world"),
            },
            runtime: RuntimeIdentity {
                adapter_name: "mujoco".to_owned(),
                engine_name: "mujoco".to_owned(),
                engine_version: "3.6.0".to_owned(),
            },
            seed: 7,
            started_at: datetime!(2026-08-03 12:00 UTC),
            finished_at: datetime!(2026-08-03 12:01 UTC),
            artifacts: vec![ArtifactReference {
                relative_path: PackagePath::new("telemetry.mcap"),
                role: ArtifactRole::Telemetry,
                media_type: ArtifactMediaType::Mcap,
                sha256: crate::sha256_digest(b"telemetry"),
                byte_length: 9,
            }],
        }
    }

    #[test]
    fn valid_manifest_passes() {
        assert!(manifest().validate().is_ok());
    }

    #[test]
    fn traversal_and_reverse_time_are_rejected_together() {
        let mut manifest = manifest();
        manifest.finished_at = datetime!(2026-08-03 11:00 UTC);
        manifest.artifacts[0].relative_path = PackagePath::new("../outside.mcap");

        let error = manifest.validate().expect_err("manifest should be invalid");
        assert_eq!(error.issues().len(), 2);
    }
}
