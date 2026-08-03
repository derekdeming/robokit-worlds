use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{EntityId, MeasurementId, Pose, ProtocolStepId, Radians, Seconds, TaskId};

/// Principal rotation axis.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    /// X axis.
    X,
    /// Y axis.
    Y,
    /// Z axis.
    Z,
}

/// Typed action that a runtime must support or explicitly reject.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ProtocolAction {
    /// Move one actor to a world-frame pose over a bounded duration.
    MoveToPose {
        /// Robot or controlled entity.
        actor_id: EntityId,
        /// Desired terminal pose.
        target: Pose,
        /// Commanded duration.
        duration: Seconds,
    },
    /// Rotate one actor around a principal axis.
    RotateAboutAxis {
        /// Robot or controlled entity.
        actor_id: EntityId,
        /// Rotation axis in the actor frame.
        axis: Axis,
        /// Signed rotation angle.
        angle: Radians,
        /// Commanded duration.
        duration: Seconds,
    },
    /// Allow dynamics or measurements to settle.
    Wait {
        /// Wait duration.
        duration: Seconds,
    },
    /// Produce a mass measurement.
    MeasureMass {
        /// Scale or balance entity.
        instrument_id: EntityId,
        /// Stable output identifier consumed by verifiers.
        measurement_id: MeasurementId,
    },
}

impl ProtocolAction {
    pub(crate) fn referenced_entities(&self) -> impl Iterator<Item = &EntityId> {
        match self {
            Self::MoveToPose { actor_id, .. } | Self::RotateAboutAxis { actor_id, .. } => {
                Some(actor_id).into_iter()
            }
            Self::MeasureMass { instrument_id, .. } => Some(instrument_id).into_iter(),
            Self::Wait { .. } => None.into_iter(),
        }
    }

    pub(crate) fn produced_measurement(&self) -> Option<&MeasurementId> {
        match self {
            Self::MeasureMass { measurement_id, .. } => Some(measurement_id),
            Self::MoveToPose { .. } | Self::RotateAboutAxis { .. } | Self::Wait { .. } => None,
        }
    }

    pub(crate) fn has_valid_values(&self) -> bool {
        match self {
            Self::MoveToPose {
                target, duration, ..
            } => {
                target.position.is_finite()
                    && target.orientation.is_normalized()
                    && duration.is_non_negative_finite()
                    && duration.get() > 0.0
            }
            Self::RotateAboutAxis {
                angle, duration, ..
            } => {
                angle.get().is_finite() && duration.is_non_negative_finite() && duration.get() > 0.0
            }
            Self::Wait { duration } => duration.is_non_negative_finite(),
            Self::MeasureMass { .. } => true,
        }
    }
}

/// One ordered protocol step.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct ProtocolStep {
    /// Stable step identifier.
    pub id: ProtocolStepId,
    /// Action executed at this step.
    #[serde(flatten)]
    pub action: ProtocolAction,
}

/// Ordered scientific task executed by a runtime.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct TaskDefinition {
    /// Stable task identifier.
    pub id: TaskId,
    /// Human-readable task name.
    pub name: String,
    /// Ordered actions.
    pub steps: Vec<ProtocolStep>,
}
