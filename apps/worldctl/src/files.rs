use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use evidence::RunManifest;
use world_domain::WorldDefinition;

use crate::error::CliError;

/// Temporary sibling directory published atomically only after all writes succeed.
pub(crate) struct StagedDirectory {
    output_path: PathBuf,
    staging: tempfile::TempDir,
}

impl StagedDirectory {
    pub(crate) fn create(output_path: PathBuf) -> Result<Self, CliError> {
        if output_path.exists() {
            return Err(CliError::OutputExists { path: output_path });
        }
        let output_parent = output_path.parent().unwrap_or_else(|| Path::new("."));
        create_directory(output_parent)?;
        let staging = tempfile::Builder::new()
            .prefix(".robokit-stage-")
            .tempdir_in(output_parent)
            .map_err(|source| CliError::Write {
                path: output_parent.to_path_buf(),
                source,
            })?;
        Ok(Self {
            output_path,
            staging,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        self.staging.path()
    }

    pub(crate) fn publish(self) -> Result<(), CliError> {
        rename_directory(self.staging.path(), &self.output_path)
    }
}

pub(crate) fn load_world(path: &Path) -> Result<world_domain::ValidatedWorldDefinition, CliError> {
    let text = read_text(path)?;
    let world = parse_world(path, &text)?;
    verify_world_assets(path, &world)?;
    Ok(world)
}

pub(crate) fn parse_world(
    path: &Path,
    text: &str,
) -> Result<world_domain::ValidatedWorldDefinition, CliError> {
    let world: WorldDefinition = toml::from_str(text).map_err(|source| CliError::ParseToml {
        path: path.to_path_buf(),
        source,
    })?;
    let world = world
        .validate()
        .map_err(|error| CliError::InvalidWorld(format_world_issues(&error)))?;
    Ok(world)
}

pub(crate) fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, CliError> {
    let bytes = read_bytes(path)?;
    parse_json(path, &bytes)
}

pub(crate) fn parse_json<T: serde::de::DeserializeOwned>(
    path: &Path,
    bytes: &[u8],
) -> Result<T, CliError> {
    serde_json::from_slice(bytes).map_err(|source| CliError::ParseJson {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn read_text(path: &Path) -> Result<String, CliError> {
    fs::read_to_string(path).map_err(|source| CliError::Read {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn read_bytes(path: &Path) -> Result<Vec<u8>, CliError> {
    fs::read(path).map_err(|source| CliError::Read {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn read_bounded_bytes(path: &Path, limit_bytes: u64) -> Result<Vec<u8>, CliError> {
    let file = fs::File::open(path).map_err(|source| CliError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let mut bytes = Vec::new();
    file.take(limit_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| CliError::Read {
            path: path.to_path_buf(),
            source,
        })?;
    let length = u64::try_from(bytes.len()).map_err(|_| CliError::InputTooLarge {
        path: path.to_path_buf(),
        limit_bytes,
    })?;
    if length > limit_bytes {
        Err(CliError::InputTooLarge {
            path: path.to_path_buf(),
            limit_bytes,
        })
    } else {
        Ok(bytes)
    }
}

pub(crate) fn verify_manifest_artifacts(
    manifest_path: &Path,
    manifest: &RunManifest,
) -> Result<(), CliError> {
    let bundle_root = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    for artifact in &manifest.artifacts {
        let path = bundle_root.join(artifact.relative_path.as_str());
        let actual_length = fs::metadata(&path)
            .map_err(|source| CliError::Read {
                path: path.clone(),
                source,
            })?
            .len();
        if actual_length != artifact.byte_length {
            return Err(CliError::ArtifactLengthMismatch {
                path,
                expected: artifact.byte_length,
                actual: actual_length,
            });
        }
        verify_digest_file(&path, &artifact.sha256)?;
    }
    Ok(())
}

pub(crate) fn verify_digest_file(
    path: &Path,
    expected: &world_domain::Sha256Digest,
) -> Result<(), CliError> {
    let file = fs::File::open(path).map_err(|source| CliError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let actual = evidence::sha256_digest_reader(file).map_err(|source| CliError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    if actual == *expected {
        Ok(())
    } else {
        Err(CliError::DigestMismatch {
            path: path.to_path_buf(),
            expected: expected.as_str().to_owned(),
            actual: actual.as_str().to_owned(),
        })
    }
}

pub(crate) fn verify_digest(
    path: &Path,
    bytes: &[u8],
    expected: &world_domain::Sha256Digest,
) -> Result<(), CliError> {
    let actual = evidence::sha256_digest(bytes);
    if actual == *expected {
        Ok(())
    } else {
        Err(CliError::DigestMismatch {
            path: path.to_path_buf(),
            expected: expected.as_str().to_owned(),
            actual: actual.as_str().to_owned(),
        })
    }
}

pub(crate) fn write_bytes(path: Option<&Path>, bytes: &[u8]) -> Result<(), CliError> {
    if let Some(path) = path {
        write_file(path, bytes)
    } else {
        let mut stdout = io::stdout().lock();
        stdout.write_all(bytes)?;
        stdout.write_all(b"\n")?;
        Ok(())
    }
}

pub(crate) fn write_file(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    fs::write(path, bytes).map_err(|source| CliError::Write {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn copy_file(source_path: &Path, destination_path: &Path) -> Result<(), CliError> {
    fs::copy(source_path, destination_path).map_err(|source| CliError::Write {
        path: destination_path.to_path_buf(),
        source,
    })?;
    Ok(())
}

pub(crate) fn pretty_json<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, CliError> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(crate) fn create_directory(path: &Path) -> Result<(), CliError> {
    fs::create_dir_all(path).map_err(|source| CliError::Write {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn rename_directory(source_path: &Path, target_path: &Path) -> Result<(), CliError> {
    fs::rename(source_path, target_path).map_err(|source| CliError::Write {
        path: target_path.to_path_buf(),
        source,
    })
}

pub(crate) fn canonicalize(path: &Path) -> Result<PathBuf, CliError> {
    fs::canonicalize(path).map_err(|source| CliError::Read {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn verify_world_assets(
    world_path: &Path,
    world: &world_domain::ValidatedWorldDefinition,
) -> Result<(), CliError> {
    let package_root = world_path.parent().unwrap_or_else(|| Path::new("."));
    for entity in &world.definition().entities {
        if let Some(asset) = &entity.asset {
            let path = package_root.join(asset.logical_path.as_str());
            verify_digest_file(&path, &asset.sha256)?;
        }
    }
    Ok(())
}

fn format_world_issues(error: &world_domain::WorldValidationErrors) -> String {
    error
        .issues()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}
