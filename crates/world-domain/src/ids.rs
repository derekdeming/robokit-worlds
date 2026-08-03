use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! string_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Deserialize, Hash, JsonSchema, PartialEq, Eq, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Creates a new `", stringify!($name), "` without validating its enclosing world.")]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[doc = concat!("Returns the string representation of this `", stringify!($name), "`.")]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub(crate) fn is_valid(&self) -> bool {
                !self.0.is_empty()
                    && self.0.len() <= 128
                    && self
                        .0
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

string_id!(
    EntityId,
    "Stable identifier for an entity within one world."
);
string_id!(
    MeasurementId,
    "Stable identifier for a produced measurement."
);
string_id!(
    ParameterId,
    "Stable identifier for an uncertain world parameter."
);
string_id!(ProtocolStepId, "Stable identifier for a protocol step.");
string_id!(TaskId, "Stable identifier for a task within one world.");

/// Globally unique identity of a scientific world across versions.
#[derive(Clone, Copy, Debug, Deserialize, Hash, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct WorldId(Uuid);

impl WorldId {
    /// Creates a random world identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wraps an existing UUID.
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the underlying UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for WorldId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WorldId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Globally unique identity of one physical or simulated execution.
#[derive(Clone, Copy, Debug, Deserialize, Hash, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RunId(Uuid);

impl RunId {
    /// Creates a random run identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wraps an existing UUID.
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the underlying UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Semantic version of a world definition.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct WorldVersion(String);

impl WorldVersion {
    /// Creates a world version without validating its enclosing world.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the semantic-version string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns whether the value follows semantic versioning.
    pub fn is_valid_semver(&self) -> bool {
        semver::Version::parse(&self.0).is_ok()
    }
}

impl fmt::Display for WorldVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}
