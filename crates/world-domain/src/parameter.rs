use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{EntityId, EntityKind, InstrumentKind, ParameterId};

/// Physical dimension attached to a parameter value.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterUnit {
    /// Pure scalar.
    Dimensionless,
    /// Grams.
    Grams,
    /// Grams per second.
    GramsPerSecond,
    /// Seconds.
    Seconds,
    /// Radians.
    Radians,
}

/// Supported calibratable property.
#[derive(
    Clone, Copy, Debug, Deserialize, Hash, JsonSchema, Ord, PartialEq, Eq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ParameterProperty {
    /// Contact friction coefficient.
    FrictionCoefficient,
    /// Effective bulk flow rate.
    FlowRate,
    /// Delay between commanded tilt and material discharge.
    DischargeDelay,
    /// Material mass remaining after the pour action.
    ResidualMass,
    /// Standard deviation of scale noise.
    ScaleNoiseStandardDeviation,
    /// Scale settling duration.
    ScaleSettlingTime,
    /// Material mass present before execution.
    InitialMaterialMass,
    /// Tilt angle above which containment is no longer trusted.
    SpillTiltThreshold,
    /// Tilt angle above which granular flow begins.
    PourOnsetTiltThreshold,
}

impl ParameterProperty {
    pub(crate) const fn expected_unit(self) -> ParameterUnit {
        match self {
            Self::FrictionCoefficient => ParameterUnit::Dimensionless,
            Self::FlowRate => ParameterUnit::GramsPerSecond,
            Self::DischargeDelay | Self::ScaleSettlingTime => ParameterUnit::Seconds,
            Self::ResidualMass | Self::ScaleNoiseStandardDeviation | Self::InitialMaterialMass => {
                ParameterUnit::Grams
            }
            Self::SpillTiltThreshold | Self::PourOnsetTiltThreshold => ParameterUnit::Radians,
        }
    }

    pub(crate) const fn supports_entity(self, entity: EntityKind) -> bool {
        match self {
            Self::FrictionCoefficient => {
                matches!(entity, EntityKind::Vessel { .. } | EntityKind::WorkSurface)
            }
            Self::FlowRate | Self::DischargeDelay | Self::InitialMaterialMass => {
                matches!(entity, EntityKind::Material { .. })
            }
            Self::ResidualMass | Self::SpillTiltThreshold | Self::PourOnsetTiltThreshold => {
                matches!(entity, EntityKind::Vessel { .. })
            }
            Self::ScaleNoiseStandardDeviation | Self::ScaleSettlingTime => matches!(
                entity,
                EntityKind::Instrument {
                    instrument: InstrumentKind::Scale
                }
            ),
        }
    }
}

/// Semantic owner of a calibratable parameter.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct ParameterTarget {
    /// Entity whose behavior the parameter modifies.
    pub entity_id: EntityId,
    /// Explicit modeled property.
    pub property: ParameterProperty,
}

/// Probability distribution representing parameter uncertainty.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "distribution", rename_all = "snake_case")]
pub enum ParameterDistribution {
    /// Known scalar value.
    Fixed {
        /// Fixed value.
        value: f64,
    },
    /// Bounded uniform uncertainty.
    Uniform {
        /// Inclusive lower bound.
        minimum: f64,
        /// Inclusive upper bound.
        maximum: f64,
    },
    /// Gaussian uncertainty.
    Normal {
        /// Distribution mean.
        mean: f64,
        /// Strictly positive standard deviation.
        standard_deviation: f64,
    },
}

impl ParameterDistribution {
    pub(crate) fn is_valid(self) -> bool {
        match self {
            Self::Fixed { value } => value.is_finite(),
            Self::Uniform { minimum, maximum } => {
                minimum.is_finite() && maximum.is_finite() && minimum < maximum
            }
            Self::Normal {
                mean,
                standard_deviation,
            } => mean.is_finite() && standard_deviation.is_finite() && standard_deviation > 0.0,
        }
    }
}

/// One uncertain parameter that calibration may update.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct ParameterDefinition {
    /// Stable parameter identifier.
    pub id: ParameterId,
    /// Entity and property affected by the parameter.
    pub target: ParameterTarget,
    /// Physical dimension.
    pub unit: ParameterUnit,
    /// Current uncertainty representation.
    pub distribution: ParameterDistribution,
}
