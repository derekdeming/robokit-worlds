use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use world_domain::{MeasurementId, ValidatedWorldDefinition, VerifierDefinition};

use crate::RunOutcome;

/// Stable category of an executed verifier.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifierKind {
    /// Inclusive gravimetric tolerance.
    MassWithinTolerance,
    /// Forbidden event absence.
    ForbiddenEventAbsent,
    /// End-to-end duration bound.
    MaximumDuration,
}

/// Result of one verifier in world-definition order.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct VerifierResult {
    /// Stable verifier category.
    pub verifier: VerifierKind,
    /// Whether the outcome satisfied the criterion.
    pub passed: bool,
    /// Deterministic human-readable explanation.
    pub message: String,
}

/// Complete deterministic evaluation of one outcome.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct EvaluationReport {
    /// True only when every verifier passed.
    pub passed: bool,
    /// Verifier results in world-definition order.
    pub results: Vec<VerifierResult>,
}

/// Invalid outcome that prevents meaningful evaluation.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum EvaluationError {
    /// Outcome contains repeated identifiers.
    #[error("outcome contains duplicate measurement `{0}`")]
    DuplicateMeasurement(MeasurementId),
    /// Required measurement is absent.
    #[error("outcome is missing required measurement `{0}`")]
    MissingMeasurement(MeasurementId),
    /// Measurement or duration is non-finite or outside its permitted range.
    #[error("outcome contains invalid numeric value at `{0}`")]
    InvalidNumericValue(String),
}

/// Evaluates one typed outcome against every verifier in a validated world.
///
/// # Errors
///
/// Returns a typed error when the outcome is incomplete, ambiguous, or numerically invalid.
pub fn evaluate_outcome(
    world: &ValidatedWorldDefinition,
    outcome: &RunOutcome,
) -> Result<EvaluationReport, EvaluationError> {
    if !outcome.duration.get().is_finite() || outcome.duration.get() < 0.0 {
        return Err(EvaluationError::InvalidNumericValue("duration".to_owned()));
    }

    let mut measurements = HashMap::new();
    for measurement in &outcome.mass_measurements {
        if !measurement.value.get().is_finite() || measurement.value.get() < 0.0 {
            return Err(EvaluationError::InvalidNumericValue(format!(
                "mass_measurements.{}",
                measurement.measurement_id
            )));
        }
        if measurements
            .insert(measurement.measurement_id.clone(), measurement.value)
            .is_some()
        {
            return Err(EvaluationError::DuplicateMeasurement(
                measurement.measurement_id.clone(),
            ));
        }
    }

    let mut results = Vec::with_capacity(world.definition().verifiers.len());
    for verifier in &world.definition().verifiers {
        results.push(evaluate_verifier(verifier, outcome, &measurements)?);
    }
    Ok(EvaluationReport {
        passed: results.iter().all(|result| result.passed),
        results,
    })
}

fn evaluate_verifier(
    verifier: &VerifierDefinition,
    outcome: &RunOutcome,
    measurements: &HashMap<MeasurementId, world_domain::Grams>,
) -> Result<VerifierResult, EvaluationError> {
    match verifier {
        VerifierDefinition::MassWithinTolerance {
            measurement_id,
            target,
            tolerance,
        } => {
            let measured = measurements
                .get(measurement_id)
                .ok_or_else(|| EvaluationError::MissingMeasurement(measurement_id.clone()))?;
            let lower = target.get() - tolerance.get();
            let upper = target.get() + tolerance.get();
            let passed = (lower..=upper).contains(&measured.get());
            Ok(VerifierResult {
                verifier: VerifierKind::MassWithinTolerance,
                passed,
                message: format!(
                    "measurement `{measurement_id}` was {:.6} g; required [{lower:.6}, {upper:.6}] g",
                    measured.get()
                ),
            })
        }
        VerifierDefinition::ForbiddenEventAbsent { event } => {
            let passed = !outcome.events.contains(event);
            Ok(VerifierResult {
                verifier: VerifierKind::ForbiddenEventAbsent,
                passed,
                message: format!("forbidden event `{event:?}` present: {}", !passed),
            })
        }
        VerifierDefinition::MaximumDuration { limit } => {
            let passed = outcome.duration.get() <= limit.get();
            Ok(VerifierResult {
                verifier: VerifierKind::MaximumDuration,
                passed,
                message: format!(
                    "duration was {:.6} s; maximum {:.6} s",
                    outcome.duration.get(),
                    limit.get()
                ),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use world_domain::{
        EntityId, EntityKind, EventKind, Grams, InstrumentKind, MeasurementId, Meters, Pose,
        Position3, ProtocolAction, ProtocolStep, ProtocolStepId, Quaternion, SchemaVersion,
        Seconds, TaskDefinition, TaskId, VerifierDefinition, WorldDefinition, WorldEntity, WorldId,
        WorldVersion,
    };

    use super::*;
    use crate::MassMeasurement;

    fn world() -> ValidatedWorldDefinition {
        let scale_id = EntityId::new("scale");
        WorldDefinition {
            schema_version: SchemaVersion::V1,
            id: WorldId::from_uuid(Uuid::nil()),
            version: WorldVersion::new("0.1.0"),
            name: "evaluation test".to_owned(),
            entities: vec![WorldEntity {
                id: scale_id.clone(),
                name: "scale".to_owned(),
                entity: EntityKind::Instrument {
                    instrument: InstrumentKind::Scale,
                },
                initial_pose: Pose {
                    position: Position3 {
                        x: Meters::new(0.0),
                        y: Meters::new(0.0),
                        z: Meters::new(0.0),
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        w: 1.0,
                    },
                },
                asset: None,
            }],
            task: TaskDefinition {
                id: TaskId::new("dose"),
                name: "dose".to_owned(),
                steps: vec![ProtocolStep {
                    id: ProtocolStepId::new("measure"),
                    action: ProtocolAction::MeasureMass {
                        instrument_id: scale_id,
                        measurement_id: MeasurementId::new("transferred_mass"),
                    },
                }],
            },
            parameters: vec![],
            verifiers: vec![
                VerifierDefinition::MassWithinTolerance {
                    measurement_id: MeasurementId::new("transferred_mass"),
                    target: Grams::new(100.0),
                    tolerance: Grams::new(2.0),
                },
                VerifierDefinition::ForbiddenEventAbsent {
                    event: EventKind::Spill,
                },
            ],
        }
        .validate()
        .expect("test world should be valid")
    }

    #[test]
    fn passing_outcome_satisfies_all_verifiers() {
        let outcome = RunOutcome {
            mass_measurements: vec![MassMeasurement {
                measurement_id: MeasurementId::new("transferred_mass"),
                value: Grams::new(100.5),
            }],
            events: vec![],
            duration: Seconds::new(5.0),
        };
        let report = evaluate_outcome(&world(), &outcome).expect("outcome should be valid");
        assert!(report.passed);
    }

    #[test]
    fn spill_fails_without_changing_mass_result() {
        let outcome = RunOutcome {
            mass_measurements: vec![MassMeasurement {
                measurement_id: MeasurementId::new("transferred_mass"),
                value: Grams::new(100.5),
            }],
            events: vec![EventKind::Spill],
            duration: Seconds::new(5.0),
        };
        let report = evaluate_outcome(&world(), &outcome).expect("outcome should be valid");
        assert!(!report.passed);
        assert!(report.results[0].passed);
        assert!(!report.results[1].passed);
    }
}
