use thiserror::Error;
use world_domain::ParameterProperty;

/// One actionable physical-dataset validation issue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatasetValidationIssue {
    /// Dot-separated contract location.
    pub path: String,
    /// Human-readable invariant failure.
    pub message: String,
}

impl DatasetValidationIssue {
    pub(crate) fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

/// Aggregated physical-dataset validation failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("calibration dataset has {count} validation issue(s)", count = .issues.len())]
pub struct DatasetValidationError {
    issues: Vec<DatasetValidationIssue>,
}

impl DatasetValidationError {
    pub(crate) fn new(issues: Vec<DatasetValidationIssue>) -> Self {
        Self { issues }
    }

    /// Returns every issue in deterministic traversal order.
    pub fn issues(&self) -> &[DatasetValidationIssue] {
        &self.issues
    }
}

/// Failure to estimate or apply a scientifically valid granular model.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum CalibrationError {
    /// New world version is invalid or not newer than its source.
    #[error("calibrated world version must be valid semver and newer than the source version")]
    InvalidWorldVersion,
    /// Required source-world parameter is absent.
    #[error("source world is missing required parameter `{0:?}`")]
    MissingParameter(ParameterProperty),
    /// Required property has ambiguous ownership.
    #[error("source world contains multiple parameters for `{0:?}`")]
    DuplicateParameter(ParameterProperty),
    /// Physical control values do not isolate the modeled pour regime.
    #[error("observation {index} violates the calibrated pour regime: {reason}")]
    InvalidObservationRegime {
        /// Zero-based observation index.
        index: usize,
        /// Concrete scientific invariant.
        reason: String,
    },
    /// Physical observations do not identify both flow rate and delay.
    #[error("physical observations do not provide sufficient hold-duration variation")]
    InsufficientVariation,
    /// Observation count cannot be represented by the bounded estimator.
    #[error("physical observation count exceeds the calibrated estimator bound")]
    TooManyObservations,
    /// Fitted flow or delay is outside the supported physical model.
    #[error("calibration produced an invalid {property:?} estimate: {value}")]
    InvalidEstimate {
        /// Parameter whose estimate is invalid.
        property: ParameterProperty,
        /// Invalid point or bound.
        value: f64,
    },
    /// Updated world failed shared domain validation.
    #[error("calibrated world is invalid: {0}")]
    InvalidCalibratedWorld(String),
}
