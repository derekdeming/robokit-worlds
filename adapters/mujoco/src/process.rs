use std::{
    ffi::OsString,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use wait_timeout::ChildExt;

use crate::{
    MujocoAdapterError,
    contract::{AdapterExecutionRequest, AdapterExecutionResponse},
};

const MAXIMUM_REQUEST_BYTES: u64 = 2 * 1024 * 1024;
const MAXIMUM_RESPONSE_BYTES: u64 = 1024 * 1024;
const MAXIMUM_DIAGNOSTIC_BYTES: u64 = 64 * 1024;
const DEFAULT_STARTUP_GRACE: Duration = Duration::from_secs(20);

/// Bounded command used to execute the isolated Python `MuJoCo` worker.
#[derive(Clone, Debug)]
pub struct MujocoProcessCommand {
    program: PathBuf,
    arguments: Vec<OsString>,
    startup_grace: Duration,
}

impl MujocoProcessCommand {
    /// Creates a command using an explicit program, arguments, and startup grace.
    pub fn new(
        program: impl Into<PathBuf>,
        arguments: Vec<OsString>,
        startup_grace: Duration,
    ) -> Self {
        Self {
            program: program.into(),
            arguments,
            startup_grace,
        }
    }

    /// Creates the pinned `uv` command for the checked-in Python project.
    pub fn uv(project_directory: &Path) -> Self {
        Self::new(
            "uv",
            vec![
                "run".into(),
                "--frozen".into(),
                "--project".into(),
                project_directory.as_os_str().to_owned(),
                "robokit-mujoco".into(),
            ],
            DEFAULT_STARTUP_GRACE,
        )
    }

    pub(crate) fn execute(
        &self,
        request: &AdapterExecutionRequest,
    ) -> Result<AdapterExecutionResponse, MujocoAdapterError> {
        let temporary_directory = tempfile::tempdir().map_err(|source| MujocoAdapterError::Io {
            operation: "create temporary adapter directory",
            path: std::env::temp_dir(),
            source,
        })?;
        let request_path = temporary_directory.path().join("request.json");
        let response_path = temporary_directory.path().join("response.json");
        let stdout_path = temporary_directory.path().join("stdout.log");
        let stderr_path = temporary_directory.path().join("stderr.log");

        let request_bytes =
            serde_json::to_vec(request).map_err(MujocoAdapterError::SerializeRequest)?;
        enforce_payload_bound("request", request_bytes.len(), MAXIMUM_REQUEST_BYTES)?;
        fs::write(&request_path, request_bytes).map_err(|source| MujocoAdapterError::Io {
            operation: "write adapter request",
            path: request_path.clone(),
            source,
        })?;

        let stdout = File::create(&stdout_path).map_err(|source| MujocoAdapterError::Io {
            operation: "create adapter stdout log",
            path: stdout_path.clone(),
            source,
        })?;
        let stderr = File::create(&stderr_path).map_err(|source| MujocoAdapterError::Io {
            operation: "create adapter stderr log",
            path: stderr_path.clone(),
            source,
        })?;
        let mut command = Command::new(&self.program);
        command
            .args(&self.arguments)
            .arg("--request")
            .arg(&request_path)
            .arg("--response")
            .arg(&response_path)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        let mut child = command.spawn().map_err(|source| MujocoAdapterError::Io {
            operation: "spawn MuJoCo subprocess",
            path: self.program.clone(),
            source,
        })?;

        let execution_timeout =
            Duration::try_from_secs_f64(request.limits.wall_clock_timeout.get())
                .map_err(|error| MujocoAdapterError::ContractViolation(error.to_string()))?;
        let process_timeout = execution_timeout
            .checked_add(self.startup_grace)
            .ok_or_else(|| {
                MujocoAdapterError::ContractViolation(
                    "wall-clock timeout plus startup grace exceeds Duration".to_owned(),
                )
            })?;
        let status =
            child
                .wait_timeout(process_timeout)
                .map_err(|source| MujocoAdapterError::Io {
                    operation: "wait for MuJoCo subprocess",
                    path: self.program.clone(),
                    source,
                })?;
        let Some(status) = status else {
            child.kill().map_err(|source| MujocoAdapterError::Io {
                operation: "terminate timed-out MuJoCo subprocess",
                path: self.program.clone(),
                source,
            })?;
            child.wait().map_err(|source| MujocoAdapterError::Io {
                operation: "reap timed-out MuJoCo subprocess",
                path: self.program.clone(),
                source,
            })?;
            return Err(MujocoAdapterError::ProcessTimeout(process_timeout));
        };
        if !status.success() {
            return Err(MujocoAdapterError::ProcessFailure {
                status,
                stderr: read_bounded_text(&stderr_path, MAXIMUM_DIAGNOSTIC_BYTES)?,
            });
        }

        let response_bytes = read_bounded_bytes(&response_path, MAXIMUM_RESPONSE_BYTES)?;
        serde_json::from_slice(&response_bytes).map_err(MujocoAdapterError::DeserializeResponse)
    }
}

fn enforce_payload_bound(
    artifact: &'static str,
    length: usize,
    limit_bytes: u64,
) -> Result<(), MujocoAdapterError> {
    let length = u64::try_from(length).map_err(|_| MujocoAdapterError::PayloadTooLarge {
        artifact,
        limit_bytes,
    })?;
    if length <= limit_bytes {
        Ok(())
    } else {
        Err(MujocoAdapterError::PayloadTooLarge {
            artifact,
            limit_bytes,
        })
    }
}

fn read_bounded_bytes(path: &Path, limit: u64) -> Result<Vec<u8>, MujocoAdapterError> {
    let file = File::open(path).map_err(|source| MujocoAdapterError::Io {
        operation: "open adapter response",
        path: path.to_owned(),
        source,
    })?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| MujocoAdapterError::Io {
            operation: "read adapter response",
            path: path.to_owned(),
            source,
        })?;
    enforce_payload_bound("response", bytes.len(), limit)?;
    Ok(bytes)
}

fn read_bounded_text(path: &Path, limit: u64) -> Result<String, MujocoAdapterError> {
    let mut file = File::open(path).map_err(|source| MujocoAdapterError::Io {
        operation: "open adapter diagnostics",
        path: path.to_owned(),
        source,
    })?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|source| MujocoAdapterError::Io {
            operation: "read adapter diagnostics",
            path: path.to_owned(),
            source,
        })?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
