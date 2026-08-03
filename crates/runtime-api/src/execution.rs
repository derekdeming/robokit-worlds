use std::error::Error;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use world_domain::{EventKind, Grams, MeasurementId, RunId, Seconds, ValidatedWorldDefinition};

use crate::RuntimeCapabilities;

/// Explicit bounds for one runtime execution.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct ExecutionLimits {
    /// Maximum permitted runtime steps.
    pub maximum_steps: u64,
    /// Maximum wall-clock duration.
    pub wall_clock_timeout: Seconds,
}

impl ExecutionLimits {
    /// Validates that both bounds are positive and finite.
    ///
    /// # Errors
    ///
    /// Returns a typed request error when a bound is invalid.
    pub fn validate(self) -> Result<Self, RuntimeRequestError> {
        if self.maximum_steps == 0 {
            return Err(RuntimeRequestError::ZeroMaximumSteps);
        }
        if !self.wall_clock_timeout.get().is_finite() || self.wall_clock_timeout.get() <= 0.0 {
            return Err(RuntimeRequestError::InvalidTimeout);
        }
        Ok(self)
    }
}

/// Reproducible request for one bounded world execution.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct ExecutionRequest {
    /// Globally unique run identity and idempotency key across deliveries.
    pub run_id: RunId,
    /// One-based delivery attempt for observability and lease ownership.
    pub attempt: u32,
    /// Runtime random seed.
    pub seed: u64,
    /// Mandatory execution bounds.
    pub limits: ExecutionLimits,
}

impl ExecutionRequest {
    /// Validates request bounds.
    ///
    /// # Errors
    ///
    /// Returns a typed request error when a bound is invalid.
    pub fn validate(self) -> Result<ValidatedExecutionRequest, RuntimeRequestError> {
        if self.attempt == 0 {
            return Err(RuntimeRequestError::ZeroAttempt);
        }
        self.limits.validate()?;
        Ok(ValidatedExecutionRequest(self))
    }
}

/// Execution request whose delivery metadata and resource bounds have been checked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValidatedExecutionRequest(ExecutionRequest);

impl ValidatedExecutionRequest {
    /// Borrows the validated request.
    pub const fn request(&self) -> &ExecutionRequest {
        &self.0
    }
}

/// Error in caller-supplied execution bounds.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum RuntimeRequestError {
    /// Attempt numbers are one-based.
    #[error("attempt must be greater than zero")]
    ZeroAttempt,
    /// No runtime steps were permitted.
    #[error("maximum_steps must be greater than zero")]
    ZeroMaximumSteps,
    /// Timeout was zero, negative, infinite, or not a number.
    #[error("wall_clock_timeout must be positive and finite")]
    InvalidTimeout,
}

/// Typed observation emitted during execution.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "observation", rename_all = "snake_case")]
pub enum RuntimeEvent {
    /// Gravimetric instrument reading.
    MassMeasurement {
        /// Measurement identity declared by the protocol.
        measurement_id: MeasurementId,
        /// Observed mass.
        value: Grams,
    },
    /// Scientific, operational, or safety event.
    Event {
        /// Stable event category.
        event: EventKind,
    },
}

/// Receives observations without coupling a runtime to evidence storage.
pub trait RuntimeObserver {
    /// Observer-specific failure.
    type Error: Error + Send + Sync + 'static;

    /// Records one ordered runtime event.
    ///
    /// # Errors
    ///
    /// Returns an observer error when the event cannot be durably accepted.
    fn observe(&mut self, event: RuntimeEvent) -> Result<(), Self::Error>;
}

/// Completion metadata independent of scientific outcome evaluation.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct ExecutionSummary {
    /// Number of executed runtime steps.
    pub executed_steps: u64,
    /// Observed wall-clock duration.
    pub wall_clock_duration: Seconds,
}

/// Capability-checked implementation of a simulator or physical cell.
pub trait RuntimeAdapter {
    /// Adapter-specific execution failure.
    type Error: Error + Send + Sync + 'static;

    /// Declares behavior supported by this exact adapter implementation.
    fn capabilities(&self) -> &RuntimeCapabilities;

    /// Executes one validated world under explicit bounds.
    ///
    /// # Errors
    ///
    /// Returns an adapter-specific error when preparation, execution, observation, or shutdown fails.
    fn execute<O: RuntimeObserver>(
        &mut self,
        world: &ValidatedWorldDefinition,
        request: &ValidatedExecutionRequest,
        observer: &mut O,
    ) -> Result<ExecutionSummary, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn execution_request_requires_both_bounds() {
        let request = ExecutionRequest {
            run_id: RunId::from_uuid(Uuid::nil()),
            attempt: 1,
            seed: 1,
            limits: ExecutionLimits {
                maximum_steps: 0,
                wall_clock_timeout: Seconds::new(1.0),
            },
        };
        assert_eq!(
            request.validate(),
            Err(RuntimeRequestError::ZeroMaximumSteps)
        );
    }
}
