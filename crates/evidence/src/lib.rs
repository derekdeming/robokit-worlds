#![doc = "Immutable provenance contracts for physical and simulated runs."]

mod digest;
mod manifest;

pub use digest::{sha256_digest, sha256_digest_reader};
pub use manifest::{
    ArtifactMediaType, ArtifactReference, ArtifactRole, ManifestValidationCode,
    ManifestValidationError, ManifestValidationIssue, RunKind, RunManifest,
    RunManifestSchemaVersion, RuntimeIdentity, ValidatedRunManifest, WorldRevision,
};
