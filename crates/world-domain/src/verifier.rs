use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Grams, MeasurementId, Seconds};

/// Runtime event with scientific or operational significance.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Material left its permitted vessels or containment region.
    Spill,
    /// Unexpected physical collision.
    Collision,
    /// Runtime or external controller invoked a safety stop.
    SafetyStop,
    /// Required instrument reading did not settle.
    UnstableMeasurement,
}

/// Quantitative success or failure criterion.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "verifier", rename_all = "snake_case")]
pub enum VerifierDefinition {
    /// Require a mass measurement to fall inside an inclusive tolerance.
    MassWithinTolerance {
        /// Measurement produced by the protocol.
        measurement_id: MeasurementId,
        /// Desired mass.
        target: Grams,
        /// Symmetric tolerance around the target.
        tolerance: Grams,
    },
    /// Require that a failure event did not occur.
    ForbiddenEventAbsent {
        /// Event whose presence fails the run.
        event: EventKind,
    },
    /// Require completion within a logical task duration.
    MaximumDuration {
        /// Inclusive duration limit.
        limit: Seconds,
    },
}
