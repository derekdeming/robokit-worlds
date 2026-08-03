use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use world_domain::{
    EventKind, Grams, Radians, RunId, Seconds, Sha256Digest, ValidatedWorldDefinition, WorldId,
    WorldVersion,
};

use crate::{DatasetValidationError, DatasetValidationIssue};

const MINIMUM_OBSERVATIONS: usize = 4;
const MAXIMUM_OBSERVATIONS: usize = 10_000;

/// Version of the physical granular-calibration dataset contract.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub enum CalibrationDatasetSchemaVersion {
    /// Initial content-addressed physical-observation contract.
    #[serde(rename = "1.0.0")]
    V1,
}

/// Exact source-world revision used by every physical observation.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct CalibrationWorldRevision {
    /// Stable world identity.
    pub id: WorldId,
    /// Exact semantic world version.
    pub version: WorldVersion,
    /// SHA-256 of the source world definition bytes.
    pub definition_sha256: Sha256Digest,
}

/// One traceable, settled physical granular-dosing observation.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct GranularDosingObservation {
    /// Physical run identity.
    pub run_id: RunId,
    /// Digest of the immutable physical run manifest.
    pub source_manifest_sha256: Sha256Digest,
    /// Commanded duration at the pour angle.
    pub hold_duration: Seconds,
    /// Commanded source-vessel tilt angle.
    pub tilt_angle: Radians,
    /// Settled transferred-mass reading.
    pub transferred_mass: Grams,
    /// Whether the scale met its physical settling criterion.
    pub scale_settled: bool,
    /// Events observed by the physical runtime.
    pub events: Vec<EventKind>,
}

/// Batch-homogeneous physical observations used for one calibration.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GranularCalibrationDataset {
    /// Serialized-contract version.
    pub schema_version: CalibrationDatasetSchemaVersion,
    /// Exact world revision executed by the physical runs.
    pub world: CalibrationWorldRevision,
    /// Material batch identity; calibration does not pool batches implicitly.
    pub material_batch_id: String,
    /// Independent physical observations.
    pub observations: Vec<GranularDosingObservation>,
}

impl GranularCalibrationDataset {
    /// Validates provenance, observation quality, and source-world identity.
    ///
    /// # Errors
    ///
    /// Returns every dataset issue in deterministic traversal order.
    pub fn validate_for_world(
        self,
        world: &ValidatedWorldDefinition,
        definition_sha256: &Sha256Digest,
    ) -> Result<ValidatedGranularCalibrationDataset, DatasetValidationError> {
        let mut issues = Vec::new();
        self.validate_header(world, definition_sha256, &mut issues);
        self.validate_observations(&mut issues);
        if issues.is_empty() {
            Ok(ValidatedGranularCalibrationDataset(self))
        } else {
            Err(DatasetValidationError::new(issues))
        }
    }

    fn validate_header(
        &self,
        world: &ValidatedWorldDefinition,
        definition_sha256: &Sha256Digest,
        issues: &mut Vec<DatasetValidationIssue>,
    ) {
        let definition = world.definition();
        if self.world.id != definition.id {
            issues.push(DatasetValidationIssue::new(
                "world.id",
                "dataset world id does not match the source world",
            ));
        }
        if self.world.version != definition.version {
            issues.push(DatasetValidationIssue::new(
                "world.version",
                "dataset world version does not match the source world",
            ));
        }
        if self.world.definition_sha256 != *definition_sha256 {
            issues.push(DatasetValidationIssue::new(
                "world.definition_sha256",
                "dataset world digest does not match the source world bytes",
            ));
        }
        if !self.world.definition_sha256.is_well_formed() {
            issues.push(DatasetValidationIssue::new(
                "world.definition_sha256",
                "source world digest must be canonical lowercase SHA-256",
            ));
        }
        if self.material_batch_id.trim().is_empty() || self.material_batch_id.len() > 128 {
            issues.push(DatasetValidationIssue::new(
                "material_batch_id",
                "material batch id must contain 1-128 characters",
            ));
        }
        if self.observations.len() < MINIMUM_OBSERVATIONS {
            issues.push(DatasetValidationIssue::new(
                "observations",
                format!("at least {MINIMUM_OBSERVATIONS} observations are required"),
            ));
        }
        if self.observations.len() > MAXIMUM_OBSERVATIONS {
            issues.push(DatasetValidationIssue::new(
                "observations",
                format!("at most {MAXIMUM_OBSERVATIONS} observations are permitted"),
            ));
        }
    }

    fn validate_observations(&self, issues: &mut Vec<DatasetValidationIssue>) {
        let mut run_ids = HashSet::new();
        let mut manifest_digests = HashSet::new();
        let reference_tilt = self
            .observations
            .first()
            .map(|observation| observation.tilt_angle.get());
        for (index, observation) in self.observations.iter().enumerate() {
            let base = format!("observations[{index}]");
            if !run_ids.insert(observation.run_id) {
                issues.push(DatasetValidationIssue::new(
                    format!("{base}.run_id"),
                    "physical run id is duplicated",
                ));
            }
            if !observation.source_manifest_sha256.is_well_formed() {
                issues.push(DatasetValidationIssue::new(
                    format!("{base}.source_manifest_sha256"),
                    "manifest digest must be canonical lowercase SHA-256",
                ));
            }
            if !manifest_digests.insert(observation.source_manifest_sha256.as_str()) {
                issues.push(DatasetValidationIssue::new(
                    format!("{base}.source_manifest_sha256"),
                    "physical run manifest digest is duplicated",
                ));
            }
            if !observation.hold_duration.is_non_negative_finite()
                || observation.hold_duration.get() == 0.0
            {
                issues.push(DatasetValidationIssue::new(
                    format!("{base}.hold_duration"),
                    "hold duration must be positive and finite",
                ));
            }
            if !observation.tilt_angle.get().is_finite() || observation.tilt_angle.get() <= 0.0 {
                issues.push(DatasetValidationIssue::new(
                    format!("{base}.tilt_angle"),
                    "tilt angle must be positive and finite",
                ));
            }
            if reference_tilt
                .is_some_and(|reference| (observation.tilt_angle.get() - reference).abs() > 1.0e-9)
            {
                issues.push(DatasetValidationIssue::new(
                    format!("{base}.tilt_angle"),
                    "v1 calibration requires one fixed tilt angle across all trials",
                ));
            }
            if !observation.transferred_mass.is_non_negative_finite() {
                issues.push(DatasetValidationIssue::new(
                    format!("{base}.transferred_mass"),
                    "transferred mass must be non-negative and finite",
                ));
            }
            if !observation.scale_settled {
                issues.push(DatasetValidationIssue::new(
                    format!("{base}.scale_settled"),
                    "unsettled scale observations cannot calibrate mass transfer",
                ));
            }
            if !observation.events.is_empty() {
                issues.push(DatasetValidationIssue::new(
                    format!("{base}.events"),
                    "eventful physical runs require a separate failure-model calibration",
                ));
            }
        }
    }
}

/// Physical dataset whose identity and observation invariants have been checked.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedGranularCalibrationDataset(GranularCalibrationDataset);

impl ValidatedGranularCalibrationDataset {
    /// Borrows the validated dataset.
    pub const fn dataset(&self) -> &GranularCalibrationDataset {
        &self.0
    }
}
