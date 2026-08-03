use std::{io, path::PathBuf};

use thiserror::Error;

/// Typed failure at the CLI composition boundary.
#[derive(Debug, Error)]
pub(crate) enum CliError {
    #[error("failed to read `{path}`: {source}", path = .path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to write `{path}`: {source}", path = .path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to write stdout: {0}")]
    Stdout(#[from] io::Error),
    #[error("invalid TOML in `{path}`: {source}", path = .path.display())]
    ParseToml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid JSON in `{path}`: {source}", path = .path.display())]
    ParseJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid world definition: {0}")]
    InvalidWorld(String),
    #[error("invalid run manifest: {0}")]
    InvalidManifest(String),
    #[error("invalid calibration dataset: {0}")]
    InvalidCalibrationDataset(String),
    #[error("invalid calibration package: {0}")]
    InvalidCalibrationPackage(String),
    #[error("invalid simulation option: {0}")]
    InvalidSimulationOption(String),
    #[error("run output directory already exists: `{path}`", path = .path.display())]
    OutputExists { path: PathBuf },
    #[error(
        "digest mismatch for `{path}`: expected {expected}, found {actual}",
        path = .path.display()
    )]
    DigestMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    #[error(
        "artifact length mismatch for `{path}`: expected {expected} bytes, found {actual}",
        path = .path.display()
    )]
    ArtifactLengthMismatch {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },
    #[error("artifact `{path}` is too large to represent on this platform", path = .path.display())]
    ArtifactTooLarge { path: PathBuf },
    #[error("input `{path}` exceeds the {limit_bytes}-byte limit", path = .path.display())]
    InputTooLarge { path: PathBuf, limit_bytes: u64 },
    #[error("invalid text input `{path}`: {reason}", path = .path.display())]
    InvalidTextInput { path: PathBuf, reason: String },
    #[error(transparent)]
    Evaluation(#[from] evaluation::EvaluationError),
    #[error(transparent)]
    Calibration(#[from] calibration::CalibrationError),
    #[error(transparent)]
    RuntimeRequest(#[from] runtime_api::RuntimeRequestError),
    #[error(transparent)]
    Mujoco(#[from] mujoco_adapter::MujocoAdapterError),
    #[error(transparent)]
    Serialize(#[from] serde_json::Error),
    #[error("failed to serialize calibrated world TOML: {0}")]
    SerializeToml(#[from] toml::ser::Error),
}
