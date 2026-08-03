use std::{error::Error, io, path::PathBuf, process::ExitStatus, time::Duration};

use runtime_api::{CapabilityMismatch, RuntimeRequestError};
use thiserror::Error;
use world_domain::ParameterProperty;

/// Typed `MuJoCo` adapter failure.
#[derive(Debug, Error)]
pub enum MujocoAdapterError {
    /// Binding TOML could not be parsed.
    #[error("invalid MuJoCo binding TOML: {0}")]
    BindingToml(toml::de::Error),
    /// Binding is syntactically valid but inconsistent.
    #[error("invalid MuJoCo binding: {0}")]
    InvalidBinding(String),
    /// Runtime does not implement a required world behavior.
    #[error(transparent)]
    Capability(#[from] CapabilityMismatch),
    /// Execution request is not bounded.
    #[error(transparent)]
    InvalidRequest(#[from] RuntimeRequestError),
    /// Required process parameter is absent.
    #[error("world is missing required process parameter `{0:?}`")]
    MissingParameter(ParameterProperty),
    /// Process parameter appears more than once.
    #[error("world contains duplicate process parameter `{0:?}`")]
    DuplicateParameter(ParameterProperty),
    /// Parameter distribution violates process-model invariants.
    #[error("invalid process parameter `{property:?}`: {reason}")]
    InvalidParameter {
        /// Invalid property.
        property: ParameterProperty,
        /// Concrete invariant failure.
        reason: String,
    },
    /// Protocol actor is not the entity bound to the runtime body.
    #[error("protocol step `{step_id}` controls `{actual}`; binding requires `{expected}`")]
    WrongControlledEntity {
        /// Stable protocol step identifier.
        step_id: String,
        /// Bound canonical entity.
        expected: String,
        /// Referenced canonical entity.
        actual: String,
    },
    /// Required MJCF scene is not declared by the world.
    #[error("binding scene asset `{0}` is not a declared MJCF world asset")]
    MissingSceneAsset(String),
    /// File operation failed.
    #[error("failed to {operation} `{path}`: {source}", path = .path.display())]
    Io {
        /// Operation being attempted.
        operation: &'static str,
        /// Affected path.
        path: PathBuf,
        /// Underlying operating-system failure.
        #[source]
        source: io::Error,
    },
    /// Adapter request serialization failed.
    #[error("failed to serialize adapter request: {0}")]
    SerializeRequest(serde_json::Error),
    /// Adapter response deserialization failed.
    #[error("failed to deserialize adapter response: {0}")]
    DeserializeResponse(serde_json::Error),
    /// Request or response exceeded its explicit byte bound.
    #[error("{artifact} exceeded the {limit_bytes}-byte adapter limit")]
    PayloadTooLarge {
        /// Request or response label.
        artifact: &'static str,
        /// Enforced byte bound.
        limit_bytes: u64,
    },
    /// Subprocess exceeded its wall-clock deadline plus startup grace.
    #[error("MuJoCo subprocess exceeded its {0:?} deadline")]
    ProcessTimeout(Duration),
    /// Subprocess returned a non-success status.
    #[error("MuJoCo subprocess exited with {status}: {stderr}")]
    ProcessFailure {
        /// Process exit status.
        status: ExitStatus,
        /// Bounded diagnostic output.
        stderr: String,
    },
    /// Response violates the declared runtime identity or execution limits.
    #[error("MuJoCo subprocess violated its contract: {0}")]
    ContractViolation(String),
    /// Observer could not durably accept an event.
    #[error("runtime observer rejected an event: {0}")]
    Observer(#[source] Box<dyn Error + Send + Sync>),
}
