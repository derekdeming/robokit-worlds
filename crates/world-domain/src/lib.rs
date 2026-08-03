#![doc = "Engine-neutral scientific-world domain types and validation."]

mod entity;
mod error;
mod ids;
mod parameter;
mod path;
mod protocol;
mod units;
mod verifier;
mod world;

pub use entity::{
    AssetFormat, AssetReference, EntityKind, InstrumentKind, MaterialClass, Pose, Position3,
    Quaternion, RobotEmbodiment, Sha256Digest, Vector3, VesselRole, WorldEntity,
};
pub use error::{ValidationCode, ValidationIssue, WorldValidationErrors};
pub use ids::{
    EntityId, MeasurementId, ParameterId, ProtocolStepId, RunId, TaskId, WorldId, WorldVersion,
};
pub use parameter::{
    ParameterDefinition, ParameterDistribution, ParameterProperty, ParameterTarget, ParameterUnit,
};
pub use path::PackagePath;
pub use protocol::{Axis, ProtocolAction, ProtocolStep, TaskDefinition};
pub use units::{Grams, Meters, Radians, Seconds};
pub use verifier::{EventKind, VerifierDefinition};
pub use world::{SchemaVersion, ValidatedWorldDefinition, WorldDefinition};
