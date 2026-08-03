#![doc = "Immutable provenance contracts for physical and simulated runs."]

mod digest;
mod manifest;

pub use digest::sha256_digest;
pub use manifest::{
    ArtifactMediaType, ArtifactReference, ArtifactRole, ManifestValidationCode,
    ManifestValidationError, ManifestValidationIssue, RunKind, RunManifest,
    RunManifestSchemaVersion, RuntimeIdentity, ValidatedRunManifest, WorldRevision,
};
