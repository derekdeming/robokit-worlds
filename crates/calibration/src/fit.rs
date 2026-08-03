use schemars::JsonSchema;
use semver::Version;
use serde::{Deserialize, Serialize};
use world_domain::{
    ParameterDistribution, ParameterProperty, ValidatedWorldDefinition, WorldDefinition,
    WorldVersion,
};

use crate::{CalibrationError, ValidatedGranularCalibrationDataset};

const UNCERTAINTY_STANDARD_ERRORS: f64 = 2.0;
const NUMERIC_EPSILON: f64 = 1.0e-12;

struct RegressionMoments {
    count: f64,
    mean_duration: f64,
    mean_mass: f64,
    duration_variation: f64,
    covariance: f64,
}

struct ResidualStatistics {
    variance: f64,
    standard_deviation: f64,
    standard_deviation_standard_error: f64,
    root_mean_square_error: f64,
    coefficient_of_determination: f64,
}

/// One calibrated parameter estimate and its published uncertainty.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct ParameterEstimate {
    /// Fitted point estimate.
    pub point_estimate: f64,
    /// Analytic standard error under the fitted linear model.
    pub standard_error: f64,
    /// Distribution written to the calibrated world.
    pub calibrated_distribution: ParameterDistribution,
}

/// Fitted granular-process parameters and goodness-of-fit statistics.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct GranularFit {
    /// Effective flow rate in grams per second.
    pub flow_rate: ParameterEstimate,
    /// Flow-onset delay in seconds.
    pub discharge_delay: ParameterEstimate,
    /// Residual measurement noise in grams.
    pub scale_noise_standard_deviation: ParameterEstimate,
    /// Root-mean-square residual in grams.
    pub root_mean_square_error: f64,
    /// Fraction of transferred-mass variance explained by the model.
    pub coefficient_of_determination: f64,
}

impl GranularFit {
    /// Compares serialized fit values within a strict floating-point round-trip tolerance.
    pub fn numerically_matches(&self, other: &Self) -> bool {
        estimates_match(&self.flow_rate, &other.flow_rate)
            && estimates_match(&self.discharge_delay, &other.discharge_delay)
            && estimates_match(
                &self.scale_noise_standard_deviation,
                &other.scale_noise_standard_deviation,
            )
            && scalars_match(self.root_mean_square_error, other.root_mean_square_error)
            && scalars_match(
                self.coefficient_of_determination,
                other.coefficient_of_determination,
            )
    }
}

/// Validated calibrated world plus the fit that produced it.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibratedGranularWorld {
    world: ValidatedWorldDefinition,
    fit: GranularFit,
}

impl CalibratedGranularWorld {
    /// Borrows the calibrated world.
    pub const fn world(&self) -> &ValidatedWorldDefinition {
        &self.world
    }

    /// Borrows the fit result.
    pub const fn fit(&self) -> &GranularFit {
        &self.fit
    }

    /// Checks a serialized world while tolerating only float round-trip noise in fitted values.
    pub fn matches_definition(&self, candidate: &WorldDefinition) -> bool {
        let mut normalized = candidate.clone();
        for property in [
            ParameterProperty::FlowRate,
            ParameterProperty::DischargeDelay,
            ParameterProperty::ScaleNoiseStandardDeviation,
        ] {
            let Ok(expected) = unique_definition_distribution(self.world.definition(), property)
            else {
                return false;
            };
            let Ok(actual) = unique_definition_distribution(candidate, property) else {
                return false;
            };
            if !distributions_match(expected, actual)
                || replace_distribution(&mut normalized, property, expected).is_err()
            {
                return false;
            }
        }
        &normalized == self.world.definition()
    }
}

/// Fits flow, discharge delay, and scale noise and publishes a new world revision.
///
/// # Errors
///
/// Returns a typed error when observations do not identify the supported model or
/// the calibrated world would violate a shared domain invariant.
pub fn calibrate_granular_world(
    source_world: &ValidatedWorldDefinition,
    dataset: &ValidatedGranularCalibrationDataset,
    new_version: &str,
) -> Result<CalibratedGranularWorld, CalibrationError> {
    validate_new_version(source_world, new_version)?;
    validate_observation_regime(source_world, dataset)?;
    let fit = fit_linear_flow(dataset)?;
    let mut definition = source_world.definition().clone();
    definition.version = WorldVersion::new(new_version);
    replace_distribution(
        &mut definition,
        ParameterProperty::FlowRate,
        fit.flow_rate.calibrated_distribution,
    )?;
    replace_distribution(
        &mut definition,
        ParameterProperty::DischargeDelay,
        fit.discharge_delay.calibrated_distribution,
    )?;
    replace_distribution(
        &mut definition,
        ParameterProperty::ScaleNoiseStandardDeviation,
        fit.scale_noise_standard_deviation.calibrated_distribution,
    )?;
    let world = definition.validate().map_err(|errors| {
        CalibrationError::InvalidCalibratedWorld(
            errors
                .issues()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;
    Ok(CalibratedGranularWorld { world, fit })
}

fn validate_new_version(
    source_world: &ValidatedWorldDefinition,
    new_version: &str,
) -> Result<(), CalibrationError> {
    let source = Version::parse(source_world.definition().version.as_str())
        .map_err(|_| CalibrationError::InvalidWorldVersion)?;
    let calibrated =
        Version::parse(new_version).map_err(|_| CalibrationError::InvalidWorldVersion)?;
    if calibrated > source {
        Ok(())
    } else {
        Err(CalibrationError::InvalidWorldVersion)
    }
}

fn validate_observation_regime(
    world: &ValidatedWorldDefinition,
    dataset: &ValidatedGranularCalibrationDataset,
) -> Result<(), CalibrationError> {
    let onset = required_distribution(world, ParameterProperty::PourOnsetTiltThreshold)?;
    let spill = required_distribution(world, ParameterProperty::SpillTiltThreshold)?;
    let initial_mass = required_distribution(world, ParameterProperty::InitialMaterialMass)?;
    let residual_mass = required_distribution(world, ParameterProperty::ResidualMass)?;
    let onset_maximum = distribution_maximum(onset);
    let spill_minimum = distribution_minimum(spill);
    let transferable_capacity =
        distribution_minimum(initial_mass) - distribution_maximum(residual_mass);

    for (index, observation) in dataset.dataset().observations.iter().enumerate() {
        if observation.tilt_angle.get() <= onset_maximum {
            return Err(CalibrationError::InvalidObservationRegime {
                index,
                reason: "tilt does not exceed every modeled pour-onset value".to_owned(),
            });
        }
        if observation.tilt_angle.get() >= spill_minimum {
            return Err(CalibrationError::InvalidObservationRegime {
                index,
                reason: "tilt reaches the modeled spill regime".to_owned(),
            });
        }
        if observation.transferred_mass.get() >= transferable_capacity {
            return Err(CalibrationError::InvalidObservationRegime {
                index,
                reason: "transfer reaches source-capacity saturation".to_owned(),
            });
        }
    }
    Ok(())
}

fn fit_linear_flow(
    dataset: &ValidatedGranularCalibrationDataset,
) -> Result<GranularFit, CalibrationError> {
    let observations = &dataset.dataset().observations;
    let moments = regression_moments(observations)?;
    let flow_rate = moments.covariance / moments.duration_variation;
    let intercept = moments.mean_mass - (flow_rate * moments.mean_duration);
    let discharge_delay = -intercept / flow_rate;
    validate_positive_estimate(ParameterProperty::FlowRate, flow_rate)?;
    validate_non_negative_estimate(ParameterProperty::DischargeDelay, discharge_delay)?;
    let minimum_duration = observations
        .iter()
        .map(|observation| observation.hold_duration.get())
        .fold(f64::INFINITY, f64::min);
    if discharge_delay >= minimum_duration {
        return Err(CalibrationError::InvalidEstimate {
            property: ParameterProperty::DischargeDelay,
            value: discharge_delay,
        });
    }

    let residuals = residual_statistics(observations, &moments, flow_rate, intercept)?;
    let flow_standard_error = (residuals.variance / moments.duration_variation).sqrt();
    let delay_standard_error = calculate_delay_standard_error(
        &moments,
        residuals.variance,
        flow_rate,
        intercept,
        flow_standard_error,
    );
    Ok(GranularFit {
        flow_rate: ParameterEstimate {
            point_estimate: flow_rate,
            standard_error: flow_standard_error,
            calibrated_distribution: uncertainty_distribution(
                ParameterProperty::FlowRate,
                flow_rate,
                flow_standard_error,
                true,
            )?,
        },
        discharge_delay: ParameterEstimate {
            point_estimate: discharge_delay,
            standard_error: delay_standard_error,
            calibrated_distribution: uncertainty_distribution(
                ParameterProperty::DischargeDelay,
                discharge_delay,
                delay_standard_error,
                false,
            )?,
        },
        scale_noise_standard_deviation: ParameterEstimate {
            point_estimate: residuals.standard_deviation,
            standard_error: residuals.standard_deviation_standard_error,
            calibrated_distribution: uncertainty_distribution(
                ParameterProperty::ScaleNoiseStandardDeviation,
                residuals.standard_deviation,
                residuals.standard_deviation_standard_error,
                false,
            )?,
        },
        root_mean_square_error: residuals.root_mean_square_error,
        coefficient_of_determination: residuals.coefficient_of_determination,
    })
}

fn regression_moments(
    observations: &[crate::GranularDosingObservation],
) -> Result<RegressionMoments, CalibrationError> {
    let count = f64::from(
        u32::try_from(observations.len()).map_err(|_| CalibrationError::TooManyObservations)?,
    );
    let mean_duration = observations
        .iter()
        .map(|observation| observation.hold_duration.get())
        .sum::<f64>()
        / count;
    let mean_mass = observations
        .iter()
        .map(|observation| observation.transferred_mass.get())
        .sum::<f64>()
        / count;
    let duration_variation = observations
        .iter()
        .map(|observation| (observation.hold_duration.get() - mean_duration).powi(2))
        .sum::<f64>();
    if duration_variation <= NUMERIC_EPSILON {
        return Err(CalibrationError::InsufficientVariation);
    }
    let covariance = observations
        .iter()
        .map(|observation| {
            (observation.hold_duration.get() - mean_duration)
                * (observation.transferred_mass.get() - mean_mass)
        })
        .sum::<f64>();
    Ok(RegressionMoments {
        count,
        mean_duration,
        mean_mass,
        duration_variation,
        covariance,
    })
}

fn residual_statistics(
    observations: &[crate::GranularDosingObservation],
    moments: &RegressionMoments,
    flow_rate: f64,
    intercept: f64,
) -> Result<ResidualStatistics, CalibrationError> {
    let residual_sum_of_squares = observations
        .iter()
        .map(|observation| {
            let predicted = flow_rate.mul_add(observation.hold_duration.get(), intercept);
            (observation.transferred_mass.get() - predicted).powi(2)
        })
        .sum::<f64>();
    let total_sum_of_squares = observations
        .iter()
        .map(|observation| (observation.transferred_mass.get() - moments.mean_mass).powi(2))
        .sum::<f64>();
    if total_sum_of_squares <= NUMERIC_EPSILON {
        return Err(CalibrationError::InsufficientVariation);
    }
    let degrees_of_freedom = moments.count - 2.0;
    let residual_variance = residual_sum_of_squares / degrees_of_freedom;
    Ok(ResidualStatistics {
        variance: residual_variance,
        standard_deviation: residual_variance.sqrt(),
        standard_deviation_standard_error: residual_variance.sqrt()
            / (2.0 * degrees_of_freedom).sqrt(),
        root_mean_square_error: (residual_sum_of_squares / moments.count).sqrt(),
        coefficient_of_determination: 1.0 - (residual_sum_of_squares / total_sum_of_squares),
    })
}

fn calculate_delay_standard_error(
    moments: &RegressionMoments,
    residual_variance: f64,
    flow_rate: f64,
    intercept: f64,
    flow_standard_error: f64,
) -> f64 {
    let intercept_variance = residual_variance
        * ((1.0 / moments.count) + moments.mean_duration.powi(2) / moments.duration_variation);
    let intercept_flow_covariance =
        -moments.mean_duration * residual_variance / moments.duration_variation;
    let intercept_derivative = -1.0 / flow_rate;
    let flow_derivative = intercept / flow_rate.powi(2);
    let delay_variance = intercept_derivative.powi(2).mul_add(
        intercept_variance,
        flow_derivative.powi(2).mul_add(
            flow_standard_error.powi(2),
            2.0 * intercept_derivative * flow_derivative * intercept_flow_covariance,
        ),
    );
    delay_variance.max(0.0).sqrt()
}

fn uncertainty_distribution(
    property: ParameterProperty,
    point: f64,
    standard_error: f64,
    strictly_positive: bool,
) -> Result<ParameterDistribution, CalibrationError> {
    if standard_error <= NUMERIC_EPSILON {
        return Ok(ParameterDistribution::Fixed { value: point });
    }
    let half_width = UNCERTAINTY_STANDARD_ERRORS * standard_error;
    let minimum = point - half_width;
    let maximum = point + half_width;
    if (strictly_positive && minimum <= 0.0) || (!strictly_positive && minimum < 0.0) {
        return Err(CalibrationError::InvalidEstimate {
            property,
            value: minimum,
        });
    }
    Ok(ParameterDistribution::Uniform { minimum, maximum })
}

fn validate_positive_estimate(
    property: ParameterProperty,
    value: f64,
) -> Result<(), CalibrationError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(CalibrationError::InvalidEstimate { property, value })
    }
}

fn validate_non_negative_estimate(
    property: ParameterProperty,
    value: f64,
) -> Result<(), CalibrationError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(CalibrationError::InvalidEstimate { property, value })
    }
}

fn required_distribution(
    world: &ValidatedWorldDefinition,
    property: ParameterProperty,
) -> Result<ParameterDistribution, CalibrationError> {
    let mut parameters = world.parameters_with_property(property);
    let distribution = parameters
        .next()
        .ok_or(CalibrationError::MissingParameter(property))?
        .distribution;
    if parameters.next().is_some() {
        Err(CalibrationError::DuplicateParameter(property))
    } else {
        Ok(distribution)
    }
}

fn replace_distribution(
    definition: &mut WorldDefinition,
    property: ParameterProperty,
    distribution: ParameterDistribution,
) -> Result<(), CalibrationError> {
    let mut matches = definition
        .parameters
        .iter_mut()
        .filter(|parameter| parameter.target.property == property);
    let parameter = matches
        .next()
        .ok_or(CalibrationError::MissingParameter(property))?;
    parameter.distribution = distribution;
    if matches.next().is_some() {
        Err(CalibrationError::DuplicateParameter(property))
    } else {
        Ok(())
    }
}

fn unique_definition_distribution(
    definition: &WorldDefinition,
    property: ParameterProperty,
) -> Result<ParameterDistribution, CalibrationError> {
    let mut matches = definition
        .parameters
        .iter()
        .filter(|parameter| parameter.target.property == property);
    let distribution = matches
        .next()
        .ok_or(CalibrationError::MissingParameter(property))?
        .distribution;
    if matches.next().is_some() {
        Err(CalibrationError::DuplicateParameter(property))
    } else {
        Ok(distribution)
    }
}

fn estimates_match(left: &ParameterEstimate, right: &ParameterEstimate) -> bool {
    scalars_match(left.point_estimate, right.point_estimate)
        && scalars_match(left.standard_error, right.standard_error)
        && distributions_match(left.calibrated_distribution, right.calibrated_distribution)
}

fn distributions_match(left: ParameterDistribution, right: ParameterDistribution) -> bool {
    match (left, right) {
        (
            ParameterDistribution::Fixed { value: left },
            ParameterDistribution::Fixed { value: right },
        ) => scalars_match(left, right),
        (
            ParameterDistribution::Uniform {
                minimum: left_minimum,
                maximum: left_maximum,
            },
            ParameterDistribution::Uniform {
                minimum: right_minimum,
                maximum: right_maximum,
            },
        ) => {
            scalars_match(left_minimum, right_minimum) && scalars_match(left_maximum, right_maximum)
        }
        (
            ParameterDistribution::Normal {
                mean: left_mean,
                standard_deviation: left_deviation,
            },
            ParameterDistribution::Normal {
                mean: right_mean,
                standard_deviation: right_deviation,
            },
        ) => scalars_match(left_mean, right_mean) && scalars_match(left_deviation, right_deviation),
        _ => false,
    }
}

fn scalars_match(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 4.0 * f64::EPSILON * scale
}

fn distribution_minimum(distribution: ParameterDistribution) -> f64 {
    match distribution {
        ParameterDistribution::Fixed { value } => value,
        ParameterDistribution::Uniform { minimum, .. } => minimum,
        ParameterDistribution::Normal { .. } => f64::NEG_INFINITY,
    }
}

fn distribution_maximum(distribution: ParameterDistribution) -> f64 {
    match distribution {
        ParameterDistribution::Fixed { value } => value,
        ParameterDistribution::Uniform { maximum, .. } => maximum,
        ParameterDistribution::Normal { .. } => f64::INFINITY,
    }
}
